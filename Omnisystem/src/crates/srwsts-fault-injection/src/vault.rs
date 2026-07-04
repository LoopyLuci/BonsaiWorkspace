//! Vault communication protocol (virtio-fault channel simulation).

use crate::fault::{FaultId, FaultType};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Vault message types for fault injection communication.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "action", content = "payload")]
pub enum VaultMessage {
    /// Request to inject a fault.
    InjectFault {
        fault_id: FaultId,
        fault_type: FaultType,
        duration_secs: u64,
    },

    /// Acknowledge fault injection.
    FaultInjectionAck {
        fault_id: FaultId,
        status: String,
    },

    /// Request fault recovery.
    RecoverFault {
        fault_id: FaultId,
    },

    /// Acknowledge fault recovery.
    FaultRecoveryAck {
        fault_id: FaultId,
        status: String,
    },

    /// Get current fault status.
    GetFaultStatus {
        fault_id: Option<FaultId>,
    },

    /// Fault status response.
    FaultStatusResponse {
        active_faults: Vec<FaultStatusInfo>,
    },

    /// Heartbeat to Vault.
    Heartbeat {
        injector_id: String,
        active_faults: u32,
    },

    /// Heartbeat response.
    HeartbeatAck {
        timestamp: u64,
    },

    /// Error response.
    Error {
        error_code: u32,
        message: String,
    },

    /// Shutdown signal.
    Shutdown {
        reason: String,
    },
}

/// Fault status information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FaultStatusInfo {
    pub fault_id: FaultId,
    pub fault_type_name: String,
    pub injected_at: u64,
    pub expected_recovery: u64,
    pub current_status: FaultStatus,
}

/// Status of a fault in the Vault.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FaultStatus {
    Pending,
    Injected,
    Active,
    Recovering,
    Recovered,
    Failed,
}

impl FaultStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            FaultStatus::Pending => "Pending",
            FaultStatus::Injected => "Injected",
            FaultStatus::Active => "Active",
            FaultStatus::Recovering => "Recovering",
            FaultStatus::Recovered => "Recovered",
            FaultStatus::Failed => "Failed",
        }
    }
}

impl std::fmt::Display for FaultStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Vault protocol handler (simulates virtio-fault channel).
pub struct VaultProtocol {
    injector_id: String,
    sequence: u64,
}

impl VaultProtocol {
    /// Create a new Vault protocol handler.
    pub fn new(injector_id: String) -> Self {
        Self {
            injector_id,
            sequence: 0,
        }
    }

    /// Create an inject fault message.
    pub fn create_inject_fault(
        &mut self,
        fault_id: FaultId,
        fault_type: FaultType,
        duration_secs: u64,
    ) -> VaultMessage {
        self.sequence += 1;
        VaultMessage::InjectFault {
            fault_id,
            fault_type,
            duration_secs,
        }
    }

    /// Create a recover fault message.
    pub fn create_recover_fault(&mut self, fault_id: FaultId) -> VaultMessage {
        self.sequence += 1;
        VaultMessage::RecoverFault { fault_id }
    }

    /// Create a status query message.
    pub fn create_status_query(&mut self, fault_id: Option<FaultId>) -> VaultMessage {
        self.sequence += 1;
        VaultMessage::GetFaultStatus { fault_id }
    }

    /// Create a heartbeat message.
    pub fn create_heartbeat(&mut self, active_faults: u32) -> VaultMessage {
        self.sequence += 1;
        VaultMessage::Heartbeat {
            injector_id: self.injector_id.clone(),
            active_faults,
        }
    }

    /// Create an error response.
    pub fn create_error(&mut self, error_code: u32, message: String) -> VaultMessage {
        self.sequence += 1;
        VaultMessage::Error {
            error_code,
            message,
        }
    }

    /// Parse a Vault response.
    pub fn parse_response(&self, message: &VaultMessage) -> Result<(), String> {
        match message {
            VaultMessage::FaultInjectionAck { status, .. } => {
                if status == "success" {
                    Ok(())
                } else {
                    Err(format!("injection failed: {}", status))
                }
            }
            VaultMessage::FaultRecoveryAck { status, .. } => {
                if status == "success" {
                    Ok(())
                } else {
                    Err(format!("recovery failed: {}", status))
                }
            }
            VaultMessage::Error {
                error_code,
                message,
            } => Err(format!("vault error {}: {}", error_code, message)),
            _ => Ok(()),
        }
    }
}

/// Vault channel abstraction.
pub struct VaultChannel {
    protocol: VaultProtocol,
    message_log: Vec<(u64, VaultMessage)>,
}

impl VaultChannel {
    /// Create a new Vault channel.
    pub fn new(injector_id: String) -> Self {
        Self {
            protocol: VaultProtocol::new(injector_id),
            message_log: Vec::new(),
        }
    }

    /// Send a message and log it.
    pub async fn send(&mut self, message: VaultMessage) -> crate::error::Result<()> {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|e| {
                crate::error::FaultError::VaultCommunicationError(e.to_string())
            })?
            .as_secs();

        self.message_log.push((timestamp, message.clone()));
        tracing::debug!("vault message sent: {:?}", message);
        Ok(())
    }

    /// Get message log.
    pub fn message_log(&self) -> &[(u64, VaultMessage)] {
        &self.message_log
    }

    /// Clear message log.
    pub fn clear_log(&mut self) {
        self.message_log.clear();
    }
}

/// Vault connection statistics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VaultStats {
    pub total_messages_sent: u64,
    pub total_messages_received: u64,
    pub connection_uptime_secs: u64,
    pub last_heartbeat: u64,
    pub errors: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vault_protocol_creation() {
        let protocol = VaultProtocol::new("injector_1".to_string());
        assert_eq!(protocol.injector_id, "injector_1");
    }

    #[test]
    fn test_vault_message_creation() {
        let mut protocol = VaultProtocol::new("injector_1".to_string());
        let fault_id = uuid::Uuid::new_v4();
        let fault_type = crate::fault::FaultType::MemoryPressure {
            pressure_percent: 50,
            page_faults: 1000,
        };

        let msg = protocol.create_inject_fault(fault_id, fault_type, 5);
        assert!(matches!(msg, VaultMessage::InjectFault { .. }));
    }

    #[test]
    fn test_vault_channel() {
        let channel = VaultChannel::new("injector_1".to_string());
        assert!(channel.message_log().is_empty());
    }

    #[test]
    fn test_fault_status_display() {
        assert_eq!(FaultStatus::Active.as_str(), "Active");
        assert_eq!(FaultStatus::Recovered.as_str(), "Recovered");
    }
}
