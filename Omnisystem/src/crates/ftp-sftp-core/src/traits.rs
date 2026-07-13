use async_trait::async_trait;
use crate::{
    BandwidthStats, Credentials, FileMetadata, FtpResult, FtpSession,
    Protocol, QuotaUsage, SessionId, TransferInfo, UserAccount, UserId,
};

#[async_trait]
pub trait SessionManager: Send + Sync {
    async fn create_session(
        &self,
        user_id: &UserId,
        protocol: Protocol,
        remote_addr: String,
    ) -> FtpResult<SessionId>;

    async fn get_session(&self, session_id: &SessionId) -> FtpResult<FtpSession>;

    async fn update_session(&self, session_id: &SessionId, session: FtpSession) -> FtpResult<()>;

    async fn close_session(&self, session_id: &SessionId) -> FtpResult<()>;

    async fn list_sessions(&self, user_id: &UserId) -> FtpResult<Vec<FtpSession>>;

    async fn get_active_session_count(&self, user_id: &UserId) -> FtpResult<usize>;
}

#[async_trait]
pub trait UserManager: Send + Sync {
    async fn create_user(
        &self,
        user_id: &UserId,
        credentials: Credentials,
        home_directory: String,
    ) -> FtpResult<UserAccount>;

    async fn get_user(&self, user_id: &UserId) -> FtpResult<UserAccount>;

    async fn authenticate(&self, user_id: &UserId, password: &str) -> FtpResult<bool>;

    async fn update_user(&self, user_id: &UserId, account: UserAccount) -> FtpResult<()>;

    async fn delete_user(&self, user_id: &UserId) -> FtpResult<()>;

    async fn list_users(&self) -> FtpResult<Vec<UserAccount>>;
}

#[async_trait]
pub trait FileOperations: Send + Sync {
    async fn upload_file(
        &self,
        session_id: &SessionId,
        remote_path: &str,
        data: Vec<u8>,
    ) -> FtpResult<u64>;

    async fn download_file(&self, session_id: &SessionId, remote_path: &str) -> FtpResult<Vec<u8>>;

    async fn delete_file(&self, session_id: &SessionId, remote_path: &str) -> FtpResult<()>;

    async fn list_directory(
        &self,
        session_id: &SessionId,
        path: &str,
    ) -> FtpResult<Vec<FileMetadata>>;

    async fn create_directory(
        &self,
        session_id: &SessionId,
        path: &str,
    ) -> FtpResult<()>;

    async fn delete_directory(&self, session_id: &SessionId, path: &str) -> FtpResult<()>;

    async fn get_file_metadata(
        &self,
        session_id: &SessionId,
        path: &str,
    ) -> FtpResult<FileMetadata>;

    async fn rename_file(
        &self,
        session_id: &SessionId,
        old_path: &str,
        new_path: &str,
    ) -> FtpResult<()>;
}

#[async_trait]
pub trait BandwidthControl: Send + Sync {
    async fn record_upload(&self, session_id: &SessionId, bytes: u64) -> FtpResult<()>;

    async fn record_download(&self, session_id: &SessionId, bytes: u64) -> FtpResult<()>;

    async fn check_bandwidth_limit(
        &self,
        session_id: &SessionId,
        bytes: u64,
    ) -> FtpResult<bool>;

    async fn get_bandwidth_stats(&self, session_id: &SessionId) -> FtpResult<BandwidthStats>;

    async fn reset_bandwidth(&self, session_id: &SessionId) -> FtpResult<()>;
}

#[async_trait]
pub trait QuotaControl: Send + Sync {
    async fn check_upload_quota(&self, user_id: &UserId, bytes: u64) -> FtpResult<bool>;

    async fn check_download_quota(&self, user_id: &UserId, bytes: u64) -> FtpResult<bool>;

    async fn check_storage_quota(&self, user_id: &UserId, bytes: u64) -> FtpResult<bool>;

    async fn record_upload(&self, user_id: &UserId, bytes: u64) -> FtpResult<()>;

    async fn record_download(&self, user_id: &UserId, bytes: u64) -> FtpResult<()>;

    async fn record_storage_usage(&self, user_id: &UserId, bytes: u64) -> FtpResult<()>;

    async fn get_quota_usage(&self, user_id: &UserId) -> FtpResult<QuotaUsage>;

    async fn reset_daily_quotas(&self, user_id: &UserId) -> FtpResult<()>;
}

#[async_trait]
pub trait TransferTracker: Send + Sync {
    async fn start_transfer(&self, transfer: TransferInfo) -> FtpResult<()>;

    async fn update_transfer(&self, transfer: TransferInfo) -> FtpResult<()>;

    async fn complete_transfer(&self, transfer: TransferInfo) -> FtpResult<()>;

    async fn get_active_transfers(&self, session_id: &SessionId) -> FtpResult<Vec<TransferInfo>>;

    async fn get_transfer_history(&self, user_id: &UserId) -> FtpResult<Vec<TransferInfo>>;
}
