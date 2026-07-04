use serde_json::Value;

pub struct AppRegistryClient {
    api_url: String,
    client: reqwest::Client,
}

impl AppRegistryClient {
    pub fn new(api_url: String) -> Self {
        Self {
            api_url,
            client: reqwest::Client::new(),
        }
    }

    pub async fn get_all_apps(&self) -> anyhow::Result<Vec<Value>> {
        let url = format!("{}/apps", self.api_url);
        let response = self
            .client
            .get(&url)
            .timeout(std::time::Duration::from_secs(5))
            .send()
            .await?;

        let apps = response.json::<Vec<Value>>().await?;
        Ok(apps)
    }

    pub async fn get_app(&self, app_id: &str) -> anyhow::Result<Value> {
        let url = format!("{}/apps/{}", self.api_url, app_id);
        let response = self
            .client
            .get(&url)
            .timeout(std::time::Duration::from_secs(5))
            .send()
            .await?;

        let app = response.json::<Value>().await?;
        Ok(app)
    }

    pub async fn search_apps(&self, query: &str) -> anyhow::Result<Vec<Value>> {
        let url = format!("{}/apps/search?q={}", self.api_url, query);
        let response = self
            .client
            .get(&url)
            .timeout(std::time::Duration::from_secs(5))
            .send()
            .await?;

        let apps = response.json::<Vec<Value>>().await?;
        Ok(apps)
    }

    pub async fn get_featured_apps(&self) -> anyhow::Result<Vec<Value>> {
        let url = format!("{}/apps/featured", self.api_url);
        let response = self
            .client
            .get(&url)
            .timeout(std::time::Duration::from_secs(5))
            .send()
            .await?;

        let apps = response.json::<Vec<Value>>().await?;
        Ok(apps)
    }

    pub async fn launch_app(&self, app_id: &str) -> anyhow::Result<()> {
        let url = format!("{}/apps/{}/launch", self.api_url, app_id);
        self.client
            .post(&url)
            .timeout(std::time::Duration::from_secs(10))
            .send()
            .await?;

        Ok(())
    }

    pub async fn get_app_by_category(&self, category: &str) -> anyhow::Result<Vec<Value>> {
        let url = format!("{}/apps?category={}", self.api_url, category);
        let response = self
            .client
            .get(&url)
            .timeout(std::time::Duration::from_secs(5))
            .send()
            .await?;

        let apps = response.json::<Vec<Value>>().await?;
        Ok(apps)
    }
}
