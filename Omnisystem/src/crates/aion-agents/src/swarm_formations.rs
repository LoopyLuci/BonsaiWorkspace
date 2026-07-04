use dashmap::DashMap;
use std::sync::Arc;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FormationType {
    VFormation,
    LineFormation,
    CircleFormation,
    SquareFormation,
}

#[derive(Debug, Clone)]
pub struct FormationMember {
    pub agent_id: String,
    pub position_offset: (f32, f32),
    pub rotation_offset: f32,
}

pub struct FormationController {
    formations: Arc<DashMap<String, FormationType>>,
    members: Arc<DashMap<String, Vec<FormationMember>>>,
}

impl FormationController {
    pub fn new() -> Self {
        Self {
            formations: Arc::new(DashMap::new()),
            members: Arc::new(DashMap::new()),
        }
    }

    pub fn create_formation(&self, formation_id: String, formation_type: FormationType) {
        self.formations.insert(formation_id, formation_type);
    }

    pub fn add_member(&self, formation_id: String, member: FormationMember) -> bool {
        if let Some(mut members) = self.members.get_mut(&formation_id) {
            members.push(member);
            true
        } else {
            self.members.insert(formation_id.clone(), vec![member]);
            true
        }
    }

    pub fn get_formation_type(&self, formation_id: &str) -> Option<FormationType> {
        self.formations.get(formation_id).map(|f| *f)
    }

    pub fn get_members(&self, formation_id: &str) -> Option<Vec<FormationMember>> {
        self.members.get(formation_id).map(|m| m.clone())
    }

    pub fn member_count(&self, formation_id: &str) -> usize {
        self.members.get(formation_id).map(|m| m.len()).unwrap_or(0)
    }

    pub fn formation_count(&self) -> usize {
        self.formations.len()
    }
}

impl Default for FormationController {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_formation_creation() {
        let fc = FormationController::new();
        fc.create_formation("f1".to_string(), FormationType::VFormation);
        assert_eq!(fc.formation_count(), 1);
    }

    #[test]
    fn test_member_addition() {
        let fc = FormationController::new();
        fc.create_formation("f1".to_string(), FormationType::LineFormation);
        let member = FormationMember {
            agent_id: "a1".to_string(),
            position_offset: (0.0, 0.0),
            rotation_offset: 0.0,
        };
        assert!(fc.add_member("f1".to_string(), member));
        assert_eq!(fc.member_count("f1"), 1);
    }
}
