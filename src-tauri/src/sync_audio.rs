// ============================================================
// FICHIER : src-tauri/src/sync_audio.rs
// ROLE    : Synchronisation automatique vidéo + audio externe
//           via corrélation croisée des formes d'onde
// ============================================================

use serde::{Deserialize, Serialize};
use uuid::Uuid;
use std::path::PathBuf;
use crate::db;

// ─── Types ────────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncResult {
    pub success:      bool,
    pub offset_secs:  f64,   // > 0 : audio démarre après la vidéo
                              // < 0 : audio démarre avant la vidéo
    pub confidence:   f64,   // 0.0 – 1.0
    pub method:       String, // "xcorr" | "transient"
    pub error:        Option<String>,
}

// ─── Commande principale ───────────────────────────────────────
//
// Calcule l'offset entre la piste audio de `video_path` et
// le fichier audio externe `audio_path`, puis persiste le
// résultat dans media_files (external_audio_path + audio_offset).

#[tauri::command]
pub fn sync_external_audio(
    project_id:  String,
    video_path:  String,
    audio_path:  String,
) -> Result<SyncResult, String> {

    // 1. Extraire les deux formes d'onde en PCM mono 16 kHz
    let video_pcm = extract_pcm(&video_path, "video_ref")?;
    let audio_pcm = extract_pcm(&audio_path, "audio_ext")?;

    // 2. Corrélation croisée → offset en samples
    let sr = 16000usize;
    let (offset_samples, confidence) = cross_correlate(&video_pcm, &audio_pcm, sr);
    let offset_secs = offset_samples as f64 / sr as f64;

    // 3. Si confiance trop faible, fallback sur détection de transient
    let (final_offset, method) = if confidence > 0.15 {
        (offset_secs, "xcorr".to_string())
    } else {
        let t_video = first_transient(&video_pcm, sr);
        let t_audio = first_transient(&audio_pcm, sr);
        let fallback_offset = t_audio - t_video;
        (fallback_offset, "transient".to_string())
    };

    // 4. Persister dans la DB
    let conn = db::open().map_err(|e| e.to_string())?;
    conn.execute(
        "UPDATE media_files
         SET external_audio_path = ?1, audio_offset = ?2
         WHERE project_id = ?3",
        rusqlite::params![audio_path, final_offset, project_id],
    ).map_err(|e| e.to_string())?;

    println!(
        "[TrimLab Sync] offset={:.3}s confidence={:.2} method={}",
        final_offset, confidence, method
    );

    Ok(SyncResult {
        success:     true,
        offset_secs: final_offset,
        confidence,
        method,
        error: None,
    })
}

// ─── Extraction PCM via FFmpeg ─────────────────────────────────
//
// Extrait la piste audio en mono 16 kHz f32le dans un fichier
// temporaire, puis charge les samples en Vec<f32>.
// On limite à 60 secondes pour que la corrélation reste rapide.


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
fn extract_pcm(source: &str, tag: &str) -> Result<Vec<f32>, String> {
    let tmp = std::env::temp_dir()
        .join(format!("autotrim_sync_{}_{}.raw", tag, Uuid::new_v4()));

    let out = std::process::Command::new(ffmpeg_bin())
        .args([
            "-y", "-i", source,
            "-t", "60",               // limiter à 60s
            "-ar", "16000",
            "-ac", "1",
            "-f", "f32le",
            tmp.to_str().unwrap(),
        ])
        .output()
        .map_err(|e| format!("ffmpeg introuvable: {}", e))?;

    if !out.status.success() {
        return Err(String::from_utf8_lossy(&out.stderr).to_string());
    }

    let bytes = std::fs::read(&tmp).map_err(|e| e.to_string())?;
    let _ = std::fs::remove_file(&tmp);

    let samples: Vec<f32> = bytes
        .chunks_exact(4)
        .map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]]))
        .collect();

    Ok(samples)
}

// ─── Corrélation croisée ───────────────────────────────────────
//
// Implémentation directe O(n·m) sur les 30 premières secondes.
// Retourne (offset_en_samples, confiance).
// offset > 0 signifie que l'audio externe commence APRÈS la vidéo.

fn cross_correlate(reference: &[f32], query: &[f32], sr: usize) -> (i64, f64) {
    // On travaille sur 30s max pour rester rapide
    let max_samples = sr * 30;
    let ref_win: &[f32] = &reference[..reference.len().min(max_samples)];
    let qry_win: &[f32] = &query[..query.len().min(max_samples)];

    // Fenêtre de recherche : ±15 secondes
    let search_window = (sr * 15) as i64;

    // Normalisation RMS pour rendre la corrélation indépendante du volume
    let rms = |s: &[f32]| -> f32 {
        let sum: f32 = s.iter().map(|x| x * x).sum();
        (sum / s.len() as f32).sqrt().max(1e-10)
    };
    let ref_rms = rms(ref_win);
    let qry_rms = rms(qry_win);

    let chunk = 4096usize; // taille du bloc de comparaison
    let _ref_chunk = &ref_win[..ref_win.len().min(chunk)];

    let mut best_offset = 0i64;
    let mut best_score  = f64::NEG_INFINITY;

    let ref_len = ref_win.len() as i64;
    let qry_len = qry_win.len() as i64;

    let start = -search_window;
    let end   =  search_window;

    // Pas de 100 samples pour accélérer (~6ms à 16kHz)
    let step = 100i64;

    for lag in (start..end).step_by(step as usize) {
        let mut sum = 0.0f64;
        let mut count = 0usize;

        for i in 0..chunk as i64 {
            let ri = i;
            let qi = i - lag;
            if ri >= 0 && ri < ref_len && qi >= 0 && qi < qry_len {
                sum += (ref_win[ri as usize] / ref_rms) as f64
                     * (qry_win[qi as usize] / qry_rms) as f64;
                count += 1;
            }
        }

        if count > 0 {
            let score = sum / count as f64;
            if score > best_score {
                best_score  = score;
                best_offset = lag;
            }
        }
    }

    // Raffinement autour du meilleur offset (pas de 10 samples)
    for lag in (best_offset - step)..(best_offset + step) {
        let mut sum = 0.0f64;
        let mut count = 0usize;
        for i in 0..chunk as i64 {
            let ri = i;
            let qi = i - lag;
            if ri >= 0 && ri < ref_len && qi >= 0 && qi < qry_len {
                sum += (ref_win[ri as usize] / ref_rms) as f64
                     * (qry_win[qi as usize] / qry_rms) as f64;
                count += 1;
            }
        }
        if count > 0 {
            let score = sum / count as f64;
            if score > best_score {
                best_score  = score;
                best_offset = lag;
            }
        }
    }

    let confidence = best_score.clamp(0.0, 1.0);
    (best_offset, confidence)
}

// ─── Détection du premier transient ───────────────────────────
//
// Cherche le premier pic d'énergie brutal (clap, snap, etc.)
// en glissant une fenêtre et en comparant l'énergie avant/après.
// Retourne la position en secondes.

fn first_transient(samples: &[f32], sr: usize) -> f64 {
    let window = sr / 100; // 10ms
    let threshold_ratio = 8.0f32; // énergie × 8 = transient

    for i in window..samples.len().saturating_sub(window) {
        let before: f32 = samples[i - window..i]
            .iter().map(|x| x * x).sum::<f32>() / window as f32;
        let after: f32 = samples[i..i + window]
            .iter().map(|x| x * x).sum::<f32>() / window as f32;

        if before > 1e-8 && after / before > threshold_ratio {
            return i as f64 / sr as f64;
        }
    }

    // Aucun transient trouvé → début du fichier
    0.0
}

// ─── Récupérer les infos de synchro pour un projet ────────────

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AudioSyncInfo {
    pub external_audio_path: Option<String>,
    pub audio_offset:        Option<f64>,
}

#[tauri::command]
pub fn get_audio_sync_info(project_id: String) -> Result<AudioSyncInfo, String> {
    let conn = db::open().map_err(|e| e.to_string())?;
    let result = conn.query_row(
        "SELECT external_audio_path, audio_offset
         FROM media_files WHERE project_id = ?1
         ORDER BY added_at ASC LIMIT 1",
        rusqlite::params![project_id],
        |row| Ok(AudioSyncInfo {
            external_audio_path: row.get(0)?,
            audio_offset:        row.get(1)?,
        }),
    ).map_err(|e| e.to_string())?;
    Ok(result)
}

// ─── Dissocier l'audio externe d'un projet ────────────────────

#[tauri::command]
pub fn remove_external_audio(project_id: String) -> Result<(), String> {
    let conn = db::open().map_err(|e| e.to_string())?;
    conn.execute(
        "UPDATE media_files
         SET external_audio_path = NULL, audio_offset = NULL
         WHERE project_id = ?1",
        rusqlite::params![project_id],
    ).map_err(|e| e.to_string())?;
    Ok(())
}