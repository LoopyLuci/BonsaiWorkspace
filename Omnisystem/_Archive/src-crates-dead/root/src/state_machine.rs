use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum InstallerMode {
    Boot,
    DetectInstall,
    FetchManifest,
    VerifyManifest,
    Welcome,
    SimpleInstall,
    AdvancedInstall,
    PlanExecute,
    Verify,
    Complete,
    UpdateCheck,
    UpdatePlan,
    UpdateExecute,
    Launcher,
    Repair,
}

#[derive(Debug, Clone)]
pub struct StateMachine {
    pub current_mode: InstallerMode,
    pub history: Vec<InstallerMode>,
}

impl Default for StateMachine {
    fn default() -> Self {
        Self {
            current_mode: InstallerMode::Boot,
            history: Vec::new(),
        }
    }
}

impl StateMachine {
    pub fn transition(&mut self, next: InstallerMode) -> Result<()> {
        if !self.is_valid_transition(&next) {
            return Err(anyhow!(
                "invalid transition from {:?} to {:?}",
                self.current_mode,
                next
            ));
        }
        self.history.push(self.current_mode.clone());
        self.current_mode = next;
        Ok(())
    }

    pub fn is_valid_transition(&self, next: &InstallerMode) -> bool {
        use InstallerMode::*;
        matches!(
            (&self.current_mode, next),
            (Boot, DetectInstall)
                | (DetectInstall, FetchManifest)
                | (DetectInstall, UpdateCheck)
                | (FetchManifest, VerifyManifest)
                | (VerifyManifest, Welcome)
                | (Welcome, SimpleInstall)
                | (Welcome, AdvancedInstall)
                | (SimpleInstall, PlanExecute)
                | (AdvancedInstall, PlanExecute)
                | (PlanExecute, Verify)
                | (Verify, Complete)
                | (Complete, Launcher)
                | (UpdateCheck, UpdatePlan)
                | (UpdatePlan, UpdateExecute)
                | (UpdateExecute, Verify)
                | (Launcher, Repair)
                | (Repair, PlanExecute)
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_valid_paths() {
        let mut sm = StateMachine::default();
        sm.transition(InstallerMode::DetectInstall).expect("transition must pass");
        sm.transition(InstallerMode::FetchManifest).expect("transition must pass");
        sm.transition(InstallerMode::VerifyManifest).expect("transition must pass");
        sm.transition(InstallerMode::Welcome).expect("transition must pass");
    }

    #[test]
    fn rejects_invalid_paths() {
        let mut sm = StateMachine::default();
        let err = sm
            .transition(InstallerMode::Launcher)
            .expect_err("transition must fail");
        assert!(err.to_string().contains("invalid transition"));
    }
}
