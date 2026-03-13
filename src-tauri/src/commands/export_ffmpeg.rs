// ============================================================
// FICHIER : trimlab/src-tauri/src/commands/export_ffmpeg.rs
// ROLE    : Export vidéo direct via FFmpeg
// ============================================================

use serde::{Deserialize, Serialize};
use std::process::Command;
use std::path::PathBuf;
use crate::db;

#[derive(Debug, Deserialize)]
pub struct ExportFfmpegOpts {
    pub project_id:  String,
    pub output_path: String,
    pub mode:        String,
    #[allow(dead_code)] // format déduit par FFmpeg depuis l'extension du output_path
    pub format:      String,
}

#[derive(Debug, Serialize)]
pub struct ExportFfmpegResult {
    pub success:           bool,
    pub output_path:       String,
    pub segments_exported: usize,
    pub duration_saved:    f64,
    pub error:             Option<String>,
}

#[tauri::command]
pub async fn export_ffmpeg(opts: ExportFfmpegOpts) -> Result<ExportFfmpegResult, String> {

    // conn et stmt droppés AVANT tout .await — obligatoire pour Send
    let (media_path, segments, duration_saved) = {
        let conn = db::open().map_err(|e: rusqlite::Error| e.to_string())?;

        let media_path: String = conn
            .query_row(
                "SELECT path FROM media_files WHERE project_id = ?1 ORDER BY added_at LIMIT 1",
                rusqlite::params![opts.project_id],
                |r: &rusqlite::Row| r.get(0),
            )
            .map_err(|e: rusqlite::Error| format!("Fichier média introuvable: {}", e))?;

        let total_duration: f64 = conn
            .query_row(
                "SELECT duration FROM media_files WHERE project_id = ?1 ORDER BY added_at LIMIT 1",
                rusqlite::params![opts.project_id],
                |r: &rusqlite::Row| r.get(0),
            )
            .unwrap_or(0.0);

        let mut stmt = conn
            .prepare(
                "SELECT start_time, end_time FROM segments
                 WHERE project_id = ?1 AND seg_type = 'keep'
                 ORDER BY start_time",
            )
            .map_err(|e: rusqlite::Error| e.to_string())?;

        let segments: Vec<(f64, f64)> = stmt
            .query_map(rusqlite::params![opts.project_id], |r: &rusqlite::Row| {
                Ok((r.get::<_, f64>(0)?, r.get::<_, f64>(1)?))
            })
            .map_err(|e: rusqlite::Error| e.to_string())?
            .filter_map(|r: Result<(f64, f64), rusqlite::Error>| r.ok())
            .collect();

        // [FIX] Fusionner les segments qui se chevauchent AVANT export
        // Sans ça : zones communes encodées deux fois → phrases répétées
        let segments = merge_keep_segments(segments);

        let kept: f64 = segments.iter().map(|(s, e)| e - s).sum();
        let duration_saved = (total_duration - kept).max(0.0);

        (media_path, segments, duration_saved)
        // conn + stmt droppés ici
    };

    if segments.is_empty() {
        return Ok(ExportFfmpegResult {
            success: false,
            output_path: opts.output_path,
            segments_exported: 0,
            duration_saved: 0.0,
            error: Some("Aucun segment à exporter".into()),
        });
    }

    let segments_exported = segments.len();

    // Fonctions sync — pas d'await donc pas de problème Send
    let result = if opts.mode == "copy" {
        export_concat_demuxer(&media_path, &segments, &opts.output_path)
    } else {
        export_reencode(&media_path, &segments, &opts.output_path)
    };

    match result {
        Ok(()) => Ok(ExportFfmpegResult {
            success: true,
            output_path: opts.output_path,
            segments_exported,
            duration_saved,
            error: None,
        }),
        Err(e) => Ok(ExportFfmpegResult {
            success: false,
            output_path: opts.output_path,
            segments_exported: 0,
            duration_saved: 0.0,
            error: Some(e),
        }),
    }
}

// ── Merge segments keep qui se chevauchent ou sont contigus ─────────────────
// Problème : après analyse silence + Whisper, des segments keep peuvent se
// chevaucher (ex: keep[0-5s] + keep[3-7s]) → FFmpeg encode la zone 3-5s deux
// fois → phrases répétées dans la vidéo finale.
// Cette fonction fusionne les overlaps avant tout export.

// ── Résolution du binaire FFmpeg ─────────────────────────────────────────────
// 1. Cherche le sidecar bundlé dans le même dossier que l'exe (production)
// 2. Fallback sur le PATH système (développement)
fn ffmpeg_bin() -> String {
    if let Ok(exe) = std::env::current_exe() {
        let sidecar = exe
            .parent()
            .unwrap_or(&std::path::PathBuf::from("."))
            .join("ffmpeg.exe");
        if sidecar.exists() {
            return sidecar.to_string_lossy().to_string();
        }
    }
    "ffmpeg".to_string()
}
fn merge_keep_segments(segs: Vec<(f64, f64)>) -> Vec<(f64, f64)> {
    if segs.is_empty() { return segs; }

    let mut sorted = segs;
    sorted.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());

    let mut merged: Vec<(f64, f64)> = Vec::new();
    let (mut cur_start, mut cur_end) = sorted[0];

    for (s, e) in sorted.into_iter().skip(1) {
        // Gap < 50ms → considéré contigu, on fusionne
        if s <= cur_end + 0.05 {
            cur_end = cur_end.max(e);
        } else {
            if cur_end - cur_start > 0.05 {
                merged.push((cur_start, cur_end));
            }
            cur_start = s;
            cur_end   = e;
        }
    }
    if cur_end - cur_start > 0.05 {
        merged.push((cur_start, cur_end));
    }
    merged
}

// ── Mode "copy" : concat demuxer via .ts intermédiaires ──────────────────────
fn export_concat_demuxer(
    media_path: &str,
    segments: &[(f64, f64)],
    output_path: &str,
) -> Result<(), String> {

    let tmp_dir = std::env::temp_dir().join("autotrim_concat");
    std::fs::create_dir_all(&tmp_dir).map_err(|e: std::io::Error| e.to_string())?;

    let mut tmp_files: Vec<std::path::PathBuf> = Vec::new();
    let mut list_content = String::new();

    for (i, (start, end)) in segments.iter().enumerate() {
        let dur = end - start;
        let tmp_path = tmp_dir.join(format!("seg_{:04}.ts", i));

        let status = Command::new(ffmpeg_bin())
            .args(["-y",
                   "-ss", &format!("{:.6}", start),
                   "-t",  &format!("{:.6}", dur),
                   "-i",  media_path,
                   "-c",  "copy",
                   "-avoid_negative_ts", "make_zero",
                   "-fflags", "+genpts",
                   tmp_path.to_str().unwrap()])
            .output()
            .map_err(|e: std::io::Error| format!("FFmpeg introuvable: {}", e))?;

        if !status.status.success() {
            for f in &tmp_files { let _ = std::fs::remove_file(f); }
            let stderr = String::from_utf8_lossy(&status.stderr);
            return Err(format!("Erreur segment {}: {}",
                i, stderr.lines().last().unwrap_or("?")));
        }

        list_content.push_str(&format!("file '{}'\n",
            tmp_path.to_str().unwrap().replace('\\', "/")));
        tmp_files.push(tmp_path);
    }

    let list_path = tmp_dir.join("concat_list.txt");
    std::fs::write(&list_path, &list_content)
        .map_err(|e: std::io::Error| format!("Erreur écriture liste: {}", e))?;

    let status = Command::new(ffmpeg_bin())
        .args(["-y",
               "-f", "concat",
               "-safe", "0",
               "-i", list_path.to_str().unwrap(),
               "-c", "copy",
               output_path])
        .output()
        .map_err(|e: std::io::Error| format!("FFmpeg introuvable: {}", e))?;

    for f in &tmp_files { let _ = std::fs::remove_file(f); }
    let _ = std::fs::remove_file(&list_path);
    let _ = std::fs::remove_dir(&tmp_dir);

    if !status.status.success() {
        let stderr = String::from_utf8_lossy(&status.stderr);
        return Err(format!("Erreur concat: {}",
            stderr.lines().last().unwrap_or("Erreur inconnue")));
    }

    Ok(())
}

// ── Mode "reencode" : H.264 CRF18 + AAC 192k, frame-accurate ─────────────────
fn export_reencode(
    media_path: &str,
    segments: &[(f64, f64)],
    output_path: &str,
) -> Result<(), String> {

    let n = segments.len();

    let inputs_filter: String = (0..n)
        .map(|i| format!("[{}:v][{}:a]", i, i))
        .collect::<Vec<_>>()
        .join("");

    let filter_complex = format!("{}concat=n={}:v=1:a=1[v][a]", inputs_filter, n);

    let mut cmd = Command::new(ffmpeg_bin());
    cmd.arg("-y");

    for (start, end) in segments {
        let dur = end - start;
        cmd.args(["-ss", &format!("{:.6}", start)]);
        cmd.args(["-t",  &format!("{:.6}", dur)]);
        cmd.args(["-i",  media_path]);
    }

    cmd.args(["-filter_complex", &filter_complex]);
    cmd.args(["-map",     "[v]"]);
    cmd.args(["-map",     "[a]"]);
    cmd.args(["-vcodec",  "libx264"]);
    cmd.args(["-crf",     "18"]);
    cmd.args(["-preset",  "fast"]);
    cmd.args(["-pix_fmt", "yuv420p"]);
    cmd.args(["-acodec",  "aac"]);
    cmd.args(["-b:a",     "192k"]);
    cmd.args(["-avoid_negative_ts", "make_zero"]);
    cmd.arg(output_path);

    let output = cmd.output()
        .map_err(|e: std::io::Error| format!("FFmpeg introuvable: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Erreur réencodage: {}",
            stderr.lines().last().unwrap_or("Erreur inconnue")));
    }

    Ok(())
}