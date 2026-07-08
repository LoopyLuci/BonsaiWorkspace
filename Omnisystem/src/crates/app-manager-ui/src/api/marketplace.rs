//! Marketplace API endpoints

use serde::{Deserialize, Serialize};
use tauri::command;

/// Review information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReviewInfo {
    pub id: String,
    pub rating: u8,
    pub title: String,
    pub content: String,
    pub helpful_count: u32,
}

/// Trending app information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrendingApp {
    pub rank: usize,
    pub name: String,
    pub downloads: u32,
    pub trending_score: f32,
}

/// Rate an app
#[command]
pub async fn rate_app(app_id: String, rating: u8) -> Result<String, String> {
    if app_id.is_empty() {
        return Err("App ID required".to_string());
    }

    if rating < 1 || rating > 5 {
        return Err("Rating must be between 1 and 5".to_string());
    }

    // Mock implementation
    // In production: Call POST /api/apps/{id}/rate
    Ok(format!("Rated {} stars", rating))
}

/// Get app reviews
#[command]
pub async fn get_reviews(app_id: String) -> Result<Vec<ReviewInfo>, String> {
    if app_id.is_empty() {
        return Err("App ID required".to_string());
    }

    // Mock implementation
    // In production: Call GET /api/apps/{id}/reviews
    Ok(vec![ReviewInfo {
        id: "review-1".to_string(),
        rating: 5,
        title: "Great app!".to_string(),
        content: "Works perfectly".to_string(),
        helpful_count: 42,
    }])
}

/// Get trending apps
#[command]
pub async fn get_trending() -> Result<Vec<TrendingApp>, String> {
    // Mock implementation
    // In production: Call GET /api/trending
    Ok(vec![
        TrendingApp {
            rank: 1,
            name: "Productivity Pro".to_string(),
            downloads: 5000,
            trending_score: 4.8,
        },
        TrendingApp {
            rank: 2,
            name: "File Manager".to_string(),
            downloads: 3000,
            trending_score: 4.6,
        },
    ])
}

/// Get featured apps
#[command]
pub async fn get_featured() -> Result<Vec<String>, String> {
    // Mock implementation
    // In production: Call GET /api/featured
    Ok(vec![
        "Productivity Pro".to_string(),
        "File Manager".to_string(),
    ])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_rate_app_valid() {
        let result = rate_app("app-1".to_string(), 5).await;
        assert!(result.is_ok());
        assert!(result.unwrap().contains("5 stars"));
    }

    #[tokio::test]
    async fn test_rate_app_invalid_rating() {
        let result = rate_app("app-1".to_string(), 0).await;
        assert!(result.is_err());

        let result = rate_app("app-1".to_string(), 6).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_rate_app_empty_id() {
        let result = rate_app(String::new(), 5).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_get_reviews() {
        let result = get_reviews("app-1".to_string()).await;
        assert!(result.is_ok());

        let reviews = result.unwrap();
        assert!(reviews.len() > 0);
        assert_eq!(reviews[0].rating, 5);
    }

    #[tokio::test]
    async fn test_get_reviews_empty_id() {
        let result = get_reviews(String::new()).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_get_trending() {
        let result = get_trending().await;
        assert!(result.is_ok());

        let apps = result.unwrap();
        assert!(apps.len() > 0);
        assert_eq!(apps[0].rank, 1);
    }

    #[tokio::test]
    async fn test_get_featured() {
        let result = get_featured().await;
        assert!(result.is_ok());

        let apps = result.unwrap();
        assert!(apps.len() > 0);
    }
}
