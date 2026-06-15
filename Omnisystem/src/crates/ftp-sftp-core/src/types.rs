use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, Hash)]
pub struct SessionId(pub Uuid);

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, Hash)]
pub struct UserId(pub String);

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, Copy)]
pub enum Protocol {
    Ftp,
    Sftp,
    Ftps,
}

impl Protocol {
    pub fn to_string(&self) -> &'static str {
        match self {
            Protocol::Ftp => "FTP",
            Protocol::Sftp => "SFTP",
            Protocol::Ftps => "FTPS",
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Credentials {
    pub username: String,
    pub password: String,
    pub public_key: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum SessionStatus {
    Authenticated,
    Idle,
    Transferring,
    Closed,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FtpSession {
    pub id: SessionId,
    pub user_id: UserId,
    pub protocol: Protocol,
    pub status: SessionStatus,
    pub remote_addr: String,
    pub created_at: DateTime<Utc>,
    pub last_activity: DateTime<Utc>,
    pub bytes_uploaded: u64,
    pub bytes_downloaded: u64,
}

impl FtpSession {
    pub fn new(
        user_id: UserId,
        protocol: Protocol,
        remote_addr: String,
    ) -> Self {
        Self {
            id: SessionId(Uuid::new_v4()),
            user_id,
            protocol,
            status: SessionStatus::Authenticated,
            remote_addr,
            created_at: Utc::now(),
            last_activity: Utc::now(),
            bytes_uploaded: 0,
            bytes_downloaded: 0,
        }
    }

    pub fn duration_secs(&self) -> u64 {
        (Utc::now() - self.created_at).num_seconds() as u64
    }

    pub fn total_bytes(&self) -> u64 {
        self.bytes_uploaded + self.bytes_downloaded
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum FileType {
    Directory,
    File,
    Link,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FileMetadata {
    pub path: String,
    pub file_type: FileType,
    pub size: u64,
    pub modified: DateTime<Utc>,
    pub permissions: u32,
    pub owner: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TransferInfo {
    pub session_id: SessionId,
    pub remote_path: String,
    pub local_path: String,
    pub protocol: Protocol,
    pub bytes_transferred: u64,
    pub total_bytes: u64,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
}

#[derive(Clone, Debug, Copy, Serialize, Deserialize)]
pub struct BandwidthConfig {
    pub upload_limit_bytes_per_sec: u64,
    pub download_limit_bytes_per_sec: u64,
    pub burst_size_bytes: u64,
}

impl Default for BandwidthConfig {
    fn default() -> Self {
        Self {
            upload_limit_bytes_per_sec: 10_000_000,
            download_limit_bytes_per_sec: 10_000_000,
            burst_size_bytes: 50_000_000,
        }
    }
}

#[derive(Clone, Debug, Copy, Serialize, Deserialize)]
pub struct QuotaConfig {
    pub max_storage_bytes: u64,
    pub max_upload_bytes_per_day: u64,
    pub max_download_bytes_per_day: u64,
    pub max_files: u64,
}

impl Default for QuotaConfig {
    fn default() -> Self {
        Self {
            max_storage_bytes: 1_000_000_000,
            max_upload_bytes_per_day: 100_000_000,
            max_download_bytes_per_day: 100_000_000,
            max_files: 10_000,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct UserAccount {
    pub user_id: UserId,
    pub credentials: Credentials,
    pub home_directory: String,
    pub bandwidth_config: BandwidthConfig,
    pub quota_config: QuotaConfig,
    pub enabled: bool,
    pub created_at: DateTime<Utc>,
}

impl UserAccount {
    pub fn new(user_id: UserId, credentials: Credentials, home_directory: String) -> Self {
        Self {
            user_id,
            credentials,
            home_directory,
            bandwidth_config: BandwidthConfig::default(),
            quota_config: QuotaConfig::default(),
            enabled: true,
            created_at: Utc::now(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct QuotaUsage {
    pub user_id: UserId,
    pub current_storage_bytes: u64,
    pub upload_bytes_today: u64,
    pub download_bytes_today: u64,
    pub file_count: u64,
    pub last_reset: DateTime<Utc>,
}

impl Default for QuotaUsage {
    fn default() -> Self {
        Self {
            user_id: UserId("".to_string()),
            current_storage_bytes: 0,
            upload_bytes_today: 0,
            download_bytes_today: 0,
            file_count: 0,
            last_reset: Utc::now(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BandwidthStats {
    pub session_id: SessionId,
    pub upload_bytes: u64,
    pub download_bytes: u64,
    pub current_upload_rate: f64,
    pub current_download_rate: f64,
    pub average_upload_rate: f64,
    pub average_download_rate: f64,
}

#[derive(Clone, Debug, Copy, Serialize, Deserialize)]
pub enum TransferMode {
    Binary,
    Ascii,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FtpCommand {
    pub command: String,
    pub args: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FtpResponse {
    pub code: u16,
    pub message: String,
}
