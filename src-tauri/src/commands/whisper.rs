// ============================================================
// FICHIER : src-tauri/src/commands/whisper.rs
// ROLE    : Transcription Whisper avec nettoyage anti-hallucination
// ============================================================

use serde::{Deserialize, Serialize};
use std::io::Write;
use std::path::PathBuf;
use uuid::Uuid;
use tauri::{AppHandle, Emitter};

use whisper_rs::{FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters};
use futures_util::StreamExt;

use crate::db;

// ─── Types ────────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Word {
    pub word: String,
    pub start: f64,
    pub end: f64,
    pub confidence: f64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct TranscriptSegment {
    pub start: f64,
    pub end: f64,
    pub text: String,
    pub words: Vec<Word>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TranscribeResult {
    pub success: bool,
    pub language: String,
    pub segments_count: usize,
    pub words_count: usize,
    pub fillers_found: usize,
    pub repeats_found: usize,
    pub error: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WhisperConfig {
    pub project_id: String,
    pub model_path: String,
    pub language: String,
    pub detect_fillers: bool,
    pub detect_repeats: bool,
    pub repeat_window: f64,
    pub repeat_threshold: f64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BatchJobConfig {
    pub project_id:       String,
    pub model_path:       String,
    pub language:         String,
    pub detect_fillers:   bool,
    pub detect_repeats:   bool,
    pub repeat_window:    f64,
    pub repeat_threshold: f64,
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct BatchJobProgress {
    pub project_id:    String,
    pub step:          String,
    pub job_index:     usize,
    pub total_jobs:    usize,
    pub words_count:   usize,
    pub fillers_found: usize,
    pub repeats_found: usize,
    pub error:         Option<String>,
}

// ─── Auto-Download ─────────────────────────────────────────────

fn get_models_dir() -> Result<PathBuf, String> {
    let data_dir = dirs::data_dir()
        .ok_or("Impossible de trouver le dossier utilisateur")?;
    let model_dir = data_dir.join("TrimLab").join("models");
    std::fs::create_dir_all(&model_dir).map_err(|e| e.to_string())?;
    Ok(model_dir)
}

#[tauri::command]
pub fn get_local_model_path(model_name: String) -> Result<String, String> {
    let dir = get_models_dir()?;
    let filename = model_filename(&model_name)?;
    Ok(dir.join(filename).to_string_lossy().to_string())
}

fn model_filename(model_name: &str) -> Result<&'static str, String> {
    match model_name {
        "tiny"   => Ok("ggml-tiny.bin"),
        "base"   => Ok("ggml-base.bin"),
        "small"  => Ok("ggml-small.bin"),
        "medium" => Ok("ggml-medium.bin"),
        "large"  => Ok("ggml-large-v3.bin"),
        _ => Err(format!("Modèle inconnu: {}", model_name))
    }
}

fn model_url(model_name: &str) -> Result<&'static str, String> {
    match model_name {
        "tiny"   => Ok("https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-tiny.bin"),
        "base"   => Ok("https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-base.bin"),
        "small"  => Ok("https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-small.bin"),
        "medium" => Ok("https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-medium.bin"),
        "large"  => Ok("https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-large-v3.bin"),
        _ => Err(format!("Modèle inconnu: {}", model_name))
    }
}

#[tauri::command]
pub async fn ensure_whisper_model(model_name: String, app_handle: AppHandle) -> Result<String, String> {
    let dir = get_models_dir()?;
    let filename = model_filename(&model_name)?;
    let path = dir.join(filename);
    if path.exists() {
        return Ok(path.to_string_lossy().to_string());
    }

    let url = model_url(&model_name)?;
    println!("[TrimLab] Téléchargement {} depuis {}", model_name, url);
    let client = reqwest::Client::new();
    let res = client.get(url).send().await.map_err(|e| format!("Erreur réseau: {}", e))?;
    if !res.status().is_success() {
        return Err(format!("Erreur serveur: {}", res.status()));
    }

    let total_size = res.content_length().unwrap_or(0);
    let mut file = std::fs::File::create(&path).map_err(|e| e.to_string())?;
    let mut downloaded: u64 = 0;
    let mut stream = res.bytes_stream();

    while let Some(item) = stream.next().await {
        let chunk = item.map_err(|e| format!("Erreur stream: {}", e))?;
        file.write_all(&chunk).map_err(|e| format!("Erreur écriture: {}", e))?;
        downloaded += chunk.len() as u64;
        if total_size > 0 {
            let _ = app_handle.emit(
                "whisper-download-progress",
                (downloaded as f64 / total_size as f64) * 100.0,
            );
        }
    }

    println!("[TrimLab] Téléchargement terminé: {:?}", path);
    Ok(path.to_string_lossy().to_string())
}

// ─── Commandes Tauri ───────────────────────────────────────────

#[tauri::command]
pub fn check_whisper_available() -> bool { true }

#[tauri::command]
pub fn get_transcript_words(project_id: String) -> Result<Vec<Word>, String> {
    let conn = db::open().map_err(|e| e.to_string())?;
    let mut stmt = conn.prepare(
        "SELECT word, start_time, end_time, confidence \
         FROM transcript_words WHERE project_id = ?1 ORDER BY start_time ASC"
    ).map_err(|e| e.to_string())?;

    let words = stmt.query_map(rusqlite::params![project_id], |row| {
        Ok(Word {
            word:       row.get(0)?,
            start:      row.get(1)?,
            end:        row.get(2)?,
            confidence: row.get(3)?,
        })
    }).map_err(|e| e.to_string())?
    .filter_map(|w| w.ok())
    .collect();

    Ok(words)
}

#[tauri::command]
pub fn delete_project_words(project_id: String) -> Result<(), String> {
    let conn = crate::db::open().map_err(|e| e.to_string())?;
    conn.execute(
        "DELETE FROM transcript_words WHERE project_id = ?1",
        rusqlite::params![project_id],
    ).map_err(|e| e.to_string())?;
    Ok(())
}

// ─── transcribe_and_detect (mode single) ───────────────────────
// [FIX BUG 1] Émet des events de progression via app_handle
// [FIX BUG 9] Nettoie les anciens mots/segments avant d'insérer

#[tauri::command]
pub fn transcribe_and_detect(config: WhisperConfig, app_handle: AppHandle) -> Result<TranscribeResult, String> {
    let conn = db::open().map_err(|e| e.to_string())?;

    let source_path: String = conn.query_row(
        "SELECT path FROM media_files WHERE project_id=?1 ORDER BY added_at ASC LIMIT 1",
        rusqlite::params![config.project_id],
        |row| row.get(0),
    ).map_err(|_| "Fichier source introuvable".to_string())?;

    // [FIX BUG 9] Nettoyer les anciens mots et segments avant de retranscrire
    conn.execute("DELETE FROM transcript_words WHERE project_id = ?1", rusqlite::params![config.project_id])
        .map_err(|e| e.to_string())?;
    conn.execute("DELETE FROM segments WHERE project_id = ?1 AND seg_type IN ('filler','repeat')", rusqlite::params![config.project_id])
        .map_err(|e| e.to_string())?;

    // [FIX BUG 1] Émettre le step "extracting"
    let _ = app_handle.emit("whisper-single-progress", "extracting");
    let wav_path = extract_audio_wav(&source_path)?;

    // [FIX BUG 1] Émettre le step "transcribing"
    let _ = app_handle.emit("whisper-single-progress", "transcribing");
    let (transcript, language) = run_whisper(&wav_path, &config.model_path, &config.language)?;
    let _ = std::fs::remove_file(&wav_path);

    let mut repeats_found = 0;
    if config.detect_repeats {
        let repeats = detect_repetitions(&transcript, config.repeat_window, config.repeat_threshold);
        repeats_found = repeats.len();
        save_repeat_segments(&config.project_id, &repeats, &conn)?;
    }

    let cleaned     = clean_hallucinations(transcript);
    let words_count = cleaned.iter().map(|s| s.words.len()).sum::<usize>();
    save_transcript(&config.project_id, &cleaned, &conn)?;

    // [FIX BUG 1] Émettre le step "detecting"
    let _ = app_handle.emit("whisper-single-progress", "detecting");

    let mut fillers_found = 0;
    if config.detect_fillers {
        let fillers = detect_fillers_multiword(&cleaned, &language);
        fillers_found = fillers.len();
        save_filler_segments(&config.project_id, &fillers, &conn)?;
    }

    Ok(TranscribeResult {
        success: true,
        language,
        segments_count: cleaned.len(),
        words_count,
        fillers_found,
        repeats_found,
        error: None,
    })
}

// ─── transcribe_batch ──────────────────────────────────────────

#[tauri::command]
pub async fn transcribe_batch(
    jobs: Vec<BatchJobConfig>,
    app_handle: AppHandle,
) -> Result<usize, String> {
    let total = jobs.len();
    let app   = app_handle.clone();

    tokio::task::spawn_blocking(move || {
        run_batch_sequential(jobs, app);
    }).await.map_err(|e| e.to_string())?;

    Ok(total)
}

fn run_batch_sequential(jobs: Vec<BatchJobConfig>, app: AppHandle) {
    let total = jobs.len();

    let mut groups: Vec<(String, Vec<(usize, BatchJobConfig)>)> = Vec::new();
    for (idx, job) in jobs.into_iter().enumerate() {
        if let Some(group) = groups.iter_mut().find(|(m, _)| m == &job.model_path) {
            group.1.push((idx, job));
        } else {
            groups.push((job.model_path.clone(), vec![(idx, job)]));
        }
    }

    for (model_path, group_jobs) in groups {
        println!("[TrimLab Batch] Chargement modèle : {}", model_path);

        for (idx, job) in &group_jobs {
            let _ = app.emit("whisper-batch-progress", BatchJobProgress {
                project_id:    job.project_id.clone(),
                step:          "loading".to_string(),
                job_index:     *idx,
                total_jobs:    total,
                words_count:   0,
                fillers_found: 0,
                repeats_found: 0,
                error:         None,
            });
        }

        let ctx = match WhisperContext::new_with_params(
            &model_path,
            WhisperContextParameters::default(),
        ) {
            Ok(c) => c,
            Err(e) => {
                for (idx, job) in &group_jobs {
                    let _ = app.emit("whisper-batch-progress", BatchJobProgress {
                        project_id:    job.project_id.clone(),
                        step:          "error".to_string(),
                        job_index:     *idx,
                        total_jobs:    total,
                        words_count:   0,
                        fillers_found: 0,
                        repeats_found: 0,
                        error:         Some(format!("Chargement modèle impossible: {}", e)),
                    });
                }
                continue;
            }
        };

        for (idx, job) in group_jobs {
            run_single_job_with_ctx(&ctx, job, idx, total, &app);
        }

        println!("[TrimLab Batch] Modèle libéré : {}", model_path);
    }
}

fn run_single_job_with_ctx(
    ctx: &WhisperContext,
    job: BatchJobConfig,
    idx: usize,
    total: usize,
    app: &AppHandle,
) {
    let pid = job.project_id.clone();

    let emit = |step: &str, words: usize, fillers: usize, repeats: usize, err: Option<String>| {
        let _ = app.emit("whisper-batch-progress", BatchJobProgress {
            project_id:    pid.clone(),
            step:          step.to_string(),
            job_index:     idx,
            total_jobs:    total,
            words_count:   words,
            fillers_found: fillers,
            repeats_found: repeats,
            error:         err,
        });
    };

    let conn = match db::open() {
        Ok(c) => c,
        Err(e) => { emit("error", 0, 0, 0, Some(e.to_string())); return; }
    };

    // [FIX BUG 12] Nettoyer anciens mots/segments avant de retranscrire
    let _ = conn.execute("DELETE FROM transcript_words WHERE project_id = ?1", rusqlite::params![&job.project_id]);
    let _ = conn.execute("DELETE FROM segments WHERE project_id = ?1 AND seg_type IN ('filler','repeat')", rusqlite::params![&job.project_id]);

    let source_path: String = match conn.query_row(
        "SELECT path FROM media_files WHERE project_id=?1 ORDER BY added_at ASC LIMIT 1",
        rusqlite::params![&job.project_id],
        |row| row.get(0),
    ) {
        Ok(p) => p,
        Err(_) => { emit("error", 0, 0, 0, Some("Fichier source introuvable".into())); return; }
    };

    emit("extracting", 0, 0, 0, None);
    let wav_path = match extract_audio_wav(&source_path) {
        Ok(p) => p,
        Err(e) => { emit("error", 0, 0, 0, Some(e)); return; }
    };

    emit("transcribing", 0, 0, 0, None);
    let (transcript, language) = match run_whisper_with_ctx(ctx, &wav_path, &job.language) {
        Ok(r) => r,
        Err(e) => {
            let _ = std::fs::remove_file(&wav_path);
            emit("error", 0, 0, 0, Some(e));
            return;
        }
    };
    let _ = std::fs::remove_file(&wav_path);

    let mut fillers_found = 0usize;
    let mut repeats_found = 0usize;

    if job.detect_repeats {
        let repeats = detect_repetitions(&transcript, job.repeat_window, job.repeat_threshold);
        repeats_found = repeats.len();
        if let Err(e) = save_repeat_segments(&job.project_id, &repeats, &conn) {
            emit("error", 0, 0, 0, Some(e)); return;
        }
    }

    let cleaned     = clean_hallucinations(transcript);
    let words_count = cleaned.iter().map(|s| s.words.len()).sum::<usize>();

    if let Err(e) = save_transcript(&job.project_id, &cleaned, &conn) {
        emit("error", words_count, 0, 0, Some(e)); return;
    }

    emit("detecting", words_count, 0, 0, None);

    if job.detect_fillers {
        // [FIX BUG 8] Utiliser la détection multi-mots
        let fillers = detect_fillers_multiword(&cleaned, &language);
        fillers_found = fillers.len();
        if let Err(e) = save_filler_segments(&job.project_id, &fillers, &conn) {
            emit("error", words_count, 0, 0, Some(e)); return;
        }
    }

    emit("done", words_count, fillers_found, repeats_found, None);
}

// ─── Inférence Whisper ─────────────────────────────────────────

fn run_whisper(
    wav_path: &str,
    model_path: &str,
    language: &str,
) -> Result<(Vec<TranscriptSegment>, String), String> {
    let ctx = WhisperContext::new_with_params(model_path, WhisperContextParameters::default())
        .map_err(|e| format!("Erreur chargement modèle: {}", e))?;
    run_whisper_with_ctx(&ctx, wav_path, language)
}

fn run_whisper_with_ctx(
    ctx: &WhisperContext,
    wav_path: &str,
    language: &str,
) -> Result<(Vec<TranscriptSegment>, String), String> {
    let mut state = ctx.create_state()
        .map_err(|e| format!("Erreur état Whisper: {}", e))?;

    let audio = load_wav_mono_16k(wav_path)?;

    // [FIX BUG 11] Threads dynamiques selon les CPUs disponibles
    let n_threads = (std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4) as i32)
        .min(8); // max 8 threads pour Whisper

    let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });
    if language != "auto" {
        params.set_language(Some(language));
    }
    params.set_n_threads(n_threads);
    params.set_token_timestamps(true);
    params.set_suppress_blank(true);
    params.set_suppress_non_speech_tokens(true);
    // [FIX BUG 11] split_on_word=true pour de meilleurs timecodes par mot
    params.set_split_on_word(true);

    state.full(params, &audio)
        .map_err(|e| format!("Erreur transcription: {}", e))?;

    // [FIX BUG 7] Récupérer la langue détectée depuis Whisper
    let detected_lang = if language == "auto" {
        state.full_lang_id_from_state()
            .ok()
            .and_then(|id| whisper_rs::get_lang_str(id).map(|s| s.to_string()))
            .unwrap_or_else(|| "en".to_string())
    } else {
        language.to_string()
    };

    let num_segments = state.full_n_segments().map_err(|e| e.to_string())?;
    let mut segments: Vec<TranscriptSegment> = Vec::new();
    let mut prev_word_lower = String::new();

    for i in 0..num_segments {
        let seg_start = state.full_get_segment_t0(i).map_err(|e| e.to_string())? as f64 / 100.0;
        let seg_end   = state.full_get_segment_t1(i).map_err(|e| e.to_string())? as f64 / 100.0;
        let num_tokens = state.full_n_tokens(i).map_err(|e| e.to_string())?;

        struct Wip { text: String, start: f64, end: f64, prob: f64, count: u32 }

        let mut wip: Option<Wip> = None;
        let mut current_words: Vec<Word> = Vec::new();

        let push_wip = |wip: &mut Option<Wip>, words: &mut Vec<Word>, prev: &mut String| {
            if let Some(w) = wip.take() {
                let clean = w.text.trim().to_string();
                if clean.is_empty() { return; }
                let lower = clean.to_lowercase();
                if lower == *prev { return; }
                let avg_prob = if w.count > 0 { w.prob / w.count as f64 } else { 0.0 };
                words.push(Word { word: clean, start: w.start, end: w.end, confidence: avg_prob });
                *prev = lower;
            }
        };

        for j in 0..num_tokens {
            let token_text = state.full_get_token_text(i, j).map_err(|e| e.to_string())?;
            let token_data = state.full_get_token_data(i, j).map_err(|e| e.to_string())?;
            if is_special_token(&token_text) { continue; }

            let t_start = token_data.t0 as f64 / 100.0;
            let t_end   = token_data.t1 as f64 / 100.0;
            let t_prob  = token_data.p as f64;

            if token_text.starts_with(' ') || wip.is_none() {
                push_wip(&mut wip, &mut current_words, &mut prev_word_lower);
                wip = Some(Wip { text: token_text.trim().to_string(), start: t_start, end: t_end, prob: t_prob, count: 1 });
            } else {
                if let Some(ref mut w) = wip {
                    w.text.push_str(token_text.trim());
                    w.end = t_end; w.prob += t_prob; w.count += 1;
                }
            }
        }
        push_wip(&mut wip, &mut current_words, &mut prev_word_lower);
        if current_words.is_empty() { continue; }

        let final_text = current_words.iter().map(|w| w.word.as_str()).collect::<Vec<_>>().join(" ");
        segments.push(TranscriptSegment { start: seg_start, end: seg_end, text: final_text, words: current_words });
    }

    Ok((segments, detected_lang))
}

// ─── Helpers ───────────────────────────────────────────────────

fn is_special_token(text: &str) -> bool {
    let t = text.trim();
    if t.starts_with("[_") && t.ends_with(']') { return true; }
    if t.starts_with("<|") && t.ends_with("|>") { return true; }
    if t.is_empty() || t == "..." { return true; }
    false
}


// ── Résolution du binaire FFmpeg ─────────────────────────────────────────────
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

fn extract_audio_wav(source: &str) -> Result<String, String> {
    let wav_path = std::env::temp_dir()
        .join(format!("voicecut_audio_{}.wav", Uuid::new_v4()))
        .to_string_lossy().to_string();

    let out = std::process::Command::new(ffmpeg_bin())
        .args(["-y", "-i", source, "-ar", "16000", "-ac", "1", "-c:a", "pcm_s16le"])
        .arg(&wav_path)
        .output()
        .map_err(|e| format!("ffmpeg introuvable: {}", e))?;

    if !out.status.success() {
        return Err(String::from_utf8_lossy(&out.stderr).to_string());
    }
    Ok(wav_path)
}

fn load_wav_mono_16k(path: &str) -> Result<Vec<f32>, String> {
    let mut reader = hound::WavReader::open(path)
        .map_err(|e| format!("Lecture wav impossible: {}", e))?;
    let spec = reader.spec();
    if spec.channels != 1 || spec.sample_rate != 16000 {
        return Err("Le WAV doit être mono 16kHz".into());
    }
    let samples: Vec<f32> = reader.samples::<i16>()
        .filter_map(|s| s.ok())
        .map(|s| s as f32 / i16::MAX as f32)
        .collect();
    Ok(samples)
}

fn clean_hallucinations(segs: Vec<TranscriptSegment>) -> Vec<TranscriptSegment> {
    let mut cleaned: Vec<TranscriptSegment> = Vec::new();
    let mut prev_text  = String::new();
    let mut prev_start = 0.0_f64;
    for seg in segs {
        if seg.text.trim().is_empty() { continue; }
        let similarity = text_similarity(&seg.text, &prev_text);
        let time_gap   = seg.start - prev_start;
        if similarity > 0.95 && time_gap < 0.3 { continue; }
        prev_text  = seg.text.clone();
        prev_start = seg.start;
        cleaned.push(seg);
    }
    cleaned
}

// [FIX BUG 8] Détection fillers multi-mots (bigrammes + trigrammes)
fn detect_fillers_multiword(transcript: &[TranscriptSegment], language: &str) -> Vec<(f64, f64, String)> {
    // Mots simples
    let single: Vec<&str> = match language {
        "fr" => vec!["euh","eh","hum","bah","ben","voilà","donc","alors","hein","ouais","nan","quoi","genre"],
        "en" => vec!["uh","um","er","ah","like","so","basically","literally","right"],
        _    => vec!["uh","um","euh"],
    };
    // Expressions multi-mots
    let multi: Vec<&str> = match language {
        "fr" => vec!["du coup","en fait","c'est-à-dire","je veux dire","tu vois","vous voyez","en gros"],
        "en" => vec!["you know","i mean","kind of","sort of","you see","i guess"],
        _    => vec![],
    };

    let mut found: Vec<(f64, f64, String)> = Vec::new();

    for seg in transcript {
        let words = &seg.words;
        let n = words.len();
        let mut i = 0;
        while i < n {
            // Essayer trigramme d'abord (3 mots)
            if i + 2 < n {
                let tri = format!("{} {} {}", words[i].word.to_lowercase(), words[i+1].word.to_lowercase(), words[i+2].word.to_lowercase());
                if multi.contains(&tri.as_str()) {
                    let label = format!("{} {} {}", words[i].word, words[i+1].word, words[i+2].word);
                    found.push((words[i].start, words[i+2].end, label));
                    i += 3;
                    continue;
                }
            }
            // Essayer bigramme (2 mots)
            if i + 1 < n {
                let bi = format!("{} {}", words[i].word.to_lowercase(), words[i+1].word.to_lowercase());
                if multi.contains(&bi.as_str()) {
                    let label = format!("{} {}", words[i].word, words[i+1].word);
                    found.push((words[i].start, words[i+1].end, label));
                    i += 2;
                    continue;
                }
            }
            // Mot simple
            let w_lower = words[i].word.to_lowercase();
            let w_clean = w_lower.trim_matches(|c: char| !c.is_alphabetic());
            if single.contains(&w_clean) {
                found.push((words[i].start, words[i].end, words[i].word.clone()));
            }
            i += 1;
        }
    }
    found
}

fn detect_repetitions(transcript: &[TranscriptSegment], window_secs: f64, threshold: f64) -> Vec<(f64, f64, String)> {
    let mut found = Vec::new();
    for (i, seg) in transcript.iter().enumerate() {
        for prev in transcript[..i].iter().rev() {
            // [FIX BUG 10] Comparer sur seg.start - prev.start (pas prev.end)
            // pour éviter le break prématuré sur segments chevauchants
            if seg.start - prev.start > window_secs { break; }
            if text_similarity(&seg.text, &prev.text) >= threshold {
                found.push((seg.start, seg.end, seg.text.clone()));
                break;
            }
        }
    }
    found
}

fn text_similarity(a: &str, b: &str) -> f64 {
    let wa: std::collections::HashSet<&str> = a.split_whitespace().collect();
    let wb: std::collections::HashSet<&str> = b.split_whitespace().collect();
    let inter = wa.intersection(&wb).count();
    let union = wa.union(&wb).count();
    if union == 0 { 0.0 } else { inter as f64 / union as f64 }
}

fn save_transcript(project_id: &str, transcript: &[TranscriptSegment], conn: &rusqlite::Connection) -> Result<(), String> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS transcript_words(\
            id TEXT PRIMARY KEY, project_id TEXT, word TEXT, \
            start_time REAL, end_time REAL, confidence REAL);"
    ).map_err(|e| e.to_string())?;
    // [FIX BUG 9] Déjà nettoyé avant l'appel, on insère directement
    for seg in transcript {
        for word in &seg.words {
            conn.execute(
                "INSERT INTO transcript_words (id, project_id, word, start_time, end_time, confidence) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                rusqlite::params![Uuid::new_v4().to_string(), project_id, word.word, word.start, word.end, word.confidence],
            ).map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

fn save_filler_segments(project_id: &str, fillers: &[(f64, f64, String)], conn: &rusqlite::Connection) -> Result<(), String> {
    for (start, end, label) in fillers {
        conn.execute(
            "INSERT INTO segments (id, project_id, start_time, end_time, seg_type, confidence, label) \
             VALUES (?1, ?2, ?3, ?4, 'filler', 0.9, ?5)",
            rusqlite::params![Uuid::new_v4().to_string(), project_id, start, end, label],
        ).map_err(|e| e.to_string())?;
    }
    Ok(())
}

fn save_repeat_segments(project_id: &str, repeats: &[(f64, f64, String)], conn: &rusqlite::Connection) -> Result<(), String> {
    for (start, end, label) in repeats {
        conn.execute(
            "INSERT INTO segments (id, project_id, start_time, end_time, seg_type, confidence, label) \
             VALUES (?1, ?2, ?3, ?4, 'repeat', 0.85, ?5)",
            rusqlite::params![Uuid::new_v4().to_string(), project_id, start, end, label],
        ).map_err(|e| e.to_string())?;
    }
    Ok(())
}

// ── Sauvegarde des blocs de sous-titres édités ────────────────────────────────

#[tauri::command]
pub fn save_subtitle_blocks(project_id: String, blocks_json: String) -> Result<(), String> {
    let conn = db::open().map_err(|e: rusqlite::Error| e.to_string())?;
    conn.execute(
        "UPDATE projects SET subtitle_blocks = ?1 WHERE id = ?2",
        rusqlite::params![blocks_json, project_id],
    ).map_err(|e: rusqlite::Error| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn get_subtitle_blocks(project_id: String) -> Result<Option<String>, String> {
    let conn = db::open().map_err(|e: rusqlite::Error| e.to_string())?;
    let result: rusqlite::Result<Option<String>> = conn.query_row(
        "SELECT subtitle_blocks FROM projects WHERE id = ?1",
        rusqlite::params![project_id],
        |r: &rusqlite::Row| r.get(0),
    );
    result.map_err(|e: rusqlite::Error| e.to_string())
}