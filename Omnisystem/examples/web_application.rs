// OMNISYSTEM WEB APPLICATION EXAMPLE
// Demonstrates Titan, Sylva, and Aether working together

use std::sync::Arc;

// ============================================================================
// APPLICATION TYPES
// ============================================================================

#[derive(Debug, Clone)]
pub struct User {
    pub id: u64,
    pub name: String,
    pub email: String,
    pub created_at: String,
}

#[derive(Debug, Clone)]
pub struct PredictionRequest {
    pub features: Vec<f64>,
    pub model_id: String,
}

#[derive(Debug, Clone)]
pub struct PredictionResponse {
    pub prediction: f64,
    pub confidence: f64,
    pub model_version: String,
}

// ============================================================================
// API SERVICE (Titan - Systems Programming)
// ============================================================================

pub struct ApiService {
    users: Arc<std::sync::Mutex<Vec<User>>>,
    port: u16,
}

impl ApiService {
    pub fn new(port: u16) -> Self {
        ApiService {
            users: Arc::new(std::sync::Mutex::new(Vec::new())),
            port,
        }
    }

    pub fn create_user(&self, name: &str, email: &str) -> Result<User, String> {
        let user = User {
            id: self.users.lock().unwrap().len() as u64 + 1,
            name: name.to_string(),
            email: email.to_string(),
            created_at: chrono::Local::now().to_rfc3339(),
        };

        self.users.lock().unwrap().push(user.clone());
        println!("➕ User created: {}", user.id);
        Ok(user)
    }

    pub fn get_user(&self, id: u64) -> Result<User, String> {
        let users = self.users.lock().unwrap();
        users.iter()
            .find(|u| u.id == id)
            .cloned()
            .ok_or_else(|| format!("User {} not found", id))
    }

    pub fn list_users(&self) -> Vec<User> {
        self.users.lock().unwrap().clone()
    }

    pub fn health_check(&self) -> String {
        format!(r#"{{"status": "healthy", "port": {}, "users": {}}}"#,
            self.port,
            self.users.lock().unwrap().len())
    }
}

// ============================================================================
// ML SERVICE (Sylva - Machine Learning)
// ============================================================================

pub struct MlService {
    models: std::sync::Mutex<std::collections::HashMap<String, ModelInfo>>,
}

#[derive(Debug, Clone)]
pub struct ModelInfo {
    pub id: String,
    pub version: String,
    pub accuracy: f64,
    pub trained_at: String,
}

impl MlService {
    pub fn new() -> Self {
        MlService {
            models: std::sync::Mutex::new(std::collections::HashMap::new()),
        }
    }

    pub fn register_model(&self, id: &str, version: &str, accuracy: f64) -> Result<(), String> {
        let mut models = self.models.lock().unwrap();
        models.insert(id.to_string(), ModelInfo {
            id: id.to_string(),
            version: version.to_string(),
            accuracy,
            trained_at: chrono::Local::now().to_rfc3339(),
        });
        println!("🧠 Model registered: {} (accuracy: {:.2}%)", id, accuracy * 100.0);
        Ok(())
    }

    pub fn predict(&self, model_id: &str, features: Vec<f64>) -> Result<PredictionResponse, String> {
        let models = self.models.lock().unwrap();
        let model = models.get(model_id)
            .ok_or_else(|| format!("Model {} not found", model_id))?;

        // Simple prediction: average of features
        let prediction = features.iter().sum::<f64>() / features.len() as f64;
        let confidence = model.accuracy;

        println!("🎯 Prediction from {}: {} (confidence: {:.2}%)",
            model_id, prediction, confidence * 100.0);

        Ok(PredictionResponse {
            prediction,
            confidence,
            model_version: model.version.clone(),
        })
    }

    pub fn get_model_info(&self, id: &str) -> Result<ModelInfo, String> {
        let models = self.models.lock().unwrap();
        models.get(id)
            .cloned()
            .ok_or_else(|| format!("Model {} not found", id))
    }
}

// ============================================================================
// CACHE SERVICE (Aether - Distributed Systems)
// ============================================================================

pub struct CacheService {
    cache: Arc<std::sync::Mutex<std::collections::HashMap<String, CacheEntry>>>,
}

#[derive(Debug, Clone)]
pub struct CacheEntry {
    pub value: String,
    pub ttl_seconds: u64,
    pub created_at: std::time::Instant,
}

impl CacheService {
    pub fn new() -> Self {
        CacheService {
            cache: Arc::new(std::sync::Mutex::new(std::collections::HashMap::new())),
        }
    }

    pub fn set(&self, key: &str, value: &str, ttl: u64) -> Result<(), String> {
        let mut cache = self.cache.lock().unwrap();
        cache.insert(key.to_string(), CacheEntry {
            value: value.to_string(),
            ttl_seconds: ttl,
            created_at: std::time::Instant::now(),
        });
        println!("💾 Cached: {} (TTL: {}s)", key, ttl);
        Ok(())
    }

    pub fn get(&self, key: &str) -> Result<String, String> {
        let cache = self.cache.lock().unwrap();
        if let Some(entry) = cache.get(key) {
            if entry.created_at.elapsed().as_secs() < entry.ttl_seconds {
                println!("⚡ Cache hit: {}", key);
                return Ok(entry.value.clone());
            }
        }
        Err(format!("Cache miss: {}", key))
    }

    pub fn invalidate(&self, key: &str) {
        let mut cache = self.cache.lock().unwrap();
        cache.remove(key);
        println!("🗑️  Cache invalidated: {}", key);
    }

    pub fn stats(&self) -> CacheStats {
        let cache = self.cache.lock().unwrap();
        CacheStats {
            entries: cache.len(),
            hits: 0, // Would track separately in production
        }
    }
}

#[derive(Debug)]
pub struct CacheStats {
    pub entries: usize,
    pub hits: u64,
}

// ============================================================================
// INTEGRATED WEB APPLICATION
// ============================================================================

pub struct WebApplication {
    api: Arc<ApiService>,
    ml: Arc<MlService>,
    cache: Arc<CacheService>,
}

impl WebApplication {
    pub fn new(port: u16) -> Self {
        WebApplication {
            api: Arc::new(ApiService::new(port)),
            ml: Arc::new(MlService::new()),
            cache: Arc::new(CacheService::new()),
        }
    }

    pub fn handle_create_user(&self, name: &str, email: &str) -> Result<String, String> {
        let user = self.api.create_user(name, email)?;

        // Cache the user
        let user_json = format!(r#"{{"id": {}, "name": "{}", "email": "{}"}}"#,
            user.id, user.name, user.email);
        let cache_key = format!("user:{}", user.id);
        let _ = self.cache.set(&cache_key, &user_json, 3600);

        Ok(format!("User created: {}", user.id))
    }

    pub fn handle_get_user(&self, id: u64) -> Result<String, String> {
        // Try cache first
        let cache_key = format!("user:{}", id);
        if let Ok(cached) = self.cache.get(&cache_key) {
            return Ok(cached);
        }

        // Fetch from API
        let user = self.api.get_user(id)?;
        let user_json = format!(r#"{{"id": {}, "name": "{}", "email": "{}"}}"#,
            user.id, user.name, user.email);

        // Cache for future requests
        let _ = self.cache.set(&cache_key, &user_json, 3600);

        Ok(user_json)
    }

    pub fn handle_prediction(&self, model_id: &str, features: Vec<f64>) -> Result<PredictionResponse, String> {
        // Get prediction from ML service
        self.ml.predict(model_id, features)
    }

    pub fn handle_health(&self) -> String {
        format!(r#"{{
            "status": "operational",
            "api": {},
            "ml_models": {},
            "cache": {{"entries": {}}}
        }}"#,
            self.api.health_check(),
            1, // Number of registered models
            self.cache.stats().entries)
    }
}

// ============================================================================
// EXAMPLE EXECUTION
// ============================================================================

pub fn run_example() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n🌐 OMNISYSTEM WEB APPLICATION EXAMPLE");
    println!("====================================\n");

    // Initialize application
    let app = WebApplication::new(8080);

    // Register ML model (Sylva)
    println!("📊 Step 1: Register ML Model");
    app.ml.register_model("user-predictor", "1.0.0", 0.95)?;
    println!();

    // Create users (Titan API)
    println!("👥 Step 2: Create Users");
    app.handle_create_user("Alice", "alice@example.com")?;
    app.handle_create_user("Bob", "bob@example.com")?;
    println!();

    // Get user (with caching via Aether)
    println!("🔍 Step 3: Get User (cached)");
    println!("First request:");
    app.handle_get_user(1)?;
    println!("Second request (from cache):");
    app.handle_get_user(1)?;
    println!();

    // Make predictions
    println!("🎯 Step 4: Make Predictions (Sylva)");
    let features = vec![1.5, 2.0, 3.5, 0.8];
    let pred = app.handle_prediction("user-predictor", features)?;
    println!("Prediction: {} (confidence: {:.2}%)\n", pred.prediction, pred.confidence * 100.0);

    // Health check
    println!("💚 Step 5: Health Check");
    println!("{}\n", app.handle_health());

    // List all users
    println!("📋 Step 6: List All Users");
    for user in app.api.list_users() {
        println!("  - User {}: {} ({})", user.id, user.name, user.email);
    }
    println!();

    println!("✅ Web Application Example Complete\n");

    Ok(())
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_api_service() {
        let api = ApiService::new(8080);
        let user = api.create_user("Test", "test@example.com").unwrap();
        assert_eq!(user.name, "Test");
        assert!(api.get_user(user.id).is_ok());
    }

    #[test]
    fn test_ml_service() {
        let ml = MlService::new();
        assert!(ml.register_model("test", "1.0", 0.9).is_ok());
        let pred = ml.predict("test", vec![1.0, 2.0, 3.0]).unwrap();
        assert!(pred.prediction > 0.0);
    }

    #[test]
    fn test_cache_service() {
        let cache = CacheService::new();
        cache.set("key", "value", 60).unwrap();
        assert!(cache.get("key").is_ok());
        cache.invalidate("key");
        assert!(cache.get("key").is_err());
    }

    #[test]
    fn test_web_application() {
        let app = WebApplication::new(8080);
        let result = app.handle_create_user("Test", "test@example.com");
        assert!(result.is_ok());
    }
}
