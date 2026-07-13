use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use lazy_static::lazy_static;
use std::collections::HashSet;

lazy_static! {
    static ref FAVORITES: Mutex<HashSet<String>> = Mutex::new(HashSet::new());
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct FavoritesResponse {
    pub success: bool,
    pub message: String,
}

#[tauri::command]
pub async fn add_favorite(app_id: String) -> Result<FavoritesResponse, String> {
    match FAVORITES.lock() {
        Ok(mut favs) => {
            favs.insert(app_id.clone());
            Ok(FavoritesResponse {
                success: true,
                message: format!("Added {} to favorites", app_id),
            })
        }
        Err(_) => Err("Failed to acquire favorites lock".to_string()),
    }
}

#[tauri::command]
pub async fn remove_favorite(app_id: String) -> Result<FavoritesResponse, String> {
    match FAVORITES.lock() {
        Ok(mut favs) => {
            favs.remove(&app_id);
            Ok(FavoritesResponse {
                success: true,
                message: format!("Removed {} from favorites", app_id),
            })
        }
        Err(_) => Err("Failed to acquire favorites lock".to_string()),
    }
}

#[tauri::command]
pub async fn get_favorites() -> Result<Vec<String>, String> {
    match FAVORITES.lock() {
        Ok(favs) => Ok(favs.iter().cloned().collect()),
        Err(_) => Err("Failed to acquire favorites lock".to_string()),
    }
}

#[tauri::command]
pub async fn is_favorite(app_id: String) -> Result<bool, String> {
    match FAVORITES.lock() {
        Ok(favs) => Ok(favs.contains(&app_id)),
        Err(_) => Err("Failed to acquire favorites lock".to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_add_favorite() {
        let result = add_favorite("app-1".to_string()).await;
        assert!(result.is_ok());
        assert!(result.unwrap().success);
    }

    #[tokio::test]
    async fn test_remove_favorite() {
        add_favorite("app-2".to_string()).await.ok();
        let result = remove_favorite("app-2".to_string()).await;
        assert!(result.is_ok());
        assert!(result.unwrap().success);
    }

    #[tokio::test]
    async fn test_is_favorite() {
        add_favorite("app-3".to_string()).await.ok();
        let result = is_favorite("app-3".to_string()).await;
        assert!(result.is_ok());
        assert!(result.unwrap());
    }

    #[tokio::test]
    async fn test_get_favorites() {
        add_favorite("app-4".to_string()).await.ok();
        let result = get_favorites().await;
        assert!(result.is_ok());
        let favs = result.unwrap();
        assert!(favs.contains(&"app-4".to_string()));
    }
}
