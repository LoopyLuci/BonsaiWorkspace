// OS credential vault abstraction.
// On Windows: Windows Credential Manager
// On macOS: Keychain Services
// On Linux: libsecret / keyutils
// All via the `keyring` crate.

const SERVICE: &str = "assistant";

pub struct SecretsStore;

impl SecretsStore {
    pub fn new() -> Self {
        SecretsStore
    }

    /// Store a secret in the OS credential vault.
    pub fn store(&self, account: &str, secret: &str) -> Result<(), String> {
        keyring::Entry::new(SERVICE, account)
            .map_err(|e| format!("keyring entry error: {e}"))?
            .set_password(secret)
            .map_err(|e| format!("keyring set error: {e}"))
    }

    /// Retrieve a secret. Returns None if not found.
    pub fn get(&self, account: &str) -> Result<Option<String>, String> {
        let entry = keyring::Entry::new(SERVICE, account)
            .map_err(|e| format!("keyring entry error: {e}"))?;
        match entry.get_password() {
            Ok(pw) => Ok(Some(pw)),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(e) => Err(format!("keyring get error: {e}")),
        }
    }

    /// Delete a secret. No-op if it doesn't exist.
    pub fn delete(&self, account: &str) -> Result<(), String> {
        let entry = keyring::Entry::new(SERVICE, account)
            .map_err(|e| format!("keyring entry error: {e}"))?;
        match entry.delete_password() {
            Ok(()) => Ok(()),
            Err(keyring::Error::NoEntry) => Ok(()),
            Err(e) => Err(format!("keyring delete error: {e}")),
        }
    }

    /// Check whether a secret exists without reading it.
    pub fn has(&self, account: &str) -> bool {
        self.get(account).ok().flatten().is_some()
    }
}

// ── Named accounts ─────────────────────────────────────────────────────────

pub const ACCOUNT_SMTP_PASSWORD: &str = "smtp_password";
pub const ACCOUNT_SMTP_USERNAME: &str = "smtp_username";
pub const ACCOUNT_SMTP_HOST: &str = "smtp_host";
pub const ACCOUNT_SMTP_FROM: &str = "smtp_from";
pub const ACCOUNT_DESKTOP_CONNECTION_TOKEN: &str = "desktop_connection_token";
