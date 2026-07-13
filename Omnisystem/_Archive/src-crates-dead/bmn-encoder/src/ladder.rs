// Adaptive bitrate ladder generation

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BitrateProfile {
    pub resolution: String, // "1080p", "720p", etc.
    pub width: u32,
    pub height: u32,
    pub bitrate_kbps: u32,
    pub fps: u32,
    pub target_vmaf: f32, // 0-100
}

pub struct AdaptiveBitrateladder;

impl AdaptiveBitrateladder {
    /// Generate ABR ladder for a given source resolution
    pub fn generate_ladder(width: u32, height: u32, max_bitrate_kbps: u32) -> Vec<BitrateProfile> {
        // Content-aware ladder generation (simplified)
        let mut profiles = Vec::new();

        // Resolution tiers
        if width >= 1920 && height >= 1080 {
            profiles.push(BitrateProfile {
                resolution: "1080p".into(),
                width: 1920,
                height: 1080,
                bitrate_kbps: max_bitrate_kbps,
                fps: 60,
                target_vmaf: 95.0,
            });
        }

        if width >= 1280 && height >= 720 {
            profiles.push(BitrateProfile {
                resolution: "720p".into(),
                width: 1280,
                height: 720,
                bitrate_kbps: (max_bitrate_kbps as f32 * 0.4) as u32,
                fps: 60,
                target_vmaf: 90.0,
            });
        }

        if width >= 854 && height >= 480 {
            profiles.push(BitrateProfile {
                resolution: "480p".into(),
                width: 854,
                height: 480,
                bitrate_kbps: (max_bitrate_kbps as f32 * 0.15) as u32,
                fps: 30,
                target_vmaf: 80.0,
            });
        }

        profiles.push(BitrateProfile {
            resolution: "360p".into(),
            width: 640,
            height: 360,
            bitrate_kbps: (max_bitrate_kbps as f32 * 0.08) as u32,
            fps: 30,
            target_vmaf: 70.0,
        });

        profiles
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ladder_generation() {
        let ladder = AdaptiveBitrateladder::generate_ladder(1920, 1080, 5000);
        assert!(!ladder.is_empty());
        assert!(ladder[0].bitrate_kbps >= ladder[1].bitrate_kbps);
    }
}
