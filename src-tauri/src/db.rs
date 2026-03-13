// ============================================================
// FICHIER : trimlab/src-tauri/src/db.rs
// ROLE    : Initialisation SQLite + schema + migrations
//           Appele au demarrage de l'app (main.rs -> db::init)
//           La base est stockee dans : AppData/trimlab/trimlab.db (Windows)
//                                      ~/.local/share/trimlab/trimlab.db (Linux)
//                                      ~/Library/Application Support/trimlab/trimlab.db (macOS)
// ============================================================

use rusqlite::{Connection, Result};
use std::path::PathBuf;

/// Retourne le chemin vers le fichier SQLite selon l'OS
pub fn db_path() -> PathBuf {
    let base = dirs_next();
    base.join("trimlab.db")
}

fn dirs_next() -> PathBuf {
    // Windows : C:\Users\<user>\AppData\Roaming\trimlab\
    // macOS   : ~/Library/Application Support/trimlab/
    // Linux   : ~/.local/share/trimlab/
    let dir = std::env::var("APPDATA")
        .map(|p| PathBuf::from(p).join("trimlab"))
        .or_else(|_| {
            std::env::var("HOME").map(|h| {
                #[cfg(target_os = "macos")]
                return PathBuf::from(h).join("Library/Application Support/trimlab");
                #[cfg(not(target_os = "macos"))]
                return PathBuf::from(h).join(".local/share/trimlab");
            })
        })
        .unwrap_or_else(|_| PathBuf::from("./trimlab-data"));

    std::fs::create_dir_all(&dir).ok();
    dir
}

/// Ouvre une connexion SQLite avec WAL mode pour de meilleures performances
pub fn open() -> Result<Connection> {
    let path = db_path();
    let conn = Connection::open(&path)?;
    // WAL = Write-Ahead Logging : lectures et ecritures simultanees
    conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;")?;
    Ok(conn)
}

/// Cree toutes les tables si elles n'existent pas (idempotent)
pub fn init() -> Result<()> {
    let conn = open()?;
    conn.execute_batch("
        -- ─── Projects ───────────────────────────────────────────────
        CREATE TABLE IF NOT EXISTS projects (
            id              TEXT PRIMARY KEY,
            name            TEXT NOT NULL,
            status          TEXT NOT NULL DEFAULT 'idle',
            progress        INTEGER NOT NULL DEFAULT 0,
            export_format   TEXT NOT NULL DEFAULT 'fcpxml',
            detection_mode  TEXT NOT NULL DEFAULT 'both',
            created_at      TEXT NOT NULL,
            updated_at      TEXT NOT NULL
        );

        -- ─── Project settings (1-1 avec project) ────────────────────
        CREATE TABLE IF NOT EXISTS project_settings (
            project_id          TEXT PRIMARY KEY REFERENCES projects(id) ON DELETE CASCADE,
            silence_threshold   REAL NOT NULL DEFAULT -40.0,
            silence_min_duration REAL NOT NULL DEFAULT 300.0,
            filler_words        TEXT NOT NULL DEFAULT 'euh,hm,donc,voila,ben,genre',
            padding_before      REAL NOT NULL DEFAULT 50.0,
            padding_after       REAL NOT NULL DEFAULT 50.0,
            ai_mode             TEXT NOT NULL DEFAULT 'fast',
            language            TEXT NOT NULL DEFAULT 'auto'
        );

        -- ─── Media files ─────────────────────────────────────────────
        CREATE TABLE IF NOT EXISTS media_files (
            id                  TEXT PRIMARY KEY,
            project_id          TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
            name                TEXT NOT NULL,
            path                TEXT NOT NULL,
            duration            REAL NOT NULL DEFAULT 0,
            size                INTEGER NOT NULL DEFAULT 0,
            media_type          TEXT NOT NULL DEFAULT 'video',
            has_video           INTEGER NOT NULL DEFAULT 1,
            has_audio           INTEGER NOT NULL DEFAULT 1,
            external_audio_path TEXT,
            audio_offset        REAL,
            fps                 REAL,
            codec               TEXT,
            sample_rate         INTEGER,
            added_at            TEXT NOT NULL
        );

        -- ─── Segments ────────────────────────────────────────────────
        CREATE TABLE IF NOT EXISTS segments (
            id          TEXT PRIMARY KEY,
            project_id  TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
            start_time  REAL NOT NULL,
            end_time    REAL NOT NULL,
            seg_type    TEXT NOT NULL DEFAULT 'keep',
            confidence  REAL NOT NULL DEFAULT 1.0,
            label       TEXT
        );

        -- ─── Processing stats (1-1 avec project) ────────────────────
        CREATE TABLE IF NOT EXISTS processing_stats (
            project_id          TEXT PRIMARY KEY REFERENCES projects(id) ON DELETE CASCADE,
            original_duration   REAL NOT NULL DEFAULT 0,
            trimmed_duration    REAL NOT NULL DEFAULT 0,
            silences_removed    INTEGER NOT NULL DEFAULT 0,
            fillers_removed     INTEGER NOT NULL DEFAULT 0,
            space_saved         INTEGER NOT NULL DEFAULT 0,
            processing_time     INTEGER NOT NULL DEFAULT 0,
            accuracy            REAL NOT NULL DEFAULT 0
        );

        -- ─── Licence ─────────────────────────────────────────────────
        CREATE TABLE IF NOT EXISTS licence (
            id              INTEGER PRIMARY KEY CHECK (id = 1),
            status          TEXT NOT NULL DEFAULT 'free',
            key_hash        TEXT,
            email           TEXT,
            activated_at    TEXT,
            expires_at      TEXT,
            machine_id      TEXT,
            activations     INTEGER NOT NULL DEFAULT 0,
            max_activations INTEGER NOT NULL DEFAULT 2
        );

        -- Insere la ligne licence si elle n'existe pas
        INSERT OR IGNORE INTO licence (id, status) VALUES (1, 'free');

        -- ─── Exports log ─────────────────────────────────────────────
        CREATE TABLE IF NOT EXISTS exports (
            id          TEXT PRIMARY KEY,
            project_id  TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
            format      TEXT NOT NULL,
            output_path TEXT NOT NULL,
            exported_at TEXT NOT NULL,
            success     INTEGER NOT NULL DEFAULT 1
        );

        -- ─── Transcript words (Whisper) ──────────────────────────────
        CREATE TABLE IF NOT EXISTS transcript_words (
            id          TEXT PRIMARY KEY,
            project_id  TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
            word        TEXT NOT NULL,
            start_time  REAL NOT NULL,
            end_time    REAL NOT NULL,
            confidence  REAL NOT NULL DEFAULT 1.0
        );

        -- Index pour les requetes frequentes
        CREATE INDEX IF NOT EXISTS idx_segments_project    ON segments(project_id);
        CREATE INDEX IF NOT EXISTS idx_media_project       ON media_files(project_id);
        CREATE INDEX IF NOT EXISTS idx_exports_project     ON exports(project_id);
        CREATE INDEX IF NOT EXISTS idx_transcript_project  ON transcript_words(project_id);
    ")?;

    // Migration : ajouter les colonnes audio si elles n'existent pas
    // (pour les bases existantes qui n'ont pas ces colonnes)
    let _ = conn.execute_batch("
        ALTER TABLE media_files ADD COLUMN external_audio_path TEXT;
        ALTER TABLE media_files ADD COLUMN audio_offset REAL;
    ");
    // Le _ ignore l'erreur si les colonnes existent déjà


    // Migration : ajouter subtitle_blocks si absent
    let _ = conn.execute_batch("
        ALTER TABLE projects ADD COLUMN subtitle_blocks TEXT;
    ");
    println!("[TrimLab DB] Initialise : {:?}", db_path());
    Ok(())
}