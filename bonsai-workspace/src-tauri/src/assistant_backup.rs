/// Backup / restore for Bonsai Assistant data.
///
/// ZIP format:
///   manifest.json      — version, created_at, app_version, sha256 of every entry
///   profiles/{id}.json
///   avatars/{id}.json
///   sessions/{id}.json — session header + all messages
///
/// Encryption: optional AES-256-GCM (PBKDF2 key derivation, 100k iterations).
/// Import modes: Merge | ReplaceProfile(id) | FullReplace
/// FullReplace auto-snapshots current state first (stored in backup_registry).

use std::collections::HashMap;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tauri::{AppHandle, Manager};
use zip::write::FileOptions;
use zip::{ZipArchive, ZipWriter};

use crate::assistant_store::{
    AssistantMessage, AssistantProfile, AssistantSession, AssistantStore, AvatarAsset,
};

// ── Public types ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "mode")]
pub enum ImportMode {
    Merge,
    ReplaceProfile { id: String },
    FullReplace,
}

#[derive(Debug, Serialize)]
pub struct ImportSummary {
    pub profiles: usize,
    pub avatars:  usize,
    pub sessions: usize,
    pub errors:   Vec<String>,
    pub rollback_snapshot: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct Manifest {
    version:     u32,
    created_at:  i64,
    app_version: String,
    encrypted:   bool,
    checksums:   HashMap<String, String>,  // entry path → sha256 hex
}

#[derive(Debug, Serialize, Deserialize)]
struct SessionExport {
    session:  AssistantSession,
    messages: Vec<AssistantMessage>,
}

// ── Encryption helpers ────────────────────────────────────────────────────────

const PBKDF2_ITERATIONS: u32 = 100_000;
const SALT_LEN:          usize = 16;
const NONCE_LEN:         usize = 12;
const KEY_LEN:           usize = 32;

fn derive_key(passphrase: &str, salt: &[u8]) -> [u8; KEY_LEN] {
    use sha2::Sha256 as HmacSha256;
    let mut key = [0u8; KEY_LEN];
    // Manual PBKDF2-HMAC-SHA256 using sha2 primitives (no pbkdf2 crate needed)
    pbkdf2_hmac_sha256(passphrase.as_bytes(), salt, PBKDF2_ITERATIONS, &mut key);
    key
}

fn pbkdf2_hmac_sha256(password: &[u8], salt: &[u8], iterations: u32, out: &mut [u8]) {
    // Single-block PBKDF2 (output ≤ 32 bytes, one PRF block is sufficient)
    use sha2::digest::Mac;
    use hmac::Hmac;
    type HmacSha256 = Hmac<Sha256>;

    let mut u = {
        let mut mac = HmacSha256::new_from_slice(password).expect("HMAC key");
        mac.update(salt);
        mac.update(&1u32.to_be_bytes()); // block index = 1
        mac.finalize().into_bytes()
    };
    let mut result = u.clone();

    for _ in 1..iterations {
        let mut mac = HmacSha256::new_from_slice(password).expect("HMAC key");
        mac.update(&u);
        u = mac.finalize().into_bytes();
        for (r, b) in result.iter_mut().zip(u.iter()) {
            *r ^= b;
        }
    }

    let n = out.len().min(result.len());
    out[..n].copy_from_slice(&result[..n]);
}

fn encrypt_bytes(data: &[u8], passphrase: &str) -> Result<Vec<u8>, String> {
    use aes_gcm::{Aes256Gcm, Key, KeyInit, Nonce};
    use aes_gcm::aead::Aead;

    let mut salt = [0u8; SALT_LEN];
    let mut nonce_bytes = [0u8; NONCE_LEN];
    // Use sha2 as a deterministic CSPRNG seed (acceptable for non-secret salt/nonce)
    let seed = {
        let mut h = Sha256::new();
        h.update(passphrase.as_bytes());
        h.update(&now_ms().to_le_bytes());
        h.finalize()
    };
    salt.copy_from_slice(&seed[..SALT_LEN]);
    nonce_bytes.copy_from_slice(&seed[SALT_LEN..SALT_LEN + NONCE_LEN]);

    let key_bytes = derive_key(passphrase, &salt);
    let key   = Key::<Aes256Gcm>::from_slice(&key_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);
    let cipher = Aes256Gcm::new(key);
    let ciphertext = cipher.encrypt(nonce, data).map_err(|e| format!("AES-GCM encrypt: {e}"))?;

    // Output: salt (16) | nonce (12) | ciphertext
    let mut out = Vec::with_capacity(SALT_LEN + NONCE_LEN + ciphertext.len());
    out.extend_from_slice(&salt);
    out.extend_from_slice(&nonce_bytes);
    out.extend_from_slice(&ciphertext);
    Ok(out)
}

fn decrypt_bytes(data: &[u8], passphrase: &str) -> Result<Vec<u8>, String> {
    use aes_gcm::{Aes256Gcm, Key, KeyInit, Nonce};
    use aes_gcm::aead::Aead;

    if data.len() < SALT_LEN + NONCE_LEN + 16 {
        return Err("Encrypted data too short".into());
    }
    let salt        = &data[..SALT_LEN];
    let nonce_bytes = &data[SALT_LEN..SALT_LEN + NONCE_LEN];
    let ciphertext  = &data[SALT_LEN + NONCE_LEN..];

    let key_bytes = derive_key(passphrase, salt);
    let key   = Key::<Aes256Gcm>::from_slice(&key_bytes);
    let nonce = Nonce::from_slice(nonce_bytes);
    let cipher = Aes256Gcm::new(key);
    cipher.decrypt(nonce, ciphertext).map_err(|_| "Decryption failed — wrong passphrase?".into())
}

// ── Export ────────────────────────────────────────────────────────────────────

pub async fn export_backup(
    app:              &AppHandle,
    store:            &AssistantStore,
    include_sessions: bool,
    include_avatars:  bool,
    encrypt:          bool,
    passphrase:       Option<&str>,
) -> Result<String, String> {
    let data_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let backups_dir = data_dir.join("backups");
    std::fs::create_dir_all(&backups_dir).map_err(|e| format!("create backups dir: {e}"))?;

    let ts = now_ms();
    let filename = format!("bonsai-buddy-backup-{ts}.zip");
    let zip_path = backups_dir.join(&filename);

    // Collect all data first
    let profiles = store.list_profiles().await?;
    let avatars  = if include_avatars { store.list_avatars().await? } else { vec![] };

    let mut session_exports: Vec<SessionExport> = vec![];
    if include_sessions {
        let sessions = store.list_sessions(None).await?;
        for s in sessions {
            let messages = store.load_messages(&s.id).await?;
            session_exports.push(SessionExport { session: s, messages });
        }
    }

    // Build in-memory zip
    let zip_bytes = build_zip(&profiles, &avatars, &session_exports, encrypt, passphrase)?;

    // Write to disk
    std::fs::write(&zip_path, &zip_bytes).map_err(|e| format!("write zip: {e}"))?;

    // Register in backup_registry
    let file_size = zip_bytes.len() as i64;
    let checksum  = hex_sha256(&zip_bytes);
    let mut includes = vec!["profiles".to_string()];
    if include_avatars  { includes.push("avatars".into()); }
    if include_sessions { includes.push("sessions".into()); }

    store.register_backup(
        &filename,
        &zip_path.to_string_lossy(),
        file_size,
        &serde_json::to_string(&includes).unwrap_or_default(),
        &checksum,
        encrypt,
    ).await?;

    // Rotate: keep last 5 auto-backups (oldest first)
    store.rotate_backups(5).await?;

    Ok(zip_path.to_string_lossy().into_owned())
}

fn build_zip(
    profiles:        &[AssistantProfile],
    avatars:         &[AvatarAsset],
    session_exports: &[SessionExport],
    encrypt:         bool,
    passphrase:      Option<&str>,
) -> Result<Vec<u8>, String> {
    let mut buf = Vec::new();
    let mut zip = ZipWriter::new(std::io::Cursor::new(&mut buf));
    let opts = FileOptions::default().compression_method(zip::CompressionMethod::Deflated);
    let mut checksums: HashMap<String, String> = HashMap::new();

    // Helper: write entry
    let mut write_entry = |zip: &mut ZipWriter<std::io::Cursor<&mut Vec<u8>>>,
                            path: &str,
                            data: &[u8]| -> Result<(), String> {
        checksums.insert(path.to_string(), hex_sha256(data));
        zip.start_file(path, opts).map_err(|e| format!("zip start_file {path}: {e}"))?;
        zip.write_all(data).map_err(|e| format!("zip write {path}: {e}"))?;
        Ok(())
    };

    // Profiles
    for p in profiles {
        let json = serde_json::to_vec_pretty(p).map_err(|e| e.to_string())?;
        write_entry(&mut zip, &format!("profiles/{}.json", p.id), &json)?;
    }

    // Avatars
    for a in avatars {
        let json = serde_json::to_vec_pretty(a).map_err(|e| e.to_string())?;
        write_entry(&mut zip, &format!("avatars/{}.json", a.id), &json)?;
    }

    // Sessions
    for sx in session_exports {
        let json = serde_json::to_vec_pretty(sx).map_err(|e| e.to_string())?;
        write_entry(&mut zip, &format!("sessions/{}.json", sx.session.id), &json)?;
    }

    // Manifest (checksums written before manifest itself)
    let manifest = Manifest {
        version:     1,
        created_at:  now_ms(),
        app_version: env!("CARGO_PKG_VERSION").to_string(),
        encrypted:   encrypt,
        checksums:   checksums.clone(),
    };
    let manifest_json = serde_json::to_vec_pretty(&manifest).map_err(|e| e.to_string())?;
    zip.start_file("manifest.json", opts).map_err(|e| format!("zip manifest: {e}"))?;
    zip.write_all(&manifest_json).map_err(|e| format!("zip write manifest: {e}"))?;

    zip.finish().map_err(|e| format!("zip finish: {e}"))?;
    drop(zip);

    if encrypt {
        let pass = passphrase.unwrap_or("");
        if pass.is_empty() {
            return Err("Passphrase required for encrypted backup".into());
        }
        encrypt_bytes(&buf, pass)
    } else {
        Ok(buf)
    }
}

// ── Import ────────────────────────────────────────────────────────────────────

pub async fn import_backup(
    app:        &AppHandle,
    store:      &AssistantStore,
    zip_path:   &str,
    mode:       ImportMode,
    passphrase: Option<&str>,
    dry_run:    bool,
) -> Result<ImportSummary, String> {
    let raw = std::fs::read(zip_path).map_err(|e| format!("read backup: {e}"))?;

    // Decrypt if needed
    let zip_bytes = if is_encrypted(&raw) {
        let pass = passphrase.unwrap_or("");
        if pass.is_empty() {
            return Err("Backup is encrypted — passphrase required".into());
        }
        decrypt_bytes(&raw, pass)?
    } else {
        raw
    };

    // Parse zip
    let cursor = std::io::Cursor::new(&zip_bytes);
    let mut archive = ZipArchive::new(cursor).map_err(|e| format!("open zip: {e}"))?;

    // Read manifest first
    let manifest: Manifest = {
        let mut f = archive.by_name("manifest.json").map_err(|_| "manifest.json not found in backup")?;
        let mut s = String::new();
        f.read_to_string(&mut s).map_err(|e| format!("read manifest: {e}"))?;
        serde_json::from_str(&s).map_err(|e| format!("parse manifest: {e}"))?
    };

    // Validate checksums (dry-run or real)
    let mut errors: Vec<String> = vec![];
    let mut profiles: Vec<AssistantProfile> = vec![];
    let mut avatars:  Vec<AvatarAsset>       = vec![];
    let mut sessions: Vec<SessionExport>     = vec![];

    for i in 0..archive.len() {
        let mut file = archive.by_index(i).map_err(|e| format!("zip entry {i}: {e}"))?;
        let name = file.name().to_string();
        if name == "manifest.json" { continue; }

        let mut data = Vec::new();
        file.read_to_end(&mut data).map_err(|e| format!("read {name}: {e}"))?;

        // Checksum validation
        let actual = hex_sha256(&data);
        if let Some(expected) = manifest.checksums.get(&name) {
            if &actual != expected {
                errors.push(format!("Checksum mismatch: {name}"));
                continue;
            }
        }

        // Parse
        if name.starts_with("profiles/") {
            match serde_json::from_slice::<AssistantProfile>(&data) {
                Ok(p)  => profiles.push(p),
                Err(e) => errors.push(format!("parse {name}: {e}")),
            }
        } else if name.starts_with("avatars/") {
            match serde_json::from_slice::<AvatarAsset>(&data) {
                Ok(a)  => avatars.push(a),
                Err(e) => errors.push(format!("parse {name}: {e}")),
            }
        } else if name.starts_with("sessions/") {
            match serde_json::from_slice::<SessionExport>(&data) {
                Ok(s)  => sessions.push(s),
                Err(e) => errors.push(format!("parse {name}: {e}")),
            }
        }
    }

    if !errors.is_empty() {
        return Ok(ImportSummary {
            profiles: 0, avatars: 0, sessions: 0,
            errors,
            rollback_snapshot: None,
        });
    }

    if dry_run {
        return Ok(ImportSummary {
            profiles: profiles.len(),
            avatars:  avatars.len(),
            sessions: sessions.len(),
            errors:   vec![],
            rollback_snapshot: None,
        });
    }

    // FullReplace: auto-snapshot current state first
    let rollback_snapshot = if mode == ImportMode::FullReplace {
        match export_backup(app, store, true, true, false, None).await {
            Ok(path) => Some(path),
            Err(e)   => { errors.push(format!("auto-snapshot failed: {e}")); None }
        }
    } else {
        None
    };

    // Apply
    apply_import(store, &profiles, &avatars, &sessions, &mode, &mut errors).await;

    Ok(ImportSummary {
        profiles: profiles.len(),
        avatars:  avatars.len(),
        sessions: sessions.len(),
        errors,
        rollback_snapshot,
    })
}

async fn apply_import(
    store:    &AssistantStore,
    profiles: &[AssistantProfile],
    avatars:  &[AvatarAsset],
    sessions: &[SessionExport],
    mode:     &ImportMode,
    errors:   &mut Vec<String>,
) {
    if *mode == ImportMode::FullReplace {
        // Wipe existing data before import
        if let Err(e) = store.delete_all_sessions().await {
            errors.push(format!("clear sessions: {e}"));
        }
        if let Err(e) = store.delete_all_profiles().await {
            errors.push(format!("clear profiles: {e}"));
        }
        if let Err(e) = store.delete_all_avatars().await {
            errors.push(format!("clear avatars: {e}"));
        }
    }

    for profile in profiles {
        let mut p = profile.clone();
        if let ImportMode::ReplaceProfile { id } = mode {
            if &p.id != id { continue; }
        }
        if let ImportMode::Merge = mode {
            // On conflict: rename
            if store.profile_exists(&p.id).await.unwrap_or(false) {
                p.name = format!("{} (imported)", p.name);
                p.id   = format!("{}_imp", p.id);
                p.is_active = false;
            }
        }
        if let Err(e) = store.upsert_profile(p).await {
            errors.push(format!("upsert profile: {e}"));
        }
    }

    for avatar in avatars {
        if let Err(e) = store.upsert_avatar(avatar.clone()).await {
            errors.push(format!("upsert avatar: {e}"));
        }
    }

    for sx in sessions {
        let mut session = sx.session.clone();
        if let ImportMode::Merge = mode {
            if store.session_exists(&session.id).await.unwrap_or(false) {
                session.id    = format!("{}_imp", session.id);
                session.title = format!("{} (imported)", session.title);
            }
        }
        if let Err(e) = store.upsert_session(session.clone()).await {
            errors.push(format!("upsert session: {e}"));
            continue;
        }
        for msg in &sx.messages {
            let mut m = msg.clone();
            m.session_id = session.id.clone();
            if let Err(e) = store.append_message(m).await {
                errors.push(format!("append message: {e}"));
            }
        }
    }
}

// ── List / verify ─────────────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct BackupEntry {
    pub id:         String,
    pub filename:   String,
    pub file_path:  String,
    pub size_bytes: i64,
    pub includes:   Vec<String>,
    pub checksum:   Option<String>,
    pub encrypted:  bool,
    pub created_at: i64,
    pub valid:      Option<bool>,
}

pub async fn list_backups(store: &AssistantStore) -> Result<Vec<BackupEntry>, String> {
    store.list_backups_raw().await
}

pub async fn verify_backup(zip_path: &str, passphrase: Option<&str>) -> Result<bool, String> {
    let raw = std::fs::read(zip_path).map_err(|e| format!("read: {e}"))?;
    let zip_bytes = if is_encrypted(&raw) {
        let pass = passphrase.unwrap_or("");
        decrypt_bytes(&raw, pass)?
    } else {
        raw
    };

    let cursor = std::io::Cursor::new(&zip_bytes);
    let mut archive = ZipArchive::new(cursor).map_err(|e| format!("zip: {e}"))?;
    let manifest: Manifest = {
        let mut f = archive.by_name("manifest.json").map_err(|_| "no manifest")?;
        let mut s = String::new();
        f.read_to_string(&mut s).map_err(|e| format!("read manifest: {e}"))?;
        serde_json::from_str(&s).map_err(|e| format!("parse manifest: {e}"))?
    };

    for i in 0..archive.len() {
        let mut file = archive.by_index(i).map_err(|e| format!("entry {i}: {e}"))?;
        let name = file.name().to_string();
        if name == "manifest.json" { continue; }
        let mut data = Vec::new();
        file.read_to_end(&mut data).map_err(|e| format!("read {name}: {e}"))?;
        if let Some(expected) = manifest.checksums.get(&name) {
            if &hex_sha256(&data) != expected { return Ok(false); }
        }
    }
    Ok(true)
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn hex_sha256(data: &[u8]) -> String {
    let mut h = Sha256::new();
    h.update(data);
    format!("{:x}", h.finalize())
}

fn now_ms() -> i64 {
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_millis() as i64
}

fn is_encrypted(data: &[u8]) -> bool {
    // ZIP magic bytes: PK\x03\x04 — if absent, assume encrypted blob
    data.len() < 4 || data[0..4] != [0x50, 0x4b, 0x03, 0x04]
}
