// ============================================================
// FICHIER : trimlab/src-tauri/src/commands/export_audio.rs
// ROLE    : Export audio seul (MP3 / WAV / AAC) via FFmpeg
// ============================================================

use serde::{Deserialize, Serialize};
use std::process::Command;
use std::path::PathBuf;
use crate::db;

#[derive(Debug, Deserialize)]
pub struct ExportAudioOpts {
    pub project_id: String,
    pub output_path: String,
    pub format: String,
    pub quality: String,
}

#[derive(Debug, Serialize)]
pub struct ExportAudioResult {
    pub success: bool,
    pub output_path: String,
    pub segments_exported: usize,
    pub error: Option<String>,
}


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
#[tauri::command]
pub async fn export_audio(opts: ExportAudioOpts) -> Result<ExportAudioResult, String> {
    let conn = db::open().map_err(|e: rusqlite::Error| e.to_string())?;

    let media_path: String = conn
        .query_row(
            "SELECT path FROM media_files WHERE project_id = ?1 ORDER BY added_at LIMIT 1",
            rusqlite::params![opts.project_id],
            |r: &rusqlite::Row| r.get(0),
        )
        .map_err(|e: rusqlite::Error| format!("Fichier média introuvable: {}", e))?;

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

    if segments.is_empty() {
        return Ok(ExportAudioResult {
            success: false,
            output_path: opts.output_path,
            segments_exported: 0,
            error: Some("Aucun segment à exporter".into()),
        });
    }

    let segments_exported = segments.len();

    // Construire le filtre concat audio
    let filter_parts: Vec<String> = (0..segments.len())
        .map(|i| format!("[{}:a]", i))
        .collect();

    let filter_complex = format!(
        "{}concat=n={}:v=0:a=1[outa]",
        filter_parts.join(""),
        segments.len()
    );

    // Paramètres codec selon format + qualité
    let (codec, bitrate): (&str, &str) = match opts.format.as_str() {
        "wav" => ("pcm_s16le", ""),
        "aac" => (
            "aac",
            match opts.quality.as_str() {
                "low"    => "128k",
                "medium" => "192k",
                _        => "256k",
            },
        ),
        _ => (
            "libmp3lame",
            match opts.quality.as_str() {
                "low"    => "128k",
                "medium" => "192k",
                _        => "320k",
            },
        ),
    };

    let mut cmd = Command::new(ffmpeg_bin());
    cmd.arg("-y");

    for (start, end) in &segments {
        let dur = end - start;
        cmd.args(["-ss", &format!("{:.6}", start)]);
        cmd.args(["-t",  &format!("{:.6}", dur)]);
        cmd.args(["-i",  &media_path]);
    }

    cmd.args(["-filter_complex", &filter_complex]);
    cmd.args(["-map", "[outa]"]);
    cmd.args(["-acodec", codec]);

    if !bitrate.is_empty() {
        cmd.args(["-b:a", bitrate]);
        if opts.format == "mp3" {
            cmd.args(["-q:a", "0"]);
        }
    }

    cmd.arg(&opts.output_path);

    let output = cmd.output().map_err(|e| format!("FFmpeg introuvable: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Ok(ExportAudioResult {
            success: false,
            output_path: opts.output_path,
            segments_exported: 0,
            error: Some(
                stderr.lines().last().unwrap_or("Erreur inconnue").to_string()
            ),
        });
    }

    Ok(ExportAudioResult {
        success: true,
        output_path: opts.output_path,
        segments_exported,
        error: None,
    })
}