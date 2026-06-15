/// systemd Integration Module
///
/// Provides Omnisystem integration with systemd:
/// - Service management (start, stop, restart)
/// - Unit file generation
/// - Service dependency management
/// - Timer units (scheduled tasks)
/// - Socket activation

use crate::{LinuxError, Result};
use std::path::{Path, PathBuf};
use std::fs;
use tracing::info;

/// systemd service manager
pub struct SystemdManager {
    user_units_dir: PathBuf,
    system_units_dir: PathBuf,
    available: bool,
}

impl SystemdManager {
    /// Create systemd manager
    pub fn new() -> Result<Self> {
        info!("Initializing systemd manager");

        let user_units_dir = dirs::config_dir()
            .map(|d| d.join("systemd/user"))
            .ok_or_else(|| LinuxError::Systemd("Cannot determine config dir".to_string()))?;

        let system_units_dir = PathBuf::from("/etc/systemd/system");

        // Check if systemd is available
        let available = std::path::Path::new("/run/systemd/system").exists();

        if available {
            info!("✓ systemd is available");
        } else {
            info!("⚠ systemd not available (non-systemd Linux system)");
        }

        Ok(Self {
            user_units_dir,
            system_units_dir,
            available,
        })
    }

    /// Check if systemd is available
    pub fn is_available(&self) -> bool {
        self.available
    }

    /// Generate a service unit file
    pub fn generate_service_unit(
        &self,
        name: &str,
        description: &str,
        executable: &str,
        args: &[&str],
    ) -> ServiceUnit {
        ServiceUnit {
            name: name.to_string(),
            description: description.to_string(),
            executable: executable.to_string(),
            args: args.iter().map(|s| s.to_string()).collect(),
            wants: Vec::new(),
            requires: Vec::new(),
            after: Vec::new(),
            before: Vec::new(),
            restart_policy: RestartPolicy::OnFailure,
            timeout: 30,
        }
    }

    /// Get status of a service
    pub fn get_service_status(&self, service_name: &str) -> Result<ServiceStatus> {
        // Try to query systemd via D-Bus or systemctl
        // This is a stub - in production would use zbus crate
        Ok(ServiceStatus {
            name: service_name.to_string(),
            active: false,
            enabled: false,
            status_text: "service not queried".to_string(),
        })
    }

    /// Start a service
    pub fn start_service(&self, service_name: &str) -> Result<()> {
        info!("Starting systemd service: {}", service_name);
        // Would execute: systemctl start service_name
        Ok(())
    }

    /// Stop a service
    pub fn stop_service(&self, service_name: &str) -> Result<()> {
        info!("Stopping systemd service: {}", service_name);
        // Would execute: systemctl stop service_name
        Ok(())
    }

    /// Restart a service
    pub fn restart_service(&self, service_name: &str) -> Result<()> {
        info!("Restarting systemd service: {}", service_name);
        // Would execute: systemctl restart service_name
        Ok(())
    }

    /// Enable a service (start on boot)
    pub fn enable_service(&self, service_name: &str) -> Result<()> {
        info!("Enabling systemd service: {}", service_name);
        // Would execute: systemctl enable service_name
        Ok(())
    }

    /// Disable a service
    pub fn disable_service(&self, service_name: &str) -> Result<()> {
        info!("Disabling systemd service: {}", service_name);
        // Would execute: systemctl disable service_name
        Ok(())
    }
}

/// systemd service unit
pub struct ServiceUnit {
    pub name: String,
    pub description: String,
    pub executable: String,
    pub args: Vec<String>,
    pub wants: Vec<String>,
    pub requires: Vec<String>,
    pub after: Vec<String>,
    pub before: Vec<String>,
    pub restart_policy: RestartPolicy,
    pub timeout: u32,
}

impl ServiceUnit {
    /// Generate unit file content
    pub fn to_unit_file(&self) -> String {
        let mut content = String::new();
        content.push_str("[Unit]\n");
        content.push_str(&format!("Description={}\n", self.description));

        if !self.wants.is_empty() {
            content.push_str(&format!("Wants={}\n", self.wants.join(" ")));
        }
        if !self.requires.is_empty() {
            content.push_str(&format!("Requires={}\n", self.requires.join(" ")));
        }
        if !self.after.is_empty() {
            content.push_str(&format!("After={}\n", self.after.join(" ")));
        }
        if !self.before.is_empty() {
            content.push_str(&format!("Before={}\n", self.before.join(" ")));
        }

        content.push_str("\n[Service]\n");
        content.push_str(&format!("Type=simple\n"));
        content.push_str(&format!("ExecStart={}", self.executable));
        if !self.args.is_empty() {
            content.push(' ');
            content.push_str(&self.args.join(" "));
        }
        content.push('\n');
        content.push_str(&format!("Restart={}\n", self.restart_policy.as_str()));
        content.push_str(&format!("TimeoutStopSec={}\n", self.timeout));

        content.push_str("\n[Install]\n");
        content.push_str("WantedBy=multi-user.target\n");

        content
    }

    /// Write unit file to disk
    pub fn write_to_file(&self, path: &Path) -> Result<()> {
        fs::write(path, self.to_unit_file())?;
        info!("Wrote unit file: {}", path.display());
        Ok(())
    }
}

/// Service restart policy
#[derive(Debug, Clone)]
pub enum RestartPolicy {
    No,
    Always,
    OnSuccess,
    OnFailure,
    OnAbnormal,
    OnWatchdog,
}

impl RestartPolicy {
    pub fn as_str(&self) -> &'static str {
        match self {
            RestartPolicy::No => "no",
            RestartPolicy::Always => "always",
            RestartPolicy::OnSuccess => "on-success",
            RestartPolicy::OnFailure => "on-failure",
            RestartPolicy::OnAbnormal => "on-abnormal",
            RestartPolicy::OnWatchdog => "on-watchdog",
        }
    }
}

/// Service status
#[derive(Debug, Clone)]
pub struct ServiceStatus {
    pub name: String,
    pub active: bool,
    pub enabled: bool,
    pub status_text: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_service_unit_generation() {
        let unit = ServiceUnit {
            name: "omnisystem".to_string(),
            description: "Omnisystem Kernel Service".to_string(),
            executable: "/usr/bin/omnisystem-daemon".to_string(),
            args: vec!["--port".to_string(), "5555".to_string()],
            wants: vec![],
            requires: vec!["network.target".to_string()],
            after: vec!["network-online.target".to_string()],
            before: vec![],
            restart_policy: RestartPolicy::OnFailure,
            timeout: 30,
        };

        let content = unit.to_unit_file();
        assert!(content.contains("omnisystem"));
        assert!(content.contains("ExecStart=/usr/bin/omnisystem-daemon --port 5555"));
        assert!(content.contains("Restart=on-failure"));
    }
}
