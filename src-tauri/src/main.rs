// ============================================================
// FICHIER : trimlab/src-tauri/src/main.rs
// ROLE    : Point d'entree Tauri - enregistre toutes les commandes
// ============================================================

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod db;
mod commands;
mod sync_audio;

use commands::projects::*;
use commands::segments::*;
use commands::licence::*;

fn ffprobe_bin() -> String {
    if let Ok(exe) = std::env::current_exe() {
        let p = exe.parent()
            .unwrap_or(&std::path::PathBuf::from("."))
            .join("ffprobe.exe");
        if p.exists() { return p.to_string_lossy().to_string(); }
    }
    "ffprobe".to_string()
}

fn main() {
    if let Err(e) = db::init() {
        eprintln!("[TrimLab] Erreur init SQLite: {}", e);
    }

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_notification::init())
        .invoke_handler(tauri::generate_handler![
            // ── Projets ──────────────────────────────
            list_projects,
            get_project,
            create_project,
            update_project_status,
            update_project_settings,
            delete_project,
            add_media_file,
            save_processing_stats,
            get_dashboard_stats,
            // ── Segments ─────────────────────────────
            list_segments,
            save_segments,
            toggle_segment,
            delete_segment,
            analyse_and_save,
            analyse_batch,
            convert_source_to_mp4,
            generate_chapters,
            save_text_file,
            update_media_file_path,
            // ── Licence ──────────────────────────────
            get_licence,
            validate_and_activate_licence,
            deactivate_licence,
            get_machine_id,
            commands::licence::verify_licence_online,
            // ── Media (FFmpeg) ────────────────────────
            probe_media,
            get_waveform_data,
            open_file_dialog,
            export_segments,
            get_ffmpeg_version,
            // ── Sous-titres ───────────────────────────
            commands::subtitle::export_srt,
            commands::subtitle::export_ass,
            commands::subtitle::burn_subtitles,
            // ── Export FFmpeg direct ──────────────────
            commands::export_ffmpeg::export_ffmpeg,
            commands::export_audio::export_audio,
            commands::export_multi_xml::export_multi_xml,
            // ── Whisper / Transcription ───────────────
            commands::whisper::check_whisper_available,
            commands::whisper::ensure_whisper_model,
            commands::whisper::get_local_model_path,
            commands::whisper::transcribe_and_detect,
            commands::whisper::get_transcript_words,
            commands::whisper::delete_project_words,
            commands::whisper::transcribe_batch,
            commands::whisper::save_subtitle_blocks,
            commands::whisper::get_subtitle_blocks,
            // ── Synchronisation audio externe ─────────
            sync_audio::sync_external_audio,
            sync_audio::get_audio_sync_info,
            sync_audio::remove_external_audio,
        ])
        .setup(|_app| {
            // Vérification licence en ligne au démarrage (max 1x/semaine)
            std::thread::spawn(|| {
                let _ = commands::licence::verify_licence_online();
            });
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("Erreur au demarrage de TrimLab");
}

// ─── Commandes media (FFmpeg) - restent dans main.rs ─────────────────────────

use std::path::PathBuf;
use serde::{Deserialize, Serialize};
use tauri::Emitter;

#[derive(Debug, Serialize, Deserialize)]
pub struct MediaInfo {
    pub duration: f64,
    pub size: i64,
    pub has_video: bool,
    pub has_audio: bool,
    pub fps: Option<f64>,
    pub codec: Option<String>,
    pub sample_rate: Option<i64>,
    pub width: Option<i64>,
    pub height: Option<i64>,
    pub channels: Option<i64>,
}

#[tauri::command]
fn probe_media(path: String) -> Result<MediaInfo, String> {
    let output = std::process::Command::new(ffprobe_bin())
        .args(["-v", "quiet", "-print_format", "json", "-show_format", "-show_streams", &path])
        .output()
        .map_err(|e| format!("ffprobe introuvable: {}", e))?;

    let json: serde_json::Value = serde_json::from_slice(&output.stdout)
        .map_err(|e| format!("Parse JSON: {}", e))?;

    let duration = json["format"]["duration"].as_str().and_then(|s| s.parse().ok()).unwrap_or(0.0);
    let size = json["format"]["size"].as_str().and_then(|s| s.parse().ok()).unwrap_or(0);

    let streams = json["streams"].as_array();
    let mut has_video = false;
    let mut has_audio = false;
    let mut fps: Option<f64> = None;
    let mut codec: Option<String> = None;
    let mut sample_rate: Option<i64> = None;
    let mut width: Option<i64> = None;
    let mut height: Option<i64> = None;
    let mut channels: Option<i64> = None;

    if let Some(streams) = streams {
        for s in streams {
            match s["codec_type"].as_str() {
                Some("video") => {
                    has_video = true;
                    codec = s["codec_name"].as_str().map(String::from);
                    width  = s["width"].as_i64();
                    height = s["height"].as_i64();
                    if let Some(r) = s["r_frame_rate"].as_str() {
                        let parts: Vec<f64> = r.split('/').filter_map(|p| p.parse().ok()).collect();
                        if parts.len() == 2 && parts[1] != 0.0 { fps = Some(parts[0] / parts[1]); }
                    }
                }
                Some("audio") => {
                    has_audio = true;
                    sample_rate = s["sample_rate"].as_str().and_then(|s| s.parse().ok());
                    channels = s["channels"].as_i64();
                    if codec.is_none() { codec = s["codec_name"].as_str().map(String::from); }
                }
                _ => {}
            }
        }
    }

    Ok(MediaInfo { duration, size, has_video, has_audio, fps, codec, sample_rate, width, height, channels })
}

#[tauri::command]
fn get_waveform_data(path: String, points: Option<usize>) -> Result<Vec<f32>, String> {
    let n = points.unwrap_or(2048);
    let output = std::process::Command::new("ffmpeg")
        .args(["-i", &path, "-ar", "8000", "-ac", "1", "-f", "f32le", "-"])
        .output()
        .map_err(|e| format!("ffmpeg: {}", e))?;

    let bytes = output.stdout;
    let samples: Vec<f32> = bytes.chunks_exact(4)
        .map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]]))
        .collect();

    if samples.is_empty() { return Ok(vec![0.0; n]); }

    let bucket = (samples.len() / n).max(1);
    let result: Vec<f32> = (0..n).map(|i| {
        let start = i * bucket;
        let end   = ((i + 1) * bucket).min(samples.len());
        let rms   = samples[start..end].iter().map(|s| s * s).sum::<f32>() / (end - start) as f32;
        let v     = rms.sqrt();
        if i % 2 == 0 { v } else { -v }
    }).collect();

    Ok(result)
}

#[tauri::command]
fn open_file_dialog() -> Result<Option<String>, String> {
    Ok(None)
}

#[derive(Debug, Serialize)]
pub struct ExportResult {
    pub success: bool,
    pub output_path: String,
    pub segments_exported: usize,
}

#[tauri::command]
fn export_segments(
    project_id: String,
    format: String,
    output_dir: String,
) -> Result<ExportResult, String> {
    use crate::commands::segments::list_segments;
    use crate::db;

    let conn = db::open().map_err(|e| e.to_string())?;
    let source_path: String = conn.query_row(
        "SELECT path FROM media_files WHERE project_id=?1 ORDER BY added_at ASC LIMIT 1",
        rusqlite::params![project_id],
        |row| row.get(0),
    ).unwrap_or_default();

    let project_name: String = conn.query_row(
        "SELECT name FROM projects WHERE id=?1",
        rusqlite::params![project_id],
        |row| row.get(0),
    ).unwrap_or_else(|_| "TrimLab Export".to_string());

    let segs = list_segments(project_id.clone())?;
    let keep: Vec<_> = segs.iter().filter(|s| s.seg_type == "keep").collect();

    let fps: u64 = conn.query_row(
        "SELECT COALESCE(fps, 25) FROM media_files WHERE project_id=?1 ORDER BY added_at ASC LIMIT 1",
        rusqlite::params![project_id],
        |row| row.get::<_, f64>(0),
    ).unwrap_or(25.0).round() as u64;

    let media_info = probe_media(source_path.clone()).unwrap_or(MediaInfo {
        duration: 0.0, size: 0, has_video: false, has_audio: false,
        fps: None, codec: None, sample_rate: None, width: None, height: None, channels: None
    });
    let is_stereo = media_info.channels.unwrap_or(2) == 2;

    // ── Audio externe synchronisé ────────────────────────────────
    // Si le projet a un audio externe, on l'intègre dans le XML export
    let ext_audio: Option<(String, f64)> = conn.query_row(
        "SELECT external_audio_path, audio_offset FROM media_files WHERE project_id=?1 ORDER BY added_at ASC LIMIT 1",
        rusqlite::params![project_id],
        |row| {
            let path: Option<String> = row.get(0)?;
            let offset: Option<f64> = row.get(1)?;
            Ok(path.zip(offset))
        },
    ).unwrap_or(None);

    let output_path = std::path::Path::new(&output_dir);
    if let Some(parent) = output_path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }

    let ext = output_path.extension().and_then(|e| e.to_str()).unwrap_or("xml");
    let fmt = if format.is_empty() { ext } else { format.as_str() };

    let content = match fmt {
        "fcpxml" => build_fcpxml(&keep, &source_path, &project_name, fps),
        "edl"    => build_edl(&keep, &project_name, fps),
        _        => build_xml(&keep, &source_path, &project_name, fps, is_stereo, ext_audio.as_ref()),
    };

    std::fs::write(&output_dir, &content).map_err(|e| e.to_string())?;
    Ok(ExportResult { success: true, output_path: output_dir, segments_exported: keep.len() })
}


// ─── Analyse batch parallèle ──────────────────────────────────────────────────
//
// Lance N analyses en parallèle via std::thread::spawn.
// Émet des events Tauri "batch-analyse-progress" pour chaque mise à jour.
//
// Payload : { project_id, step: "analysing"|"done"|"error", progress: 0-100, message }

#[derive(Debug, Clone, serde::Serialize)]
struct BatchAnalyseProgress {
    project_id: String,
    step: String,        // "analysing" | "done" | "error"
    progress: u8,
    message: String,
}

#[derive(Debug, serde::Deserialize)]
struct BatchAnalyseJob {
    project_id: String,
    file_path: String,
    threshold_db: f64,
    min_duration_ms: f64,
    duration: f64,
    padding_before: f64,
    padding_after: f64,
    min_speech_ms: f64,
    aggressiveness: u8,
}

#[tauri::command]
fn analyse_batch(
    window: tauri::Window,
    jobs: Vec<BatchAnalyseJob>,
) -> Result<usize, String> {
    use std::sync::{Arc, Mutex};
    use std::thread;

    let done_count = Arc::new(Mutex::new(0usize));
    let total = jobs.len();
    let mut handles = vec![];

    for job in jobs {
        let win = window.clone();
        let done = Arc::clone(&done_count);

        let handle = thread::spawn(move || {
            let pid = job.project_id.clone();

            // ── Signal : démarrage ─────────────────────
            let _ = win.emit("batch-analyse-progress", BatchAnalyseProgress {
                project_id: pid.clone(),
                step: "analysing".into(),
                progress: 10,
                message: "Analyse audio…".into(),
            });

            // ── Appel réel à analyse_and_save ──────────
            // analyse_and_save est en scope via `use commands::segments::*`
            // C'est une fn Rust normale appelable directement hors contexte Tauri
            let result = analyse_and_save(
                pid.clone(),
                job.file_path,
                job.threshold_db,
                job.min_duration_ms,
                job.duration,
                Some(job.padding_before),
                Some(job.padding_after),
                Some(job.min_speech_ms),
                Some(job.aggressiveness),
            );

            match result {
                Ok(segs) => {
                    // Mettre à jour le statut en DB
                    let _ = crate::db::open().map(|conn| {
                        conn.execute(
                            "UPDATE projects SET status=?1, progress=100 WHERE id=?2",
                            rusqlite::params!["ready", pid],
                        )
                    });

                    let mut d = done.lock().unwrap();
                    *d += 1;

                    let _ = win.emit("batch-analyse-progress", BatchAnalyseProgress {
                        project_id: pid,
                        step: "done".into(),
                        progress: 100,
                        message: format!("{} segments", segs.len()),
                    });
                }
                Err(e) => {
                    let _ = win.emit("batch-analyse-progress", BatchAnalyseProgress {
                        project_id: pid,
                        step: "error".into(),
                        progress: 0,
                        message: e,
                    });
                }
            }
        });

        handles.push(handle);
    }

    // Attendre tous les threads (non-bloquant du côté Tauri grâce au thread séparé)
    thread::spawn(move || {
        for h in handles { let _ = h.join(); }
    });

    Ok(total)
}

#[tauri::command]
fn get_ffmpeg_version() -> Result<String, String> {
    let out = std::process::Command::new("ffmpeg")
        .arg("-version")
        .output()
        .map_err(|e| format!("ffmpeg introuvable: {}", e))?;
    let s = String::from_utf8_lossy(&out.stdout);
    Ok(s.lines().next().unwrap_or("inconnu").to_string())
}

fn to_tc(secs: f64, fps: u64) -> String {
    let total = (secs * fps as f64).round() as u64;
    let frames = total % fps;
    let s      = (total / fps) % 60;
    let m      = (total / fps / 60) % 60;
    let h      = total / fps / 3600;
    format!("{:02}:{:02}:{:02}:{:02}", h, m, s, frames)
}

fn to_fcp_time(secs: f64, fps: u64) -> String {
    let frames = (secs * fps as f64).round() as u64;
    format!("{}/{}s", frames, fps)
}

fn build_fcpxml(segs: &[&crate::commands::segments::Segment], source_path: &str, title: &str, fps: u64) -> String {
    let total_dur: f64 = segs.iter().map(|s| s.end_time - s.start_time).sum();
    let clean_path = source_path.replace('\\', "/");
    let file_uri = format!("file:///{}" , clean_path.trim_start_matches('/'));

    let clips: String = segs.iter().enumerate().map(|(i, s)| {
        let dur    = s.end_time - s.start_time;
        let offset: f64 = segs[..i].iter().map(|x| x.end_time - x.start_time).sum();
        format!(
            r#"        <asset-clip ref="r1" name="{}" offset="{}" duration="{}" start="{}" tcFormat="NDF">
            <audio lane="1"/>
            </asset-clip>"#,
            title,
            to_fcp_time(offset, fps),
            to_fcp_time(dur, fps),
            to_fcp_time(s.start_time, fps)
        )
    }).collect::<Vec<_>>().join("\n");

    format!(r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE fcpxml>
<fcpxml version="1.10">
  <resources>
    <format id="r0" name="FFVideoFormat1080p{fps}" frameDuration="1/{fps}s" width="1920" height="1080"/>
    <asset id="r1" name="{title}" start="0s" duration="{total_dur}" hasVideo="1" hasAudio="1" videoSources="1" audioSources="1" audioChannels="2">
      <media-rep kind="original-media" src="{file_uri}"/>
    </asset>
  </resources>
  <library>
    <event name="{title}">
      <project name="{title}">
        <sequence format="r0" duration="{total_dur}" tcStart="0s" tcFormat="NDF" audioLayout="stereo" audioRate="48k">
          <spine>
{clips}
          </spine>
        </sequence>
      </project>
    </event>
  </library>
</fcpxml>"#,
        fps = fps,
        title = title,
        total_dur = to_fcp_time(total_dur, fps),
        file_uri = file_uri,
        clips = clips,
    )
}

fn build_edl(segs: &[&crate::commands::segments::Segment], title: &str, fps: u64) -> String {
    let mut lines = vec![
        format!("TITLE: {}", title),
        "FCM: NON-DROP FRAME".to_string(),
        String::new(),
    ];
    let mut record_tc = 0.0_f64;
    for (i, s) in segs.iter().enumerate() {
        let dur = s.end_time - s.start_time;
        lines.push(format!(
            "{:03}  AX       AA/V  C        {} {} {} {}",
            i + 1,
            to_tc(s.start_time, fps), to_tc(s.end_time, fps),
            to_tc(record_tc, fps), to_tc(record_tc + dur, fps)
        ));
        record_tc += dur;
    }
    lines.join("\n")
}

// FCP7 XML — Compatible Premiere Pro
//
// Sans audio externe : vidéo + caméra sur A1/A2
// Avec audio externe : audio externe sur A1/A2 (actif), caméra sur A3/A4 (muté)
//   L'audio externe est décalé de ext_offset pour compenser le décalage de sync.
fn build_xml(
    segs: &[&crate::commands::segments::Segment],
    source_path: &str,
    title: &str,
    fps: u64,
    is_stereo: bool,
    ext_audio: Option<&(String, f64)>,
) -> String {
    let ticks_per_frame: u64 = 254016000000 / fps;
    let total_dur: f64 = segs.iter().map(|s| s.end_time - s.start_time).sum();
    let total_frames = (total_dur * fps as f64).round() as u64;
    let has_ext = ext_audio.is_some();
    let ext_offset = ext_audio.map(|(_, o)| *o).unwrap_or(0.0);

    // ── Encode chemin en URI  file://localhost/D%3a/... ────────────
    let encode_uri = |p: &str| -> String {
        p.replace('\\', "/")
         .chars().map(|ch| match ch {
             ':' => "%3a".to_string(),
             ' ' => "%20".to_string(),
             _   => ch.to_string(),
         }).collect()
    };

    let file_uri  = format!("file://localhost/{}", encode_uri(source_path.trim_start_matches('/')));
    let file_name = std::path::Path::new(source_path)
        .file_name().and_then(|n| n.to_str()).unwrap_or("source");
    let src_end_frames = (segs.last().map(|s| s.end_time).unwrap_or(total_dur) * fps as f64).round() as u64;

    // ── Bloc <file> caméra (file-1) ────────────────────────────────
    let cam_audio_media = if is_stereo {
        "\t\t\t\t\t\t\t\t<audio>\n\
         \t\t\t\t\t\t\t\t\t<samplecharacteristics><depth>16</depth><samplerate>48000</samplerate></samplecharacteristics>\n\
         \t\t\t\t\t\t\t\t\t<channelcount>1</channelcount><layout>stereo</layout>\n\
         \t\t\t\t\t\t\t\t\t<audiochannel><sourcechannel>1</sourcechannel><channellabel>left</channellabel></audiochannel>\n\
         \t\t\t\t\t\t\t\t</audio>\n\
         \t\t\t\t\t\t\t\t<audio>\n\
         \t\t\t\t\t\t\t\t\t<samplecharacteristics><depth>16</depth><samplerate>48000</samplerate></samplecharacteristics>\n\
         \t\t\t\t\t\t\t\t\t<channelcount>1</channelcount><layout>stereo</layout>\n\
         \t\t\t\t\t\t\t\t\t<audiochannel><sourcechannel>2</sourcechannel><channellabel>right</channellabel></audiochannel>\n\
         \t\t\t\t\t\t\t\t</audio>"
    } else {
        "\t\t\t\t\t\t\t\t<audio>\n\
         \t\t\t\t\t\t\t\t\t<samplecharacteristics><depth>16</depth><samplerate>48000</samplerate></samplecharacteristics>\n\
         \t\t\t\t\t\t\t\t\t<channelcount>1</channelcount><layout>mono</layout>\n\
         \t\t\t\t\t\t\t\t\t<audiochannel><sourcechannel>1</sourcechannel><channellabel>center</channellabel></audiochannel>\n\
         \t\t\t\t\t\t\t\t</audio>"
    };

    let file1_block = format!(
        "\t\t\t\t\t\t<file id=\"file-1\">\n\
         \t\t\t\t\t\t\t<n>{fn}</n>\n\
         \t\t\t\t\t\t\t<pathurl>{url}</pathurl>\n\
         \t\t\t\t\t\t\t<rate><timebase>{fps}</timebase><ntsc>FALSE</ntsc></rate>\n\
         \t\t\t\t\t\t\t<duration>{sf}</duration>\n\
         \t\t\t\t\t\t\t<timecode>\n\
         \t\t\t\t\t\t\t\t<rate><timebase>{fps}</timebase><ntsc>FALSE</ntsc></rate>\n\
         \t\t\t\t\t\t\t\t<string>00:00:00:00</string><frame>0</frame><displayformat>NDF</displayformat>\n\
         \t\t\t\t\t\t\t</timecode>\n\
         \t\t\t\t\t\t\t<media>\n\
         \t\t\t\t\t\t\t\t<video><samplecharacteristics>\n\
         \t\t\t\t\t\t\t\t\t<rate><timebase>{fps}</timebase><ntsc>FALSE</ntsc></rate>\n\
         \t\t\t\t\t\t\t\t\t<width>1920</width><height>1080</height>\n\
         \t\t\t\t\t\t\t\t\t<anamorphic>FALSE</anamorphic>\n\
         \t\t\t\t\t\t\t\t\t<pixelaspectratio>square</pixelaspectratio>\n\
         \t\t\t\t\t\t\t\t\t<fielddominance>none</fielddominance>\n\
         \t\t\t\t\t\t\t\t</samplecharacteristics></video>\n\
         {audio_media}\n\
         \t\t\t\t\t\t\t</media>\n\
         \t\t\t\t\t\t</file>",
        fn=file_name, url=file_uri, fps=fps, sf=src_end_frames,
        audio_media=cam_audio_media
    );

    // ── Bloc <file> audio externe (file-2, si présent) ─────────────
    let file2_block = ext_audio.map(|(ext_path, _)| {
        let ext_uri  = format!("file://localhost/{}", encode_uri(ext_path.trim_start_matches('/')));
        let ext_name = std::path::Path::new(ext_path)
            .file_name().and_then(|n| n.to_str()).unwrap_or("audio_ext");
        format!(
            "\t\t\t\t\t\t<file id=\"file-2\">\n\
             \t\t\t\t\t\t\t<n>{fn}</n>\n\
             \t\t\t\t\t\t\t<pathurl>{url}</pathurl>\n\
             \t\t\t\t\t\t\t<rate><timebase>{fps}</timebase><ntsc>FALSE</ntsc></rate>\n\
             \t\t\t\t\t\t\t<timecode>\n\
             \t\t\t\t\t\t\t\t<rate><timebase>{fps}</timebase><ntsc>FALSE</ntsc></rate>\n\
             \t\t\t\t\t\t\t\t<string>00:00:00:00</string><frame>0</frame><displayformat>NDF</displayformat>\n\
             \t\t\t\t\t\t\t</timecode>\n\
             \t\t\t\t\t\t\t<media>\n\
             \t\t\t\t\t\t\t\t<audio>\n\
             \t\t\t\t\t\t\t\t\t<samplecharacteristics><depth>16</depth><samplerate>48000</samplerate></samplecharacteristics>\n\
             \t\t\t\t\t\t\t\t\t<channelcount>1</channelcount><layout>stereo</layout>\n\
             \t\t\t\t\t\t\t\t\t<audiochannel><sourcechannel>1</sourcechannel><channellabel>left</channellabel></audiochannel>\n\
             \t\t\t\t\t\t\t\t</audio>\n\
             \t\t\t\t\t\t\t\t<audio>\n\
             \t\t\t\t\t\t\t\t\t<samplecharacteristics><depth>16</depth><samplerate>48000</samplerate></samplecharacteristics>\n\
             \t\t\t\t\t\t\t\t\t<channelcount>1</channelcount><layout>stereo</layout>\n\
             \t\t\t\t\t\t\t\t\t<audiochannel><sourcechannel>2</sourcechannel><channellabel>right</channellabel></audiochannel>\n\
             \t\t\t\t\t\t\t\t</audio>\n\
             \t\t\t\t\t\t\t</media>\n\
             \t\t\t\t\t\t</file>",
            fn=ext_name, url=ext_uri, fps=fps
        )
    });

    // ── Plan des pistes ────────────────────────────────────────────
    // Sans ext : A1=cam gauche  A2=cam droite (si stéréo)
    // Avec ext : A1=ext gauche  A2=ext droite  A3=cam gauche (muté)  A4=cam droite (muté)
    let n_clips = segs.len();
    // IDs clips (1-based) :
    //   vidéo  : 1..n
    //   A1 ext : n+1..2n      (ou cam sans ext)
    //   A2 ext : 2n+1..3n
    //   A3 cam : 3n+1..4n     (muté, seulement si ext)
    //   A4 cam : 4n+1..5n     (muté, stéréo + ext)
    let vid_base  = 0;
    let au1_base  = n_clips;             // A1 (ext ou cam)
    let au2_base  = n_clips * 2;         // A2 (ext droite ou cam droite)
    let au3_base  = n_clips * 3;         // A3 (cam gauche muté, si ext)
    let au4_base  = n_clips * 4;         // A4 (cam droite muté, si ext + stéréo)

    // Nombre total de pistes audio dans le XML
    let n_audio_tracks = if has_ext {
        if is_stereo { 4usize } else { 3usize }
    } else {
        if is_stereo { 2usize } else { 1usize }
    };

    let mut video_clips = String::new();
    let mut au1_clips   = String::new(); // ext gauche (ou cam gauche)
    let mut au2_clips   = String::new(); // ext droite (ou cam droite)
    let mut au3_clips   = String::new(); // cam gauche muté (si ext)
    let mut au4_clips   = String::new(); // cam droite muté (si ext + stéréo)
    let mut record_offset = 0.0f64;

    for (i, s) in segs.iter().enumerate() {
        let dur   = s.end_time - s.start_time;
        let in_f  = (s.start_time * fps as f64).round() as u64;
        let out_f = (s.end_time   * fps as f64).round() as u64;
        let rec_s = (record_offset * fps as f64).round() as u64;
        let rec_e = ((record_offset + dur) * fps as f64).round() as u64;
        let dur_f = out_f - in_f;
        let in_t  = in_f  * ticks_per_frame;
        let out_t = out_f * ticks_per_frame;
        let ci = i + 1;

        let vid_id = vid_base + ci;
        let au1_id = au1_base + ci;
        let au2_id = au2_base + ci;
        let au3_id = au3_base + ci;
        let au4_id = au4_base + ci;

        // Fichier caméra — bloc complet au premier clip, ref courte ensuite
        let file1_ref = if i == 0 { file1_block.clone() }
                        else { "\t\t\t\t\t\t<file id=\"file-1\"/>".to_string() };

        // Fichier ext — bloc complet au premier clip de A1, ref courte ensuite
        let file2_ref_full = file2_block.clone().unwrap_or_default();
        let file2_ref = if i == 0 { file2_ref_full }
                        else { "\t\t\t\t\t\t<file id=\"file-2\"/>".to_string() };

        // ── Liens croisés ─────────────────────────────────────────
        // Même structure que Premiere : chaque clipitem liste tous les clipitems liés
        let mut links = format!(
            "\t\t\t\t\t\t<link><linkclipref>clipitem-{vid}</linkclipref>\
             <mediatype>video</mediatype><trackindex>1</trackindex><clipindex>{ci}</clipindex></link>\n\
             \t\t\t\t\t\t<link><linkclipref>clipitem-{au1}</linkclipref>\
             <mediatype>audio</mediatype><trackindex>1</trackindex><clipindex>{ci}</clipindex><groupindex>1</groupindex></link>\n",
            vid=vid_id, au1=au1_id, ci=ci
        );
        if is_stereo || has_ext {
            links.push_str(&format!(
                "\t\t\t\t\t\t<link><linkclipref>clipitem-{au2}</linkclipref>\
                 <mediatype>audio</mediatype><trackindex>2</trackindex><clipindex>{ci}</clipindex><groupindex>1</groupindex></link>\n",
                au2=au2_id
            ));
        }
        if has_ext {
            links.push_str(&format!(
                "\t\t\t\t\t\t<link><linkclipref>clipitem-{au3}</linkclipref>\
                 <mediatype>audio</mediatype><trackindex>3</trackindex><clipindex>{ci}</clipindex><groupindex>1</groupindex></link>\n",
                au3=au3_id
            ));
            if is_stereo {
                links.push_str(&format!(
                    "\t\t\t\t\t\t<link><linkclipref>clipitem-{au4}</linkclipref>\
                     <mediatype>audio</mediatype><trackindex>4</trackindex><clipindex>{ci}</clipindex><groupindex>1</groupindex></link>\n",
                    au4=au4_id
                ));
            }
        }

        // ── Clip vidéo ────────────────────────────────────────────
        video_clips.push_str(&format!(
            "\t\t\t\t\t<clipitem id=\"clipitem-{vid}\">\n\
             \t\t\t\t\t\t<masterclipid>masterclip-1</masterclipid>\n\
             \t\t\t\t\t\t<enabled>TRUE</enabled>\n\
             \t\t\t\t\t\t<duration>{dur_f}</duration>\n\
             \t\t\t\t\t\t<rate><timebase>{fps}</timebase><ntsc>FALSE</ntsc></rate>\n\
             \t\t\t\t\t\t<start>{rec_s}</start><end>{rec_e}</end>\n\
             \t\t\t\t\t\t<in>{in_f}</in><out>{out_f}</out>\n\
             \t\t\t\t\t\t<pproTicksIn>{in_t}</pproTicksIn><pproTicksOut>{out_t}</pproTicksOut>\n\
             \t\t\t\t\t\t<alphatype>none</alphatype>\n\
             \t\t\t\t\t\t<pixelaspectratio>square</pixelaspectratio>\n\
             \t\t\t\t\t\t<anamorphic>FALSE</anamorphic>\n\
             {file1}\n\
             {links}\t\t\t\t\t</clipitem>\n",
            vid=vid_id, dur_f=dur_f, fps=fps,
            rec_s=rec_s, rec_e=rec_e, in_f=in_f, out_f=out_f,
            in_t=in_t, out_t=out_t, file1=file1_ref, links=links
        ));

        // ── A1 : audio externe gauche (ou caméra gauche si pas d'ext) ─
        let (a1_file, a1_track_idx, a1_enabled, a1_masterclip) = if has_ext {
            (file2_ref.clone(), 1usize, "TRUE", "masterclip-2")
        } else {
            ("\t\t\t\t\t\t<file id=\"file-1\"/>".to_string(), 1usize, "TRUE", "masterclip-1")
        };

        // Timecodes audio externe décalés par ext_offset
        let (a1_in_f, a1_out_f, a1_in_t, a1_out_t) = if has_ext {
            let ei = ((s.start_time + ext_offset) * fps as f64).round() as i64;
            let eo = ((s.end_time   + ext_offset) * fps as f64).round() as i64;
            let ei = ei.max(0) as u64;
            let eo = eo.max(0) as u64;
            (ei, eo, ei * ticks_per_frame, eo * ticks_per_frame)
        } else {
            (in_f, out_f, in_t, out_t)
        };

        au1_clips.push_str(&format!(
            "\t\t\t\t\t<clipitem id=\"clipitem-{au1}\" premiereChannelType=\"mono\">\n\
             \t\t\t\t\t\t<masterclipid>{mc}</masterclipid>\n\
             \t\t\t\t\t\t<enabled>{en}</enabled>\n\
             \t\t\t\t\t\t<duration>{dur_f}</duration>\n\
             \t\t\t\t\t\t<rate><timebase>{fps}</timebase><ntsc>FALSE</ntsc></rate>\n\
             \t\t\t\t\t\t<start>{rec_s}</start><end>{rec_e}</end>\n\
             \t\t\t\t\t\t<in>{ai}</in><out>{ao}</out>\n\
             \t\t\t\t\t\t<pproTicksIn>{ait}</pproTicksIn><pproTicksOut>{aot}</pproTicksOut>\n\
             {file}\n\
             \t\t\t\t\t\t<sourcetrack><mediatype>audio</mediatype><trackindex>{ti}</trackindex></sourcetrack>\n\
             {links}\t\t\t\t\t</clipitem>\n",
            au1=au1_id, mc=a1_masterclip, en=a1_enabled, dur_f=dur_f, fps=fps,
            rec_s=rec_s, rec_e=rec_e, ai=a1_in_f, ao=a1_out_f,
            ait=a1_in_t, aot=a1_out_t, file=a1_file, ti=a1_track_idx, links=links
        ));

        // ── A2 : audio externe droite (ou caméra droite si stéréo sans ext) ─
        if is_stereo || has_ext {
            let (a2_file, a2_track_idx, a2_enabled, a2_masterclip) = if has_ext {
                ("\t\t\t\t\t\t<file id=\"file-2\"/>".to_string(), 2usize, "TRUE", "masterclip-2")
            } else {
                ("\t\t\t\t\t\t<file id=\"file-1\"/>".to_string(), 2usize, "TRUE", "masterclip-1")
            };

            let (a2_in_f, a2_out_f, a2_in_t, a2_out_t) = if has_ext {
                let ei = ((s.start_time + ext_offset) * fps as f64).round() as i64;
                let eo = ((s.end_time   + ext_offset) * fps as f64).round() as i64;
                let ei = ei.max(0) as u64;
                let eo = eo.max(0) as u64;
                (ei, eo, ei * ticks_per_frame, eo * ticks_per_frame)
            } else {
                (in_f, out_f, in_t, out_t)
            };

            au2_clips.push_str(&format!(
                "\t\t\t\t\t<clipitem id=\"clipitem-{au2}\" premiereChannelType=\"mono\">\n\
                 \t\t\t\t\t\t<masterclipid>{mc}</masterclipid>\n\
                 \t\t\t\t\t\t<enabled>{en}</enabled>\n\
                 \t\t\t\t\t\t<duration>{dur_f}</duration>\n\
                 \t\t\t\t\t\t<rate><timebase>{fps}</timebase><ntsc>FALSE</ntsc></rate>\n\
                 \t\t\t\t\t\t<start>{rec_s}</start><end>{rec_e}</end>\n\
                 \t\t\t\t\t\t<in>{ai}</in><out>{ao}</out>\n\
                 \t\t\t\t\t\t<pproTicksIn>{ait}</pproTicksIn><pproTicksOut>{aot}</pproTicksOut>\n\
                 {file}\n\
                 \t\t\t\t\t\t<sourcetrack><mediatype>audio</mediatype><trackindex>{ti}</trackindex></sourcetrack>\n\
                 {links}\t\t\t\t\t</clipitem>\n",
                au2=au2_id, mc=a2_masterclip, en=a2_enabled, dur_f=dur_f, fps=fps,
                rec_s=rec_s, rec_e=rec_e, ai=a2_in_f, ao=a2_out_f,
                ait=a2_in_t, aot=a2_out_t, file=a2_file, ti=a2_track_idx, links=links
            ));
        }

        // ── A3 : caméra gauche mutée (seulement si audio externe présent) ─
        if has_ext {
            au3_clips.push_str(&format!(
                "\t\t\t\t\t<clipitem id=\"clipitem-{au3}\" premiereChannelType=\"mono\">\n\
                 \t\t\t\t\t\t<masterclipid>masterclip-1</masterclipid>\n\
                 \t\t\t\t\t\t<enabled>FALSE</enabled>\n\
                 \t\t\t\t\t\t<duration>{dur_f}</duration>\n\
                 \t\t\t\t\t\t<rate><timebase>{fps}</timebase><ntsc>FALSE</ntsc></rate>\n\
                 \t\t\t\t\t\t<start>{rec_s}</start><end>{rec_e}</end>\n\
                 \t\t\t\t\t\t<in>{in_f}</in><out>{out_f}</out>\n\
                 \t\t\t\t\t\t<pproTicksIn>{in_t}</pproTicksIn><pproTicksOut>{out_t}</pproTicksOut>\n\
                 \t\t\t\t\t\t<file id=\"file-1\"/>\n\
                 \t\t\t\t\t\t<sourcetrack><mediatype>audio</mediatype><trackindex>1</trackindex></sourcetrack>\n\
                 {links}\t\t\t\t\t</clipitem>\n",
                au3=au3_id, dur_f=dur_f, fps=fps,
                rec_s=rec_s, rec_e=rec_e, in_f=in_f, out_f=out_f,
                in_t=in_t, out_t=out_t, links=links
            ));

            // ── A4 : caméra droite mutée (stéréo + ext) ──────────────
            if is_stereo {
                au4_clips.push_str(&format!(
                    "\t\t\t\t\t<clipitem id=\"clipitem-{au4}\" premiereChannelType=\"mono\">\n\
                     \t\t\t\t\t\t<masterclipid>masterclip-1</masterclipid>\n\
                     \t\t\t\t\t\t<enabled>FALSE</enabled>\n\
                     \t\t\t\t\t\t<duration>{dur_f}</duration>\n\
                     \t\t\t\t\t\t<rate><timebase>{fps}</timebase><ntsc>FALSE</ntsc></rate>\n\
                     \t\t\t\t\t\t<start>{rec_s}</start><end>{rec_e}</end>\n\
                     \t\t\t\t\t\t<in>{in_f}</in><out>{out_f}</out>\n\
                     \t\t\t\t\t\t<pproTicksIn>{in_t}</pproTicksIn><pproTicksOut>{out_t}</pproTicksOut>\n\
                     \t\t\t\t\t\t<file id=\"file-1\"/>\n\
                     \t\t\t\t\t\t<sourcetrack><mediatype>audio</mediatype><trackindex>2</trackindex></sourcetrack>\n\
                     {links}\t\t\t\t\t</clipitem>\n",
                    au4=au4_id, dur_f=dur_f, fps=fps,
                    rec_s=rec_s, rec_e=rec_e, in_f=in_f, out_f=out_f,
                    in_t=in_t, out_t=out_t, links=links
                ));
            }
        }

        record_offset += dur;
    }

    // ── Construction des pistes audio ──────────────────────────────
    let make_track = |clips: &str, idx: usize, total: usize, out_ch: usize| -> String {
        format!(
            "\t\t\t\t<track MZ.TrackTargeted=\"1\" premiereTrackType=\"Stereo\"\
             currentExplodedTrackIndex=\"{i}\" totalExplodedTrackCount=\"{total}\">\n\
             {clips}\t\t\t\t\t<enabled>TRUE</enabled>\n\
             \t\t\t\t\t<locked>FALSE</locked>\n\
             \t\t\t\t\t<outputchannelindex>{out_ch}</outputchannelindex>\n\
             \t\t\t\t</track>\n",
            i=idx, total=total, clips=clips, out_ch=out_ch
        )
    };

    let mut audio_tracks = String::new();
    audio_tracks.push_str(&make_track(&au1_clips, 0, n_audio_tracks, 1));
    if is_stereo || has_ext {
        audio_tracks.push_str(&make_track(&au2_clips, 1, n_audio_tracks, 2));
    }
    if has_ext {
        audio_tracks.push_str(&make_track(&au3_clips, 2, n_audio_tracks, 1));
        if is_stereo {
            audio_tracks.push_str(&make_track(&au4_clips, 3, n_audio_tracks, 2));
        }
    }

    // ── outputs block ──────────────────────────────────────────────
    let outputs_block = "\t\t\t\t<outputs>\n\
         \t\t\t\t\t<group><index>1</index><numchannels>1</numchannels><downmix>0</downmix><channel><index>1</index></channel></group>\n\
         \t\t\t\t\t<group><index>2</index><numchannels>1</numchannels><downmix>0</downmix><channel><index>2</index></channel></group>\n\
         \t\t\t\t</outputs>";

    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
         <!DOCTYPE xmeml>\n\
         <xmeml version=\"4\">\n\
         \t<sequence id=\"sequence-1\" MZ.Sequence.PreviewFrameSizeHeight=\"1080\" \
         MZ.Sequence.PreviewFrameSizeWidth=\"1920\" MZ.Sequence.AudioTimeDisplayFormat=\"200\" \
         MZ.Sequence.VideoTimeDisplayFormat=\"108\" explodedTracks=\"true\">\n\
         \t\t<uuid>trimlab-{title}-seq</uuid>\n\
         \t\t<duration>{tf}</duration>\n\
         \t\t<rate><timebase>{fps}</timebase><ntsc>FALSE</ntsc></rate>\n\
         \t\t<n>{title}</n>\n\
         \t\t<media>\n\
         \t\t\t<video>\n\
         \t\t\t\t<format><samplecharacteristics>\n\
         \t\t\t\t\t<rate><timebase>{fps}</timebase><ntsc>FALSE</ntsc></rate>\n\
         \t\t\t\t\t<width>1920</width><height>1080</height>\n\
         \t\t\t\t\t<anamorphic>FALSE</anamorphic>\n\
         \t\t\t\t\t<pixelaspectratio>square</pixelaspectratio>\n\
         \t\t\t\t\t<fielddominance>none</fielddominance>\n\
         \t\t\t\t\t<colordepth>24</colordepth>\n\
         \t\t\t\t</samplecharacteristics></format>\n\
         \t\t\t\t<track MZ.TrackTargeted=\"1\">\n\
         {vc}\t\t\t\t\t<enabled>TRUE</enabled>\n\
         \t\t\t\t\t<locked>FALSE</locked>\n\
         \t\t\t\t</track>\n\
         \t\t\t</video>\n\
         \t\t\t<audio>\n\
         \t\t\t\t<numOutputChannels>2</numOutputChannels>\n\
         \t\t\t\t<format><samplecharacteristics>\n\
         \t\t\t\t\t<depth>16</depth><samplerate>48000</samplerate>\n\
         \t\t\t\t</samplecharacteristics></format>\n\
         {ob}\n\
         {at}\t\t\t</audio>\n\
         \t\t</media>\n\
         \t\t<timecode>\n\
         \t\t\t<rate><timebase>{fps}</timebase><ntsc>FALSE</ntsc></rate>\n\
         \t\t\t<string>00:00:00:00</string><frame>0</frame><displayformat>NDF</displayformat>\n\
         \t\t</timecode>\n\
         \t</sequence>\n\
         </xmeml>",
        title=title, tf=total_frames, fps=fps,
        vc=video_clips, ob=outputs_block, at=audio_tracks
    )
}

// ── Conversion MKV→MP4 via FFmpeg stream-copy ─────────────────────────────
// Tente d'abord un stream-copy (ultra-rapide, sans ré-encodage).
// Si FFmpeg signale une incompatibilité de codec, ré-encode en H.264/AAC.
#[tauri::command]
fn convert_source_to_mp4(project_id: String, input_path: String, output_path: String) -> Result<String, String> {
    use std::process::Command;

    // ── Tentative 1 : stream-copy ─────────────────────────────────────────
    let status_copy = Command::new("ffmpeg")
        .args(["-y", "-i", &input_path,
               "-c", "copy",
               "-movflags", "+faststart",
               &output_path])
        .status()
        .map_err(|e| format!("ffmpeg introuvable : {}", e))?;

    if status_copy.success() {
        return Ok(output_path);
    }

    // ── Tentative 2 : ré-encodage H.264 + AAC ────────────────────────────
    let status_encode = Command::new("ffmpeg")
        .args(["-y", "-i", &input_path,
               "-c:v", "libx264", "-preset", "fast", "-crf", "18",
               "-c:a", "aac", "-b:a", "192k",
               "-movflags", "+faststart",
               &output_path])
        .status()
        .map_err(|e| format!("ffmpeg introuvable : {}", e))?;

    if status_encode.success() {
        Ok(output_path)
    } else {
        Err(format!("Échec de la conversion pour le projet {}", project_id))
    }
}

// ── Mise à jour du chemin source en base ──────────────────────────────────
#[tauri::command]
fn update_media_file_path(project_id: String, new_path: String) -> Result<(), String> {
    use crate::db;
    let conn = db::open().map_err(|e| e.to_string())?;
    conn.execute(
        "UPDATE media_files SET path = ?1 WHERE project_id = ?2",
        rusqlite::params![new_path, project_id],
    ).map_err(|e: rusqlite::Error| format!("DB update_media_file_path : {}", e))?;
    Ok(())
}

// ══════════════════════════════════════════════════════════════════════════════
// CHAPITRES YOUTUBE
// ══════════════════════════════════════════════════════════════════════════════

#[derive(serde::Serialize)]
struct ChapterMark {
    /// Temps en secondes dans la vidéo ÉDITÉE (après suppression silences)
    time_edited: f64,
    /// Temps dans la source brute (pour référence)
    time_source: f64,
    /// Titre généré depuis les premiers mots du segment
    title: String,
}

/// Extrait jusqu'à `max_words` mots dans [t_start, t_end] depuis la liste words triée
fn words_in_range(
    words: &[(f64, f64, String)],
    t_start: f64,
    t_end: f64,
    max_words: usize,
) -> String {
    words.iter()
        .filter(|(s, e, _)| *s >= t_start - 0.05 && *e <= t_end + 0.05)
        .take(max_words)
        .map(|(_, _, w)| w.trim().to_string())
        .collect::<Vec<_>>()
        .join(" ")
}

/// Capitalise la première lettre
fn capitalize(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        None => String::new(),
        Some(f) => f.to_uppercase().to_string() + c.as_str(),
    }
}

#[tauri::command]
fn generate_chapters(project_id: String) -> Result<Vec<ChapterMark>, String> {
    use crate::db;
    use crate::commands::segments::list_segments;

    let conn = db::open().map_err(|e| e.to_string())?;

    // ── 1. Keep segments ──────────────────────────────────────────────────────
    let segs = list_segments(project_id.clone())?;
    let keeps: Vec<_> = segs.iter().filter(|s| s.seg_type == "keep").collect();
    if keeps.is_empty() {
        return Ok(vec![]);
    }

    // ── 2. Mots de transcription (optionnel) ──────────────────────────────────
    let words: Vec<(f64, f64, String)> = conn
        .prepare(
            "SELECT start_time, end_time, word FROM transcript_words              WHERE project_id=?1 ORDER BY start_time ASC",
        )
        .and_then(|mut stmt| {
            stmt.query_map(rusqlite::params![project_id], |row| {
                Ok((
                    row.get::<_, f64>(0)?,
                    row.get::<_, f64>(1)?,
                    row.get::<_, String>(2)?,
                ))
            })
            .and_then(|rows| rows.collect())
        })
        .unwrap_or_default();

    // ── 3. Détection des frontières de chapitres ──────────────────────────────
    // Règles :
    //   • Toujours un chapitre à 00:00
    //   • Nouvelle frontière si :  gap silence >= MIN_GAP  ET  durée depuis dernier >= MIN_DURATION
    //   • Maximum MAX_CHAPTERS chapitres (évite les micro-chapitres)
    // Durée totale éditée pour calibrer les seuils dynamiquement
    let total_edited: f64 = keeps.iter().map(|s| s.end_time - s.start_time).sum();

    // Seuils adaptatifs : courts pour les vidéos courtes, plus larges sinon
    // MIN_GAP  : gap de silence suffisant pour marquer un changement de sujet
    // MIN_DURATION : durée minimum entre deux chapitres (évite micro-chapitres)
    let min_gap: f64 = if total_edited < 120.0 { 1.5 }
                       else if total_edited < 600.0 { 2.5 }
                       else { 4.0 };
    let min_duration: f64 = if total_edited < 120.0 { 15.0 }
                            else if total_edited < 600.0 { 30.0 }
                            else { 45.0 };
    const MAX_CHAPTERS: usize = 20;

    let mut chapters: Vec<ChapterMark> = vec![];

    // Temps "édité" = position dans la vidéo après coupures
    // On le calcule en accumulant la durée des keeps.
    let mut edited_cursor: f64 = 0.0;
    let mut last_chapter_edited: f64 = 0.0;

    // Chapitre 0 : toujours 00:00
    let intro_title = {
        let t = words_in_range(&words, keeps[0].start_time, keeps[0].start_time + 12.0, 5);
        if t.is_empty() { "Intro".to_string() } else { capitalize(&t) }
    };
    chapters.push(ChapterMark {
        time_edited: 0.0,
        time_source: keeps[0].start_time,
        title: intro_title,
    });

    for i in 0..keeps.len() {
        let keep = keeps[i];
        let keep_dur = keep.end_time - keep.start_time;

        if i > 0 {
            let gap = keep.start_time - keeps[i - 1].end_time;
            let since_last = edited_cursor - last_chapter_edited;

            if gap >= min_gap && since_last >= min_duration && chapters.len() < MAX_CHAPTERS {
                let title = {
                    let t = words_in_range(&words, keep.start_time, keep.start_time + 12.0, 5);
                    if t.is_empty() {
                        format!("Partie {}", chapters.len() + 1)
                    } else {
                        capitalize(&t)
                    }
                };
                chapters.push(ChapterMark {
                    time_edited: edited_cursor,
                    time_source: keep.start_time,
                    title,
                });
                last_chapter_edited = edited_cursor;
            }
        }

        edited_cursor += keep_dur;
    }

    Ok(chapters)
}

// ── Écriture fichier texte (chapitres YouTube, etc.) ──────────────────────────
#[tauri::command]
fn save_text_file(path: String, content: String) -> Result<(), String> {
    std::fs::write(&path, content.as_bytes())
        .map_err(|e| format!("Impossible d'écrire {path}: {e}"))
}