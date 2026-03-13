// ============================================================
// FICHIER : trimlab/src-tauri/src/commands/segments.rs
// ROLE    : CRUD segments + analyse silence avancée
//
// NOUVEAUTÉS :
//   - padding_before / padding_after appliqués aux segments keep
//   - min_speech_ms : ignore les keep trop courts
//   - threshold_db = 0.0 → mode "auto" (ffmpeg volumedetect)
//   - aggressiveness 1-5 : ajuste noise floor et durée min silence
//   - get_silence_presets : presets podcast/vlog/interview/tutoriel/cinema
// ============================================================

use crate::db;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Segment {
    pub id: String,
    pub project_id: String,
    pub start_time: f64,
    pub end_time: f64,
    pub seg_type: String,
    pub confidence: f64,
    pub label: Option<String>,
}

// ─── LIST ────────────────────────────────────────────────────────────────────

#[tauri::command]
pub fn list_segments(project_id: String) -> Result<Vec<Segment>, String> {
    let conn = db::open().map_err(|e| e.to_string())?;
    let mut stmt = conn.prepare(
        "SELECT id, project_id, start_time, end_time, seg_type, confidence, label
         FROM segments WHERE project_id=?1 ORDER BY start_time"
    ).map_err(|e| e.to_string())?;

    let segs = stmt.query_map([&project_id], |row| {
        Ok(Segment {
            id:         row.get(0)?,
            project_id: row.get(1)?,
            start_time: row.get(2)?,
            end_time:   row.get(3)?,
            seg_type:   row.get(4)?,
            confidence: row.get(5)?,
            label:      row.get(6)?,
        })
    })
    .map_err(|e| e.to_string())?
    .filter_map(|r| r.ok())
    .collect();

    Ok(segs)
}

// ─── SAVE BATCH ──────────────────────────────────────────────────────────────

#[tauri::command]
pub fn save_segments(project_id: String, segments: Vec<Segment>) -> Result<(), String> {
    let mut conn = db::open().map_err(|e| e.to_string())?;
    let tx = conn.transaction().map_err(|e| e.to_string())?;

    tx.execute("DELETE FROM segments WHERE project_id=?1", [&project_id])
        .map_err(|e| e.to_string())?;

    for seg in &segments {
        let id = if seg.id.is_empty() { Uuid::new_v4().to_string() } else { seg.id.clone() };
        tx.execute(
            "INSERT INTO segments (id, project_id, start_time, end_time, seg_type, confidence, label)
             VALUES (?1,?2,?3,?4,?5,?6,?7)",
            rusqlite::params![
                id, project_id, seg.start_time, seg.end_time,
                seg.seg_type, seg.confidence, seg.label
            ],
        ).map_err(|e| e.to_string())?;
    }

    tx.commit().map_err(|e| e.to_string())?;
    Ok(())
}

// ─── TOGGLE TYPE ─────────────────────────────────────────────────────────────

#[tauri::command]
pub fn toggle_segment(segment_id: String) -> Result<Segment, String> {
    let conn = db::open().map_err(|e| e.to_string())?;

    let current_type: String = conn.query_row(
        "SELECT seg_type FROM segments WHERE id=?1",
        [&segment_id],
        |r| r.get(0),
    ).map_err(|e| format!("Segment introuvable: {}", e))?;

    let new_type = match current_type.as_str() {
        "keep"    => "cut",
        "cut"     => "keep",
        "silence" => "keep",
        "filler"  => "keep",
        _         => "keep",
    };

    conn.execute(
        "UPDATE segments SET seg_type=?1 WHERE id=?2",
        rusqlite::params![new_type, segment_id],
    ).map_err(|e| e.to_string())?;

    conn.query_row(
        "SELECT id, project_id, start_time, end_time, seg_type, confidence, label
         FROM segments WHERE id=?1",
        [&segment_id],
        |row| Ok(Segment {
            id:         row.get(0)?,
            project_id: row.get(1)?,
            start_time: row.get(2)?,
            end_time:   row.get(3)?,
            seg_type:   row.get(4)?,
            confidence: row.get(5)?,
            label:      row.get(6)?,
        }),
    ).map_err(|e| e.to_string())
}

// ─── DELETE ONE ──────────────────────────────────────────────────────────────

#[tauri::command]
pub fn delete_segment(segment_id: String) -> Result<(), String> {
    let conn = db::open().map_err(|e| e.to_string())?;
    conn.execute("DELETE FROM segments WHERE id=?1", [&segment_id])
        .map_err(|e| e.to_string())?;
    Ok(())
}

// ─── ANALYSE SILENCE ─────────────────────────────────────────────────────────
//
// threshold_db = 0.0  → mode auto (volumedetect)
// aggressiveness 1-5  → ajuste seuil (+/-1.5dB) et durée min silence
// padding_before/after → marges conservées autour des coupes
// min_speech_ms        → segments keep plus courts → marqués silence
//
// RÉTROCOMPATIBILITÉ : padding/min_speech/aggressiveness ont des defaults
// → l'ancien appel à 5 args continue de fonctionner (padding=0, aggr=3)

#[tauri::command]
pub fn analyse_and_save(
    project_id:      String,
    file_path:       String,
    threshold_db:    f64,
    min_duration_ms: f64,
    duration:        f64,
    padding_before:  Option<f64>,
    padding_after:   Option<f64>,
    min_speech_ms:   Option<f64>,
    aggressiveness:  Option<u8>,
) -> Result<Vec<Segment>, String> {
    let padding_before  = padding_before.unwrap_or(0.0);
    let padding_after   = padding_after.unwrap_or(0.0);
    let min_speech_ms   = min_speech_ms.unwrap_or(0.0);
    let aggressiveness  = aggressiveness.unwrap_or(3);

    // ── Résoudre le seuil effectif ────────────────────────────────────────────
    let aggr_offset = (aggressiveness.clamp(1, 5) as f64 - 3.0) * 1.5;

    let effective_threshold = if threshold_db == 0.0 {
        auto_detect_threshold(&file_path)
            .map(|mean| (mean + 5.0 + aggr_offset).clamp(-60.0, -20.0))
            .unwrap_or(-35.0 + aggr_offset)
    } else {
        (threshold_db + aggr_offset).clamp(-60.0, -20.0)
    };

    // Aggressiveness réduit la durée min de silence (détecte des silences plus courts)
    let aggr_dur_factor = match aggressiveness.clamp(1, 5) {
        1 => 1.4,
        2 => 1.2,
        3 => 1.0,
        4 => 0.8,
        5 => 0.6,
        _ => 1.0,
    };
    let effective_min_duration_s = (min_duration_ms / 1000.0) * aggr_dur_factor;

    // ── FFmpeg silencedetect ──────────────────────────────────────────────────
    let output = std::process::Command::new(ffmpeg_bin())
        .args([
            "-i", &file_path,
            "-af", &format!(
                "silencedetect=noise={:.2}dB:duration={:.4}",
                effective_threshold, effective_min_duration_s
            ),
            "-f", "null", "-"
        ])
        .output()
        .map_err(|e| format!("FFmpeg introuvable: {}", e))?;

    let stderr = String::from_utf8_lossy(&output.stderr);
    let silences = parse_silences(&stderr);

    // ── Construire segments avec padding + min_speech ─────────────────────────
    let min_speech_s = (min_speech_ms / 1000.0).max(0.05);
    let mut segments: Vec<Segment> = Vec::new();
    let mut cursor = 0.0f64;

    for (sil_start, sil_end) in &silences {
        // Padding : on "mord" sur le silence des deux côtés
        // keep se termine à sil_start + padding_after (on garde un peu après le dernier mot)
        // keep suivant commence à sil_end - padding_before (on commence avant le prochain mot)
        let keep_end   = (*sil_start + padding_after).min(*sil_end);
        let next_start = (*sil_end   - padding_before).max(keep_end);

        // Segment keep avant ce silence
        let seg_dur = keep_end - cursor;
        if seg_dur >= min_speech_s {
            segments.push(Segment {
                id:         Uuid::new_v4().to_string(),
                project_id: project_id.clone(),
                start_time: cursor,
                end_time:   keep_end.min(duration),
                seg_type:   "keep".into(),
                confidence: 1.0,
                label:      None,
            });
        }

        // Segment silence (zone réellement coupée)
        let sil_dur = next_start - keep_end;
        if sil_dur > 0.01 {
            segments.push(Segment {
                id:         Uuid::new_v4().to_string(),
                project_id: project_id.clone(),
                start_time: keep_end,
                end_time:   next_start.min(duration),
                seg_type:   "silence".into(),
                confidence: 0.95,
                label:      None,
            });
        }

        cursor = next_start;
    }

    // Segment keep final
    let final_dur = duration - cursor;
    if final_dur >= min_speech_s {
        segments.push(Segment {
            id:         Uuid::new_v4().to_string(),
            project_id: project_id.clone(),
            start_time: cursor,
            end_time:   duration,
            seg_type:   "keep".into(),
            confidence: 1.0,
            label:      None,
        });
    }

    save_segments_internal(&project_id, &segments)?;
    Ok(segments)
}

// ─── Mode auto-threshold ─────────────────────────────────────────────────────


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
fn auto_detect_threshold(file_path: &str) -> Option<f64> {
    let out = std::process::Command::new(ffmpeg_bin())
        .args(["-i", file_path, "-af", "volumedetect", "-f", "null", "-"])
        .output()
        .ok()?;

    let stderr = String::from_utf8_lossy(&out.stderr);
    for line in stderr.lines() {
        if line.contains("mean_volume:") {
            let pos = line.find("mean_volume:")?;
            let rest = line[pos + 12..].trim();
            let end = rest.find(" dB").unwrap_or(rest.len());
            return rest[..end].trim().parse().ok();
        }
    }
    None
}

// ─── Presets ─────────────────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SilencePreset {
    pub threshold_db:    f64,
    pub min_duration_ms: f64,
    pub padding_before:  f64,
    pub padding_after:   f64,
    pub min_speech_ms:   f64,
    pub aggressiveness:  u8,
    pub description:     String,
}

#[tauri::command]
pub fn get_silence_presets() -> Vec<(String, SilencePreset)> {
    vec![
        ("podcast".into(), SilencePreset {
            threshold_db: 0.0, min_duration_ms: 300.0,
            padding_before: 0.05, padding_after: 0.12,
            min_speech_ms: 200.0, aggressiveness: 3,
            description: "Podcast / interview : coupes naturelles".into(),
        }),
        ("vlog".into(), SilencePreset {
            threshold_db: 0.0, min_duration_ms: 200.0,
            padding_before: 0.03, padding_after: 0.08,
            min_speech_ms: 150.0, aggressiveness: 3,
            description: "Vlog / caméra : rythme dynamique".into(),
        }),
        ("interview".into(), SilencePreset {
            threshold_db: 0.0, min_duration_ms: 450.0,
            padding_before: 0.08, padding_after: 0.18,
            min_speech_ms: 300.0, aggressiveness: 2,
            description: "Interview : préserve les pauses de réflexion".into(),
        }),
        ("tutoriel".into(), SilencePreset {
            threshold_db: 0.0, min_duration_ms: 220.0,
            padding_before: 0.03, padding_after: 0.08,
            min_speech_ms: 150.0, aggressiveness: 4,
            description: "Tutoriel : agressif, supprime toutes les hésitations".into(),
        }),
        ("cinema".into(), SilencePreset {
            threshold_db: -45.0, min_duration_ms: 600.0,
            padding_before: 0.12, padding_after: 0.25,
            min_speech_ms: 500.0, aggressiveness: 1,
            description: "Court-métrage : conserve les silences dramatiques".into(),
        }),
    ]
}

// ─── Helpers internes ────────────────────────────────────────────────────────

fn parse_silences(stderr: &str) -> Vec<(f64, f64)> {
    let mut starts: Vec<f64> = Vec::new();
    let mut ends:   Vec<f64> = Vec::new();

    for line in stderr.lines() {
        if line.contains("silence_start:") {
            if let Some(t) = extract_time(line, "silence_start:") {
                starts.push(t);
            }
        }
        if line.contains("silence_end:") {
            if let Some(t) = extract_time(line, "silence_end:") {
                ends.push(t);
            }
        }
    }

    starts.into_iter().zip(ends).collect()
}

fn extract_time(line: &str, key: &str) -> Option<f64> {
    let pos = line.find(key)?;
    let rest = line[pos + key.len()..].trim();
    let end = rest.find(|c: char| !c.is_ascii_digit() && c != '.' && c != '-').unwrap_or(rest.len());
    rest[..end].parse().ok()
}

fn save_segments_internal(project_id: &str, segments: &[Segment]) -> Result<(), String> {
    let mut conn = db::open().map_err(|e| e.to_string())?;
    let tx = conn.transaction().map_err(|e| e.to_string())?;

    tx.execute(
        "DELETE FROM segments WHERE project_id=?1 AND seg_type IN ('keep','silence','cut')",
        [project_id]
    ).map_err(|e| e.to_string())?;

    for seg in segments {
        tx.execute(
            "INSERT INTO segments (id, project_id, start_time, end_time, seg_type, confidence, label)
             VALUES (?1,?2,?3,?4,?5,?6,?7)",
            rusqlite::params![seg.id, project_id, seg.start_time, seg.end_time,
                              seg.seg_type, seg.confidence, seg.label],
        ).map_err(|e| e.to_string())?;
    }
    tx.commit().map_err(|e| e.to_string())
}