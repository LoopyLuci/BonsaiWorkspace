use crate::Result;
use dashmap::DashMap;
use std::sync::Arc;

pub struct AchievementManager {
    achievements: Arc<DashMap<String, Achievement>>,
    user_achievements: Arc<DashMap<String, Vec<String>>>,
}

#[derive(Debug, Clone)]
pub struct Achievement {
    pub id: String,
    pub name: String,
    pub description: String,
    pub badge_url: String,
}

impl AchievementManager {
    pub fn new() -> Self {
        Self {
            achievements: Arc::new(DashMap::new()),
            user_achievements: Arc::new(DashMap::new()),
        }
    }

    pub fn define_achievement(&self, achievement: Achievement) -> Result<()> {
        self.achievements.insert(achievement.id.clone(), achievement);
        Ok(())
    }

    pub fn unlock_achievement(&self, user_id: String, achievement_id: String) -> Result<()> {
        self.user_achievements
            .entry(user_id)
            .or_insert_with(Vec::new)
            .push(achievement_id);
        tracing::info!("Achievement unlocked");
        Ok(())
    }

    pub fn get_user_achievements(&self, user_id: &str) -> Vec<Achievement> {
        if let Some(ids) = self.user_achievements.get(user_id) {
            ids.value()
                .iter()
                .filter_map(|id| self.achievements.get(id).map(|a| a.value().clone()))
                .collect()
        } else {
            Vec::new()
        }
    }

    pub fn achievement_count(&self) -> usize {
        self.achievements.len()
    }
}

impl Default for AchievementManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_achievements() {
        let manager = AchievementManager::new();
        let achievement = Achievement {
            id: "first_course".to_string(),
            name: "First Course".to_string(),
            description: "Complete your first course".to_string(),
            badge_url: "badge.png".to_string(),
        };
        assert!(manager.define_achievement(achievement).is_ok());
        assert!(manager.unlock_achievement("u1".to_string(), "first_course".to_string()).is_ok());
        assert_eq!(manager.achievement_count(), 1);
    }
}
