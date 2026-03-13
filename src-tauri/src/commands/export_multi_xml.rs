// ============================================================
// FICHIER : trimlab/src-tauri/src/commands/export_multi_xml.rs
// ROLE    : Fusion de plusieurs projets en une seule timeline XML
//           Structure calquée exactement sur l'export TrimLab single-projet
// ============================================================

use serde::{Deserialize, Serialize};
use crate::db;

#[derive(Debug, Deserialize)]
pub struct MultiExportOpts {
    pub project_ids: Vec<String>,
    pub output_path: String,
    pub title:       String,
}

#[derive(Debug, Serialize)]
pub struct MultiExportResult {
    pub success:         bool,
    pub output_path:     String,
    pub clips_exported:  usize,
    pub projects_merged: usize,
    pub error:           Option<String>,
}

struct ProjectData {
    name:        String,
    source_path: String,
    fps:         u64,
    total_frames: u64,   // durée totale du fichier source en frames
    is_stereo:   bool,
    segments:    Vec<(f64, f64)>,
}

#[tauri::command]
pub fn export_multi_xml(opts: MultiExportOpts) -> Result<MultiExportResult, String> {
    if opts.project_ids.is_empty() {
        return Ok(MultiExportResult {
            success: false,
            output_path: opts.output_path,
            clips_exported: 0,
            projects_merged: 0,
            error: Some("Aucun projet sélectionné".into()),
        });
    }

    let conn = db::open().map_err(|e: rusqlite::Error| e.to_string())?;
    let mut projects: Vec<ProjectData> = Vec::new();

    for pid in &opts.project_ids {
        let source_path: String = conn
            .query_row(
                "SELECT path FROM media_files WHERE project_id=?1 ORDER BY added_at ASC LIMIT 1",
                rusqlite::params![pid],
                |r: &rusqlite::Row| r.get(0),
            )
            .map_err(|e: rusqlite::Error| format!("Projet {}: fichier introuvable: {}", pid, e))?;

        let name: String = conn
            .query_row(
                "SELECT name FROM projects WHERE id=?1",
                rusqlite::params![pid],
                |r: &rusqlite::Row| r.get(0),
            )
            .unwrap_or_else(|_| pid.clone());

        let fps: u64 = conn
            .query_row(
                "SELECT COALESCE(fps, 25) FROM media_files WHERE project_id=?1 ORDER BY added_at ASC LIMIT 1",
                rusqlite::params![pid],
                |r: &rusqlite::Row| r.get::<_, f64>(0),
            )
            .unwrap_or(25.0)
            .round() as u64;

        // Durée totale du fichier source (pour le tag <duration> dans <file>)
        let duration_secs: f64 = conn
            .query_row(
                "SELECT COALESCE(duration, 0) FROM media_files WHERE project_id=?1 ORDER BY added_at ASC LIMIT 1",
                rusqlite::params![pid],
                |r: &rusqlite::Row| r.get::<_, f64>(0),
            )
            .unwrap_or(0.0);
        let total_frames = (duration_secs * fps as f64).round() as u64;

        let channels: i64 = conn
            .query_row(
                "SELECT COALESCE(sample_rate, 2) FROM media_files WHERE project_id=?1 ORDER BY added_at ASC LIMIT 1",
                rusqlite::params![pid],
                |r: &rusqlite::Row| r.get(0),
            )
            .unwrap_or(2);

        let mut stmt = conn
            .prepare(
                "SELECT start_time, end_time FROM segments
                 WHERE project_id=?1 AND seg_type='keep'
                 ORDER BY start_time",
            )
            .map_err(|e: rusqlite::Error| e.to_string())?;

        let segments: Vec<(f64, f64)> = stmt
            .query_map(rusqlite::params![pid], |r: &rusqlite::Row| {
                Ok((r.get::<_, f64>(0)?, r.get::<_, f64>(1)?))
            })
            .map_err(|e: rusqlite::Error| e.to_string())?
            .filter_map(|r: Result<(f64, f64), rusqlite::Error>| r.ok())
            .collect();

        if !segments.is_empty() {
            projects.push(ProjectData {
                name,
                source_path,
                fps,
                total_frames,
                is_stereo: channels >= 2,
                segments,
            });
        }
    }

    if projects.is_empty() {
        return Ok(MultiExportResult {
            success: false,
            output_path: opts.output_path,
            clips_exported: 0,
            projects_merged: 0,
            error: Some("Aucun segment à exporter dans les projets sélectionnés".into()),
        });
    }

    let timeline_fps = projects[0].fps;
    let total_clips: usize = projects.iter().map(|p| p.segments.len()).sum();
    let xml = build_multi_xml(&projects, &opts.title, timeline_fps);

    if let Some(parent) = std::path::Path::new(&opts.output_path).parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    std::fs::write(&opts.output_path, xml).map_err(|e| e.to_string())?;

    Ok(MultiExportResult {
        success: true,
        output_path: opts.output_path,
        clips_exported: total_clips,
        projects_merged: projects.len(),
        error: None,
    })
}

fn encode_path(p: &str) -> String {
    // Premiere attend %3a (minuscule) et %20 — calqué sur le XML de référence
    let forward = p.replace('\\', "/");
    let encoded: String = forward
        .chars()
        .map(|c| match c {
            ':' => "%3a".to_string(),  // IMPORTANT: minuscule — Premiere refuse %3A
            ' ' => "%20".to_string(),
            c   => c.to_string(),
        })
        .collect();
    format!("file://localhost/{}", encoded)
}

fn file_basename(path: &str) -> String {
    // "D:\obs\foo.mp4" -> "foo.mp4"
    path.replace('\\', "/")
        .split('/')
        .last()
        .unwrap_or(path)
        .to_string()
}

fn build_multi_xml(projects: &[ProjectData], title: &str, fps: u64) -> String {
    let ticks: u64 = 254016000000 / fps;
    let has_stereo = projects.iter().any(|p| p.is_stereo);

    // IDs audio : même schéma que le XML de référence single-projet
    // vidéo : 1..N
    // audio A1 : N+1 .. 2N   (offset = n_video)
    // audio A2 : 2N+1 .. 3N  (offset = 2*n_video)
    let n_video: usize = projects.iter().map(|p| p.segments.len()).sum();

    let mut video_clips = String::new();
    let mut au1_clips   = String::new();
    let mut au2_clips   = String::new();
    let mut rec: u64 = 0;
    let mut vi = 0usize; // index clip vidéo (0-based)
    let mut file_done = vec![false; projects.len()];

    for (fi, proj) in projects.iter().enumerate() {
        let fid = fi + 1;

        for (_, (start, end)) in proj.segments.iter().enumerate() {
            let in_f  = (start * fps as f64).round() as u64;
            let out_f = (end   * fps as f64).round() as u64;
            let dur_f = out_f - in_f;
            let in_t  = in_f  * ticks;
            let out_t = out_f * ticks;
            let rec_s = rec;
            let rec_e = rec + dur_f;

            // IDs calqués sur le XML de référence
            let vc_id  = vi + 1;
            let au1_id = n_video + vi + 1;
            let au2_id = 2 * n_video + vi + 1;
            let ci     = vi + 1;

            // <file> complet uniquement sur le premier clipitem vidéo de chaque source
            let file_block = if !file_done[fi] {
                file_done[fi] = true;
                format!(
                    "\t\t\t\t\t\t<file id=\"file-{fid}\">\n\
                     \t\t\t\t\t\t\t<n>{basename}</n>\n\
                     \t\t\t\t\t\t\t<pathurl>{url}</pathurl>\n\
                     \t\t\t\t\t\t\t<rate><timebase>{fps}</timebase><ntsc>FALSE</ntsc></rate>\n\
                     \t\t\t\t\t\t\t<duration>{total}</duration>\n\
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
                     \t\t\t\t\t\t</file>\n",
                    fid=fid,
                    basename=file_basename(&proj.source_path),
                    url=encode_path(&proj.source_path),
                    fps=fps,
                    total=proj.total_frames,
                )
            } else {
                format!("\t\t\t\t\t\t<file id=\"file-{fid}\"/>\n", fid=fid)
            };

            // Links avec <groupindex> comme dans la référence
            let links_v = format!(
                "\t\t\t\t\t\t<link><linkclipref>clipitem-{vc}</linkclipref><mediatype>video</mediatype><trackindex>1</trackindex><clipindex>{ci}</clipindex></link>\n\
                 \t\t\t\t\t\t<link><linkclipref>clipitem-{au1}</linkclipref><mediatype>audio</mediatype><trackindex>1</trackindex><clipindex>{ci}</clipindex><groupindex>1</groupindex></link>\n\
                 \t\t\t\t\t\t<link><linkclipref>clipitem-{au2}</linkclipref><mediatype>audio</mediatype><trackindex>2</trackindex><clipindex>{ci}</clipindex><groupindex>1</groupindex></link>\n",
                vc=vc_id, au1=au1_id, au2=au2_id, ci=ci
            );
            let links_a1 = format!(
                "\t\t\t\t\t\t<link><linkclipref>clipitem-{vc}</linkclipref><mediatype>video</mediatype><trackindex>1</trackindex><clipindex>{ci}</clipindex></link>\n\
                 \t\t\t\t\t\t<link><linkclipref>clipitem-{au1}</linkclipref><mediatype>audio</mediatype><trackindex>1</trackindex><clipindex>{ci}</clipindex><groupindex>1</groupindex></link>\n\
                 \t\t\t\t\t\t<link><linkclipref>clipitem-{au2}</linkclipref><mediatype>audio</mediatype><trackindex>2</trackindex><clipindex>{ci}</clipindex><groupindex>1</groupindex></link>\n",
                vc=vc_id, au1=au1_id, au2=au2_id, ci=ci
            );
            let links_a2 = links_a1.clone();

            // Clip vidéo — avec alphatype + pixelaspectratio + anamorphic comme la référence
            video_clips.push_str(&format!(
                "\t\t\t\t\t<clipitem id=\"clipitem-{vc}\">\n\
                 \t\t\t\t\t\t<masterclipid>masterclip-{fid}</masterclipid>\n\
                 \t\t\t\t\t\t<enabled>TRUE</enabled>\n\
                 \t\t\t\t\t\t<duration>{dur}</duration>\n\
                 \t\t\t\t\t\t<rate><timebase>{fps}</timebase><ntsc>FALSE</ntsc></rate>\n\
                 \t\t\t\t\t\t<start>{rs}</start><end>{re}</end>\n\
                 \t\t\t\t\t\t<in>{inf}</in><out>{outf}</out>\n\
                 \t\t\t\t\t\t<pproTicksIn>{int}</pproTicksIn><pproTicksOut>{outt}</pproTicksOut>\n\
                 \t\t\t\t\t\t<alphatype>none</alphatype>\n\
                 \t\t\t\t\t\t<pixelaspectratio>square</pixelaspectratio>\n\
                 \t\t\t\t\t\t<anamorphic>FALSE</anamorphic>\n\
                 {file}{links}\t\t\t\t\t</clipitem>\n",
                vc=vc_id, fid=fid, dur=dur_f, fps=fps,
                rs=rec_s, re=rec_e, inf=in_f, outf=out_f,
                int=in_t, outt=out_t,
                file=file_block, links=links_v
            ));

            let file_ref = format!("\t\t\t\t\t\t<file id=\"file-{fid}\"/>\n", fid=fid);

            // Clip audio A1
            au1_clips.push_str(&format!(
                "\t\t\t\t\t<clipitem id=\"clipitem-{au1}\" premiereChannelType=\"mono\">\n\
                 \t\t\t\t\t\t<masterclipid>masterclip-{fid}</masterclipid>\n\
                 \t\t\t\t\t\t<enabled>TRUE</enabled>\n\
                 \t\t\t\t\t\t<duration>{dur}</duration>\n\
                 \t\t\t\t\t\t<rate><timebase>{fps}</timebase><ntsc>FALSE</ntsc></rate>\n\
                 \t\t\t\t\t\t<start>{rs}</start><end>{re}</end>\n\
                 \t\t\t\t\t\t<in>{inf}</in><out>{outf}</out>\n\
                 \t\t\t\t\t\t<pproTicksIn>{int}</pproTicksIn><pproTicksOut>{outt}</pproTicksOut>\n\
                 {file}\
                 \t\t\t\t\t\t<sourcetrack><mediatype>audio</mediatype><trackindex>1</trackindex></sourcetrack>\n\
                 {links}\t\t\t\t\t</clipitem>\n",
                au1=au1_id, fid=fid, dur=dur_f, fps=fps,
                rs=rec_s, re=rec_e, inf=in_f, outf=out_f,
                int=in_t, outt=out_t,
                file=file_ref, links=links_a1
            ));

            // Clip audio A2
            if proj.is_stereo {
                let file_ref2 = format!("\t\t\t\t\t\t<file id=\"file-{fid}\"/>\n", fid=fid);
                au2_clips.push_str(&format!(
                    "\t\t\t\t\t<clipitem id=\"clipitem-{au2}\" premiereChannelType=\"mono\">\n\
                     \t\t\t\t\t\t<masterclipid>masterclip-{fid}</masterclipid>\n\
                     \t\t\t\t\t\t<enabled>TRUE</enabled>\n\
                     \t\t\t\t\t\t<duration>{dur}</duration>\n\
                     \t\t\t\t\t\t<rate><timebase>{fps}</timebase><ntsc>FALSE</ntsc></rate>\n\
                     \t\t\t\t\t\t<start>{rs}</start><end>{re}</end>\n\
                     \t\t\t\t\t\t<in>{inf}</in><out>{outf}</out>\n\
                     \t\t\t\t\t\t<pproTicksIn>{int}</pproTicksIn><pproTicksOut>{outt}</pproTicksOut>\n\
                     {file}\
                     \t\t\t\t\t\t<sourcetrack><mediatype>audio</mediatype><trackindex>2</trackindex></sourcetrack>\n\
                     {links}\t\t\t\t\t</clipitem>\n",
                    au2=au2_id, fid=fid, dur=dur_f, fps=fps,
                    rs=rec_s, re=rec_e, inf=in_f, outf=out_f,
                    int=in_t, outt=out_t,
                    file=file_ref2, links=links_a2
                ));
            }

            rec += dur_f;
            vi  += 1;
        }
    }

    let total_frames = rec;
    let n_audio = if has_stereo { 2 } else { 1 };

    // Track audio — pas d'espace avant currentExplodedTrackIndex (comme la référence)
    let track_a1 = format!(
        "\t\t\t\t<track MZ.TrackTargeted=\"1\" premiereTrackType=\"Stereo\"currentExplodedTrackIndex=\"0\" totalExplodedTrackCount=\"{n}\">\n\
         {clips}\t\t\t\t\t<enabled>TRUE</enabled>\n\
         \t\t\t\t\t<locked>FALSE</locked>\n\
         \t\t\t\t\t<outputchannelindex>1</outputchannelindex>\n\
         \t\t\t\t</track>\n",
        n=n_audio, clips=au1_clips
    );
    let track_a2 = if has_stereo {
        format!(
            "\t\t\t\t<track MZ.TrackTargeted=\"1\" premiereTrackType=\"Stereo\"currentExplodedTrackIndex=\"1\" totalExplodedTrackCount=\"{n}\">\n\
             {clips}\t\t\t\t\t<enabled>TRUE</enabled>\n\
             \t\t\t\t\t<locked>FALSE</locked>\n\
             \t\t\t\t\t<outputchannelindex>2</outputchannelindex>\n\
             \t\t\t\t</track>\n",
            n=n_audio, clips=au2_clips
        )
    } else {
        String::new()
    };

    // UUID sans accents ni espaces
    let safe_title: String = title.chars().map(|c| {
        if c.is_ascii_alphanumeric() || c == '-' { c } else { '-' }
    }).collect();

    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
         <!DOCTYPE xmeml>\n\
         <xmeml version=\"4\">\n\
         \t<sequence id=\"sequence-1\" MZ.Sequence.PreviewFrameSizeHeight=\"1080\" \
         MZ.Sequence.PreviewFrameSizeWidth=\"1920\" MZ.Sequence.AudioTimeDisplayFormat=\"200\" \
         MZ.Sequence.VideoTimeDisplayFormat=\"108\" explodedTracks=\"true\">\n\
         \t\t<uuid>trimlab-multi-{uid}</uuid>\n\
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
         \t\t\t\t<outputs>\n\
         \t\t\t\t\t<group><index>1</index><numchannels>1</numchannels><downmix>0</downmix><channel><index>1</index></channel></group>\n\
         \t\t\t\t\t<group><index>2</index><numchannels>1</numchannels><downmix>0</downmix><channel><index>2</index></channel></group>\n\
         \t\t\t\t</outputs>\n\
         {a1}{a2}\t\t\t</audio>\n\
         \t\t</media>\n\
         \t\t<timecode>\n\
         \t\t\t<rate><timebase>{fps}</timebase><ntsc>FALSE</ntsc></rate>\n\
         \t\t\t<string>00:00:00:00</string><frame>0</frame><displayformat>NDF</displayformat>\n\
         \t\t</timecode>\n\
         \t</sequence>\n\
         </xmeml>",
        uid=safe_title, title=title, tf=total_frames, fps=fps,
        vc=video_clips, a1=track_a1, a2=track_a2
    )
}