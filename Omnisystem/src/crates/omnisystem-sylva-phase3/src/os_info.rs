// Operating System Information and Detection

use serde::{Deserialize, Serialize};

/// Supported operating systems
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OperatingSystem {
    Linux,
    Windows,
    MacOS,
}

impl OperatingSystem {
    pub fn as_str(&self) -> &str {
        match self {
            OperatingSystem::Linux => "linux",
            OperatingSystem::Windows => "windows",
            OperatingSystem::MacOS => "macos",
        }
    }

    pub fn family(&self) -> &str {
        match self {
            OperatingSystem::Linux => "unix",
            OperatingSystem::Windows => "windows",
            OperatingSystem::MacOS => "unix",
        }
    }
}

/// OS version information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OSInfo {
    pub os: OperatingSystem,
    pub major_version: u32,
    pub minor_version: u32,
    pub patch_version: u32,
    pub full_version: String,
    pub build: String,
}

impl OSInfo {
    pub fn new(os: OperatingSystem, major: u32, minor: u32, patch: u32, build: String) -> Self {
        let full_version = format!("{}.{}.{}", major, minor, patch);
        Self {
            os,
            major_version: major,
            minor_version: minor,
            patch_version: patch,
            full_version,
            build,
        }
    }

    pub fn version_tuple(&self) -> (u32, u32, u32) {
        (self.major_version, self.minor_version, self.patch_version)
    }

    pub fn is_supported(&self) -> bool {
        match self.os {
            OperatingSystem::Linux => self.major_version >= 4, // Linux 4.0+
            OperatingSystem::Windows => self.major_version >= 10, // Windows 10+
            OperatingSystem::MacOS => self.major_version >= 10 && self.minor_version >= 14, // macOS 10.14+
        }
    }
}

/// Linux distribution information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LinuxDistro {
    Ubuntu,
    Debian,
    RHEL,
    CentOS,
    Fedora,
    ArchLinux,
    Alpine,
    OpenSUSE,
    Other(String),
}

impl LinuxDistro {
    pub fn as_str(&self) -> &str {
        match self {
            LinuxDistro::Ubuntu => "ubuntu",
            LinuxDistro::Debian => "debian",
            LinuxDistro::RHEL => "rhel",
            LinuxDistro::CentOS => "centos",
            LinuxDistro::Fedora => "fedora",
            LinuxDistro::ArchLinux => "arch",
            LinuxDistro::Alpine => "alpine",
            LinuxDistro::OpenSUSE => "opensuse",
            LinuxDistro::Other(name) => name,
        }
    }
}

/// Windows version information
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WindowsVersion {
    Windows10,
    Windows11,
    Other,
}

impl WindowsVersion {
    pub fn as_str(&self) -> &str {
        match self {
            WindowsVersion::Windows10 => "Windows 10",
            WindowsVersion::Windows11 => "Windows 11",
            WindowsVersion::Other => "Other",
        }
    }
}

/// macOS version information
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MacOSVersion {
    Monterey,      // 12
    Ventura,       // 13
    Sonoma,        // 14
    Sequoia,       // 15
    Other,
}

impl MacOSVersion {
    pub fn as_str(&self) -> &str {
        match self {
            MacOSVersion::Monterey => "macOS Monterey 12",
            MacOSVersion::Ventura => "macOS Ventura 13",
            MacOSVersion::Sonoma => "macOS Sonoma 14",
            MacOSVersion::Sequoia => "macOS Sequoia 15",
            MacOSVersion::Other => "Other",
        }
    }

    pub fn version_number(&self) -> u32 {
        match self {
            MacOSVersion::Monterey => 12,
            MacOSVersion::Ventura => 13,
            MacOSVersion::Sonoma => 14,
            MacOSVersion::Sequoia => 15,
            MacOSVersion::Other => 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_os_info() {
        let info = OSInfo::new(OperatingSystem::Linux, 5, 15, 0, "5.15.0-56".to_string());
        assert_eq!(info.major_version, 5);
        assert_eq!(info.full_version, "5.15.0");
        assert!(info.is_supported());
    }

    #[test]
    fn test_linux_distro_names() {
        assert_eq!(LinuxDistro::Ubuntu.as_str(), "ubuntu");
        assert_eq!(LinuxDistro::Alpine.as_str(), "alpine");
    }

    #[test]
    fn test_windows_version() {
        assert_eq!(WindowsVersion::Windows11.as_str(), "Windows 11");
    }

    #[test]
    fn test_macos_version() {
        assert_eq!(MacOSVersion::Sonoma.version_number(), 14);
    }
}
