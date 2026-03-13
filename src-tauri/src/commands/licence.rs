// ============================================================
// FICHIER : src-tauri/src/commands/licence.rs
// ROLE    : Validation HMAC-SHA256 + persistance licence SQLite
// FORMAT  : TRIMLAB-XXXX-XXXX-XXXX
// ============================================================
//
// Ajouter dans src-tauri/Cargo.toml [dependencies] :
//   hmac = "0.12"
//   sha2 = "0.10"
// ============================================================

use crate::db;
// Ajouter dans Cargo.toml : reqwest = { version = "0.12", features = ["blocking", "json"] }
use chrono::Utc;
use hmac::{Hmac, Mac};
use sha2::Sha256;
use serde::{Deserialize, Serialize};

// ⚠️  MÊME VALEUR que dans generate_key.py et le Worker Cloudflare
//     À remplacer par votre secret réel avant la release
const LICENCE_SECRET: &str = "02525994ab93fda781accaff35a1de3dddcc9fa77704b8a19813e70fab303427";

type HmacSha256 = Hmac<Sha256>;

// ─── Alphabet Base32 RFC 4648 ────────────────────────────────────────────────

const BASE32: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ234567";

fn base32_encode(data: &[u8]) -> String {
    let mut output = String::new();
    let mut bits: u32 = 0;
    let mut value: u32 = 0;
    for &byte in data {
        value = (value << 8) | byte as u32;
        bits += 8;
        while bits >= 5 {
            output.push(BASE32[((value >> (bits - 5)) & 31) as usize] as char);
            bits -= 5;
        }
    }
    if bits > 0 {
        output.push(BASE32[((value << (5 - bits)) & 31) as usize] as char);
    }
    output
}

// ─── Validation HMAC ─────────────────────────────────────────────────────────
// Format attendu : TRIMLAB-NONCE-SIG1-SIG2
// NONCE = 4 chars base32 aléatoires
// SIG1+SIG2 = 8 premiers chars de HMAC-SHA256(SECRET, NONCE) en base32

fn validate_hmac_key(key: &str) -> bool {
    let key = key.trim().to_uppercase();
    let parts: Vec<&str> = key.split('-').collect();

    if parts.len() != 4 || parts[0] != "TRIMLAB" {
        return false;
    }

    let nonce = parts[1];
    let sig_provided = format!("{}{}", parts[2], parts[3]);

    if nonce.len() != 4 || sig_provided.len() != 8 {
        return false;
    }

    let mut mac: HmacSha256 = match HmacSha256::new_from_slice(LICENCE_SECRET.as_bytes()) {
        Ok(m) => m,
        Err(_) => return false,
    };
    mac.update(nonce.as_bytes());
    let result = mac.finalize().into_bytes();
    let sig_expected = base32_encode(&result);
    let sig_expected_8 = &sig_expected[..8.min(sig_expected.len())];

    sig_provided == sig_expected_8
}

// ─── Structs ─────────────────────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize)]
pub struct LicenceInfo {
    pub status: String,
    pub email: Option<String>,
    pub activated_at: Option<String>,
    pub expires_at: Option<String>,
    pub machine_id: Option<String>,
    pub activations: i64,
    pub max_activations: i64,
}

#[derive(Debug, Serialize)]
pub struct LicenceValidation {
    pub valid: bool,
    pub plan: Option<String>,
    pub email: Option<String>,
    pub error: Option<String>,
}

// ─── READ ────────────────────────────────────────────────────────────────────

#[tauri::command]
pub fn get_licence() -> Result<LicenceInfo, String> {
    let conn = db::open().map_err(|e| e.to_string())?;
    conn.query_row(
        "SELECT status, email, activated_at, expires_at, machine_id, activations, max_activations
         FROM licence WHERE id=1",
        [],
        |row| Ok(LicenceInfo {
            status:          row.get(0)?,
            email:           row.get(1)?,
            activated_at:    row.get(2)?,
            expires_at:      row.get(3)?,
            machine_id:      row.get(4)?,
            activations:     row.get(5)?,
            max_activations: row.get(6)?,
        }),
    ).map_err(|e| e.to_string())
}

// ─── VALIDATE + ACTIVATE ─────────────────────────────────────────────────────
// L'app appelle le Worker Cloudflare /activate avec la clé + machine_id.
// Le Worker vérifie dans KV que la clé est valide et pas encore activée
// sur une autre machine. Résultat stocké en SQLite localement.

const WORKER_URL: &str = "https://trimlab-licence.brunogirat.workers.dev/activate";
const WORKER_VERIFY_URL: &str = "https://trimlab-licence.brunogirat.workers.dev/verify";

#[tauri::command]
pub fn validate_and_activate_licence(key: String) -> Result<LicenceValidation, String> {
    let key_clean = key.trim().to_uppercase();

    // Vérification HMAC locale d'abord — filtre les clés invalides sans appel réseau
    if !validate_hmac_key(&key_clean) {
        return Ok(LicenceValidation {
            valid: false,
            plan:  None,
            email: None,
            error: Some("Clé invalide ou incorrecte.".into()),
        });
    }

    let machine = get_machine_id_str();

    // Appel Worker pour vérifier 1 machine max
    let body = serde_json::json!({
        "key": key_clean,
        "machine_id": machine,
    });

    let client = reqwest::blocking::Client::new();
    let res = client
        .post(WORKER_URL)
        .header("Content-Type", "application/json")
        .body(body.to_string())
        .timeout(std::time::Duration::from_secs(10))
        .send()
        .map_err(|e| format!("Erreur réseau : {e}"))?;

    let json: serde_json::Value = res.json()
        .map_err(|e| format!("Réponse invalide : {e}"))?;

    let valid = json["valid"].as_bool().unwrap_or(false);

    if !valid {
        let error = json["error"].as_str().unwrap_or("Clé refusée").to_string();
        return Ok(LicenceValidation {
            valid: false,
            plan:  None,
            email: None,
            error: Some(error),
        });
    }

    // Activer localement en SQLite (stocker la clé pour vérification hebdo)
    persist_licence_with_key("lifetime", &key_clean, &machine)?;

    Ok(LicenceValidation {
        valid: true,
        plan:  Some("lifetime".into()),
        email: None,
        error: None,
    })
}

// ─── VÉRIFICATION PÉRIODIQUE ─────────────────────────────────────────────────
// Appelée au démarrage de l'app — vérifie en ligne max 1x/semaine
// Si la clé a été révoquée (remboursement) → repasse en mode trial

#[tauri::command]
pub fn verify_licence_online() -> Result<bool, String> {
    let conn = db::open().map_err(|e| e.to_string())?;

    // Récupérer le statut actuel
    let (status, machine_id, last_check): (String, Option<String>, Option<String>) = conn
        .query_row(
            "SELECT status, machine_id, activated_at FROM licence WHERE id=1",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .map_err(|e| e.to_string())?;

    // Seulement vérifier si licence active
    if status != "lifetime" {
        return Ok(true);
    }

    // Vérifier max 1x par semaine — on stocke la date dans expires_at
    // (champ réutilisé comme "last_verified_at" puisqu'on n'a pas d'expiration)
    let last_verified: Option<String> = conn
        .query_row(
            "SELECT expires_at FROM licence WHERE id=1",
            [],
            |row| row.get(0),
        )
        .unwrap_or(None);

    let should_verify = match &last_verified {
        None => true,
        Some(date_str) => {
            if let Ok(last) = chrono::DateTime::parse_from_rfc3339(date_str) {
                let elapsed = chrono::Utc::now().signed_duration_since(last.with_timezone(&chrono::Utc));
                elapsed.num_days() >= 7
            } else {
                true
            }
        }
    };

    if !should_verify {
        return Ok(true);
    }

    // Récupérer la clé hashée — on stocke le key_hash pour la vérification
    let key_hash: Option<String> = conn
        .query_row(
            "SELECT key_hash FROM licence WHERE id=1",
            [],
            |row| row.get(0),
        )
        .unwrap_or(None);

    let key = match key_hash {
        Some(k) if !k.is_empty() => k,
        _ => return Ok(true), // Pas de clé stockée → on laisse passer
    };

    let machine = machine_id.unwrap_or_else(get_machine_id_str);

    let body = serde_json::json!({
        "key": key,
        "machine_id": machine,
    });

    let client = reqwest::blocking::Client::new();
    let res = client
        .post(WORKER_VERIFY_URL)
        .header("Content-Type", "application/json")
        .body(body.to_string())
        .timeout(std::time::Duration::from_secs(8))
        .send();

    match res {
        Err(_) => {
            // Pas de réseau → bénéfice du doute, on laisse actif
            Ok(true)
        }
        Ok(response) => {
            let json: serde_json::Value = response.json().unwrap_or_default();
            let valid = json["valid"].as_bool().unwrap_or(true);

            if !valid {
                // Clé révoquée → repasser en trial
                let conn2 = db::open().map_err(|e| e.to_string())?;
                conn2.execute(
                    "UPDATE licence SET status='trial', key_hash=NULL, activated_at=NULL,
                     expires_at=NULL, machine_id=NULL WHERE id=1",
                    [],
                ).map_err(|e| e.to_string())?;
                return Ok(false);
            }

            // Mettre à jour la date de dernière vérification
            let now = chrono::Utc::now().to_rfc3339();
            conn.execute(
                "UPDATE licence SET expires_at=?1 WHERE id=1",
                rusqlite::params![now],
            ).map_err(|e| e.to_string())?;

            Ok(true)
        }
    }
}

// ─── DEACTIVATE ──────────────────────────────────────────────────────────────

#[tauri::command]
pub fn deactivate_licence() -> Result<(), String> {
    let conn = db::open().map_err(|e| e.to_string())?;
    conn.execute(
        "UPDATE licence SET status='free', email=NULL, activated_at=NULL,
         expires_at=NULL, key_hash=NULL, activations=0 WHERE id=1",
        [],
    ).map_err(|e| e.to_string())?;
    Ok(())
}

// ─── MACHINE ID ──────────────────────────────────────────────────────────────

#[tauri::command]
pub fn get_machine_id() -> String {
    get_machine_id_str()
}

fn get_machine_id_str() -> String {
    #[cfg(target_os = "windows")]
    {
        let out = std::process::Command::new("wmic")
            .args(["csproduct", "get", "UUID"])
            .output()
            .ok();
        if let Some(o) = out {
            let s = String::from_utf8_lossy(&o.stdout);
            let uuid = s.lines()
                .skip(1)
                .find(|l| !l.trim().is_empty())
                .map(|l| l.trim().to_string())
                .unwrap_or_default();
            if !uuid.is_empty() {
                return format!("WIN-{}", &uuid[..8.min(uuid.len())]);
            }
        }
    }
    #[cfg(target_os = "macos")]
    {
        let out = std::process::Command::new("ioreg")
            .args(["-rd1", "-c", "IOPlatformExpertDevice"])
            .output()
            .ok();
        if let Some(o) = out {
            let s = String::from_utf8_lossy(&o.stdout);
            for line in s.lines() {
                if line.contains("IOPlatformUUID") {
                    if let Some(start) = line.rfind('"') {
                        let rest = &line[..start];
                        if let Some(end) = rest.rfind('"') {
                            return format!("MAC-{}", &rest[end+1..][..8.min(rest.len())]);
                        }
                    }
                }
            }
        }
    }
    #[cfg(target_os = "linux")]
    {
        if let Ok(id) = std::fs::read_to_string("/etc/machine-id") {
            let trimmed = id.trim();
            return format!("LNX-{}", &trimmed[..8.min(trimmed.len())]);
        }
    }
    "UNKNOWN-MACHINE".into()
}

// ─── Helper persistance ───────────────────────────────────────────────────────

fn persist_licence_with_key(
    plan: &str,
    key: &str,
    machine_id: &str,
) -> Result<(), String> {
    let conn = db::open().map_err(|e| e.to_string())?;
    let now = chrono::Utc::now().to_rfc3339();
    conn.execute(
        "UPDATE licence SET status=?1, key_hash=?2, activated_at=?3, expires_at=NULL,
         machine_id=?4, activations=activations+1 WHERE id=1",
        rusqlite::params![plan, key, now, machine_id],
    ).map_err(|e| e.to_string())?;
    Ok(())
}

fn persist_licence(
    plan: &str,
    email: Option<&str>,
    expires_at: Option<&str>,
    machine_id: &str,
) -> Result<(), String> {
    let conn = db::open().map_err(|e| e.to_string())?;
    let now = Utc::now().to_rfc3339();
    conn.execute(
        "UPDATE licence SET status=?1, email=?2, activated_at=?3, expires_at=?4,
         machine_id=?5, activations=activations+1 WHERE id=1",
        rusqlite::params![plan, email, now, expires_at, machine_id],
    ).map_err(|e| e.to_string())?;
    Ok(())
}