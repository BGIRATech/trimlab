// ============================================================
// FICHIER : trimlab/src-tauri/src/commands/projects.rs
// ROLE    : Commandes Tauri CRUD projets -> SQLite
// APPELE  : depuis le frontend via commands.ts
// ============================================================

use crate::db;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Project {
    pub id: String,
    pub name: String,
    pub status: String,
    pub progress: i64,
    pub export_format: String,
    pub detection_mode: String,
    pub created_at: String,
    pub updated_at: String,
    pub settings: ProjectSettings,
    pub files: Vec<MediaFile>,
    pub stats: Option<ProcessingStats>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ProjectSettings {
    pub silence_threshold: f64,
    pub silence_min_duration: f64,
    pub filler_words: Vec<String>,
    pub padding_before: f64,
    pub padding_after: f64,
    pub ai_mode: String,
    pub language: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MediaFile {
    pub id: String,
    pub project_id: String,
    pub name: String,
    pub path: String,
    pub duration: f64,
    pub size: i64,
    pub media_type: String,
    pub has_video: bool,
    pub has_audio: bool,
    pub fps: Option<f64>,
    pub codec: Option<String>,
    pub sample_rate: Option<i64>,
    pub added_at: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ProcessingStats {
    pub original_duration: f64,
    pub trimmed_duration: f64,
    pub silences_removed: i64,
    pub fillers_removed: i64,
    pub space_saved: i64,
    pub processing_time: i64,
    pub accuracy: f64,
}

// ─── LIST ────────────────────────────────────────────────────────────────────

#[tauri::command]
pub fn list_projects() -> Result<Vec<Project>, String> {
    let conn = db::open().map_err(|e| e.to_string())?;
    let mut stmt = conn.prepare(
        "SELECT id, name, status, progress, export_format, detection_mode, created_at, updated_at
         FROM projects ORDER BY updated_at DESC"
    ).map_err(|e| e.to_string())?;

    let ids: Vec<Project> = stmt.query_map([], |row| {
        Ok(Project {
            id:               row.get(0)?,
            name:             row.get(1)?,
            status:           row.get(2)?,
            progress:         row.get(3)?,
            export_format:    row.get(4)?,
            detection_mode:   row.get(5)?,
            created_at:       row.get(6)?,
            updated_at:       row.get(7)?,
            settings:         ProjectSettings::default(),
            files:            vec![],
            stats:            None,
        })
    })
    .map_err(|e| e.to_string())?
    .filter_map(|r| r.ok())
    .collect();

    // Charger settings + files + stats pour chaque projet
    let mut projects = Vec::new();
    for mut p in ids {
        p.settings = load_settings(&conn, &p.id)?;
        p.files    = load_files(&conn, &p.id)?;
        p.stats    = load_stats(&conn, &p.id)?;
        projects.push(p);
    }
    Ok(projects)
}

// ─── GET ─────────────────────────────────────────────────────────────────────

#[tauri::command]
pub fn get_project(id: String) -> Result<Project, String> {
    let conn = db::open().map_err(|e| e.to_string())?;
    let mut p: Project = conn.query_row(
        "SELECT id, name, status, progress, export_format, detection_mode, created_at, updated_at
         FROM projects WHERE id = ?1",
        [&id],
        |row| Ok(Project {
            id:             row.get(0)?,
            name:           row.get(1)?,
            status:         row.get(2)?,
            progress:       row.get(3)?,
            export_format:  row.get(4)?,
            detection_mode: row.get(5)?,
            created_at:     row.get(6)?,
            updated_at:     row.get(7)?,
            settings:       ProjectSettings::default(),
            files:          vec![],
            stats:          None,
        }),
    ).map_err(|e| format!("Projet introuvable: {}", e))?;

    p.settings = load_settings(&conn, &p.id)?;
    p.files    = load_files(&conn, &p.id)?;
    p.stats    = load_stats(&conn, &p.id)?;
    Ok(p)
}

// ─── CREATE ──────────────────────────────────────────────────────────────────

#[tauri::command]
pub fn create_project(name: String) -> Result<Project, String> {
    let conn = db::open().map_err(|e| e.to_string())?;
    let id  = Uuid::new_v4().to_string();
    let now = Utc::now().to_rfc3339();

    conn.execute(
        "INSERT INTO projects (id, name, status, progress, export_format, detection_mode, created_at, updated_at)
         VALUES (?1, ?2, 'idle', 0, 'fcpxml', 'both', ?3, ?4)",
        rusqlite::params![id, name, now, now],
    ).map_err(|e| e.to_string())?;

    // Settings par defaut
    conn.execute(
        "INSERT INTO project_settings (project_id) VALUES (?1)",
        [&id],
    ).map_err(|e| e.to_string())?;

    get_project(id)
}

// ─── UPDATE ──────────────────────────────────────────────────────────────────

#[tauri::command]
pub fn update_project_status(id: String, status: String, progress: i64) -> Result<(), String> {
    let conn = db::open().map_err(|e| e.to_string())?;
    let now  = Utc::now().to_rfc3339();
    conn.execute(
        "UPDATE projects SET status=?1, progress=?2, updated_at=?3 WHERE id=?4",
        rusqlite::params![status, progress, now, id],
    ).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn update_project_settings(id: String, settings: ProjectSettings) -> Result<(), String> {
    let conn = db::open().map_err(|e| e.to_string())?;
    let filler_words = settings.filler_words.join(",");
    conn.execute(
        "INSERT INTO project_settings
            (project_id, silence_threshold, silence_min_duration, filler_words, padding_before, padding_after, ai_mode, language)
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8)
         ON CONFLICT(project_id) DO UPDATE SET
            silence_threshold=?2, silence_min_duration=?3, filler_words=?4,
            padding_before=?5, padding_after=?6, ai_mode=?7, language=?8",
        rusqlite::params![
            id, settings.silence_threshold, settings.silence_min_duration,
            filler_words, settings.padding_before, settings.padding_after,
            settings.ai_mode, settings.language
        ],
    ).map_err(|e| e.to_string())?;

    // updated_at
    let now = Utc::now().to_rfc3339();
    conn.execute("UPDATE projects SET updated_at=?1 WHERE id=?2", rusqlite::params![now, id])
        .map_err(|e| e.to_string())?;
    Ok(())
}

// ─── DELETE ──────────────────────────────────────────────────────────────────

#[tauri::command]
pub fn delete_project(id: String) -> Result<(), String> {
    let conn = db::open().map_err(|e| e.to_string())?;
    // CASCADE supprime segments, files, settings, stats automatiquement
    conn.execute("DELETE FROM projects WHERE id=?1", [&id])
        .map_err(|e| e.to_string())?;
    Ok(())
}

// ─── MEDIA FILES ─────────────────────────────────────────────────────────────

// Struct d'input sans id ni added_at (generes cote Rust)
#[derive(Debug, Serialize, Deserialize)]
pub struct MediaFileInput {
    pub name:        String,
    pub path:        String,
    pub duration:    f64,
    pub size:        i64,
    pub media_type:  String,
    pub has_video:   bool,
    pub has_audio:   bool,
    pub fps:         Option<f64>,
    pub codec:       Option<String>,
    pub sample_rate: Option<i64>,
}

#[tauri::command]
pub fn add_media_file(project_id: String, file: MediaFileInput) -> Result<MediaFile, String> {
    let conn = db::open().map_err(|e| e.to_string())?;
    let id   = Uuid::new_v4().to_string();
    let now  = Utc::now().to_rfc3339();

    conn.execute(
        "INSERT INTO media_files
            (id, project_id, name, path, duration, size, media_type, has_video, has_audio, fps, codec, sample_rate, added_at)
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13)",
        rusqlite::params![
            id, project_id, file.name, file.path, file.duration, file.size,
            file.media_type, file.has_video as i64, file.has_audio as i64,
            file.fps, file.codec, file.sample_rate, now
        ],
    ).map_err(|e| e.to_string())?;

    let now2 = Utc::now().to_rfc3339();
    conn.execute("UPDATE projects SET updated_at=?1 WHERE id=?2",
        rusqlite::params![now2, project_id]).map_err(|e| e.to_string())?;

    Ok(MediaFile {
        id,
        project_id,
        name:        file.name,
        path:        file.path,
        duration:    file.duration,
        size:        file.size,
        media_type:  file.media_type,
        has_video:   file.has_video,
        has_audio:   file.has_audio,
        fps:         file.fps,
        codec:       file.codec,
        sample_rate: file.sample_rate,
        added_at:    now,
    })
}

// ─── STATS ───────────────────────────────────────────────────────────────────

#[tauri::command]
pub fn save_processing_stats(project_id: String, stats: ProcessingStats) -> Result<(), String> {
    let conn = db::open().map_err(|e| e.to_string())?;
    conn.execute(
        "INSERT INTO processing_stats
            (project_id, original_duration, trimmed_duration, silences_removed, fillers_removed, space_saved, processing_time, accuracy)
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8)
         ON CONFLICT(project_id) DO UPDATE SET
            original_duration=?2, trimmed_duration=?3, silences_removed=?4,
            fillers_removed=?5, space_saved=?6, processing_time=?7, accuracy=?8",
        rusqlite::params![
            project_id, stats.original_duration, stats.trimmed_duration,
            stats.silences_removed, stats.fillers_removed,
            stats.space_saved, stats.processing_time, stats.accuracy
        ],
    ).map_err(|e| e.to_string())?;
    Ok(())
}

// ─── DASHBOARD STATS ─────────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct DashboardStats {
    pub total_projects: i64,
    pub total_files: i64,
    pub total_time_saved: f64,
    pub total_exports: i64,
    pub avg_accuracy: f64,
}

#[tauri::command]
pub fn get_dashboard_stats() -> Result<DashboardStats, String> {
    let conn = db::open().map_err(|e| e.to_string())?;

    let total_projects: i64 = conn.query_row(
        "SELECT COUNT(*) FROM projects", [], |r| r.get(0)
    ).unwrap_or(0);

    let total_files: i64 = conn.query_row(
        "SELECT COUNT(*) FROM media_files", [], |r| r.get(0)
    ).unwrap_or(0);

    let total_time_saved: f64 = conn.query_row(
        "SELECT COALESCE(SUM(original_duration - trimmed_duration), 0) FROM processing_stats",
        [], |r| r.get(0)
    ).unwrap_or(0.0);

    let total_exports: i64 = conn.query_row(
        "SELECT COUNT(*) FROM exports", [], |r| r.get(0)
    ).unwrap_or(0);

    let avg_accuracy: f64 = conn.query_row(
        "SELECT COALESCE(AVG(accuracy), 0) FROM processing_stats WHERE accuracy > 0",
        [], |r| r.get(0)
    ).unwrap_or(0.0);

    Ok(DashboardStats { total_projects, total_files, total_time_saved, total_exports, avg_accuracy })
}

// ─── Helpers internes ────────────────────────────────────────────────────────

fn load_settings(conn: &rusqlite::Connection, project_id: &str) -> Result<ProjectSettings, String> {
    conn.query_row(
        "SELECT silence_threshold, silence_min_duration, filler_words,
                padding_before, padding_after, ai_mode, language
         FROM project_settings WHERE project_id=?1",
        [project_id],
        |row| {
            let words: String = row.get(2)?;
            Ok(ProjectSettings {
                silence_threshold:    row.get(0)?,
                silence_min_duration: row.get(1)?,
                filler_words:         words.split(',').map(|s| s.to_string()).collect(),
                padding_before:       row.get(3)?,
                padding_after:        row.get(4)?,
                ai_mode:              row.get(5)?,
                language:             row.get(6)?,
            })
        }
    ).map_err(|e| e.to_string())
}

fn load_files(conn: &rusqlite::Connection, project_id: &str) -> Result<Vec<MediaFile>, String> {
    let mut stmt = conn.prepare(
        "SELECT id, project_id, name, path, duration, size, media_type,
                has_video, has_audio, fps, codec, sample_rate, added_at
         FROM media_files WHERE project_id=?1"
    ).map_err(|e| e.to_string())?;

    let files = stmt.query_map([project_id], |row| {
        Ok(MediaFile {
            id:          row.get(0)?,
            project_id:  row.get(1)?,
            name:        row.get(2)?,
            path:        row.get(3)?,
            duration:    row.get(4)?,
            size:        row.get(5)?,
            media_type:  row.get(6)?,
            has_video:   row.get::<_, i64>(7)? != 0,
            has_audio:   row.get::<_, i64>(8)? != 0,
            fps:         row.get(9)?,
            codec:       row.get(10)?,
            sample_rate: row.get(11)?,
            added_at:    row.get(12)?,
        })
    })
    .map_err(|e| e.to_string())?
    .filter_map(|r| r.ok())
    .collect();

    Ok(files)
}

fn load_stats(conn: &rusqlite::Connection, project_id: &str) -> Result<Option<ProcessingStats>, String> {
    let result = conn.query_row(
        "SELECT original_duration, trimmed_duration, silences_removed,
                fillers_removed, space_saved, processing_time, accuracy
         FROM processing_stats WHERE project_id=?1",
        [project_id],
        |row| Ok(ProcessingStats {
            original_duration: row.get(0)?,
            trimmed_duration:  row.get(1)?,
            silences_removed:  row.get(2)?,
            fillers_removed:   row.get(3)?,
            space_saved:       row.get(4)?,
            processing_time:   row.get(5)?,
            accuracy:          row.get(6)?,
        }),
    );
    match result {
        Ok(s)  => Ok(Some(s)),
        Err(_) => Ok(None),
    }
}

impl Default for ProjectSettings {
    fn default() -> Self {
        Self {
            silence_threshold:    -40.0,
            silence_min_duration: 300.0,
            filler_words:         vec!["euh".into(), "hm".into(), "donc".into(), "voila".into()],
            padding_before:       50.0,
            padding_after:        50.0,
            ai_mode:              "fast".into(),
            language:             "auto".into(),
        }
    }
}