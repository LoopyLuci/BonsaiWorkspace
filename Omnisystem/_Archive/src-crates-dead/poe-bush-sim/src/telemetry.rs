use poe_core::HostBiometricTelemetry;
use rand::Rng;

pub struct BiometricSimulator {
    rng: rand::rngs::ThreadRng,
    base_heart_rate: u32,
}

impl BiometricSimulator {
    pub fn new() -> Self {
        Self {
            rng: rand::thread_rng(),
            base_heart_rate: 72,
        }
    }

    /// Generate normal resting telemetry.
    pub fn normal(&mut self) -> HostBiometricTelemetry {
        let variation: i32 = self.rng.gen_range(-5..=5);
        HostBiometricTelemetry {
            host_id: "sim-companion-1".into(),
            is_conscious: true,
            heart_rate: (self.base_heart_rate as i32 + variation) as u32,
            is_captured: false,
            cryptographic_signature: "sim-sig-valid".into(),
        }
    }

    /// Generate elevated stress telemetry.
    pub fn stressed(&mut self) -> HostBiometricTelemetry {
        HostBiometricTelemetry {
            host_id: "sim-companion-1".into(),
            is_conscious: true,
            heart_rate: self.rng.gen_range(110..=130),
            is_captured: false,
            cryptographic_signature: "sim-sig-valid".into(),
        }
    }

    /// Generate critical trauma telemetry.
    pub fn critical(&mut self) -> HostBiometricTelemetry {
        HostBiometricTelemetry {
            host_id: "sim-companion-1".into(),
            is_conscious: true,
            heart_rate: self.rng.gen_range(155..=180),
            is_captured: false,
            cryptographic_signature: "sim-sig-valid".into(),
        }
    }

    /// Generate captive/distress telemetry.
    pub fn captive(&mut self) -> HostBiometricTelemetry {
        HostBiometricTelemetry {
            host_id: "sim-companion-1".into(),
            is_conscious: true,
            heart_rate: self.rng.gen_range(120..=145),
            is_captured: true,
            cryptographic_signature: "sim-sig-valid".into(),
        }
    }

    /// Generate unconscious emergency telemetry.
    pub fn unconscious(&mut self) -> HostBiometricTelemetry {
        HostBiometricTelemetry {
            host_id: "sim-companion-1".into(),
            is_conscious: false,
            heart_rate: self.rng.gen_range(40..=55),
            is_captured: false,
            cryptographic_signature: "sim-sig-valid".into(),
        }
    }
}

impl Default for BiometricSimulator {
    fn default() -> Self {
        Self::new()
    }
}
