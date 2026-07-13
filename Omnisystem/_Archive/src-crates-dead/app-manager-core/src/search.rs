//! Advanced search and filtering capabilities

use crate::app::RegisteredApp;
use std::collections::HashMap;

/// Search result with relevance score
#[derive(Debug, Clone)]
pub struct SearchResult {
    pub app: RegisteredApp,
    pub relevance_score: f32,
}

impl PartialEq for SearchResult {
    fn eq(&self, other: &Self) -> bool {
        self.app.manifest.id == other.app.manifest.id
    }
}

impl Eq for SearchResult {}

/// Advanced search engine with relevance ranking
pub struct SearchEngine;

impl SearchEngine {
    /// Perform full-text search with relevance scoring
    pub fn search(apps: &[RegisteredApp], query: &str) -> Vec<SearchResult> {
        let query_lower = query.to_lowercase();
        let query_terms: Vec<&str> = query_lower.split_whitespace().collect();

        let mut results: Vec<SearchResult> = apps
            .iter()
            .filter_map(|app| {
                let score = Self::calculate_relevance(&query_terms, app);
                if score > 0.0 {
                    Some(SearchResult {
                        app: app.clone(),
                        relevance_score: score,
                    })
                } else {
                    None
                }
            })
            .collect();

        results.sort_by(|a, b| {
            b.relevance_score
                .partial_cmp(&a.relevance_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        results
    }

    /// Calculate relevance score for an app
    fn calculate_relevance(terms: &[&str], app: &RegisteredApp) -> f32 {
        let mut score = 0.0;

        let app_name_lower = app.manifest.name.to_lowercase();
        let app_desc_lower = app.manifest.description.to_lowercase();

        for term in terms {
            // Name match is most relevant (weight: 3.0)
            if app_name_lower.contains(term) {
                score += 3.0;
            }

            // Description match (weight: 1.5)
            if app_desc_lower.contains(term) {
                score += 1.5;
            }

            // Tag match (weight: 2.0)
            for tag in &app.manifest.tags {
                if tag.to_lowercase().contains(term) {
                    score += 2.0;
                    break;
                }
            }

            // Category match (weight: 1.0)
            for cat in &app.manifest.categories {
                if cat.to_lowercase().contains(term) {
                    score += 1.0;
                    break;
                }
            }
        }

        // Apply rating boost (normalized to 0.0-1.0)
        let rating_factor = 1.0 + (app.rating / 10.0);
        score *= rating_factor;

        // Apply download count boost (logarithmic)
        let download_factor = 1.0 + (app.download_count as f32).log10() / 5.0;
        score *= download_factor;

        score
    }

    /// Fuzzy search with typo tolerance
    pub fn fuzzy_search(apps: &[RegisteredApp], query: &str) -> Vec<SearchResult> {
        let query_lower = query.to_lowercase();

        let mut results: Vec<SearchResult> = apps
            .iter()
            .filter_map(|app| {
                let score = Self::fuzzy_score(&query_lower, app);
                if score > 0.5 {
                    Some(SearchResult {
                        app: app.clone(),
                        relevance_score: score,
                    })
                } else {
                    None
                }
            })
            .collect();

        results.sort_by(|a, b| {
            b.relevance_score
                .partial_cmp(&a.relevance_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        results
    }

    /// Calculate fuzzy match score using Levenshtein distance
    fn fuzzy_score(query: &str, app: &RegisteredApp) -> f32 {
        let app_name_lower = app.manifest.name.to_lowercase();

        let dist = Self::levenshtein_distance(query, &app_name_lower);
        let max_len = query.len().max(app_name_lower.len());

        if max_len == 0 {
            return 1.0;
        }

        let similarity = 1.0 - (dist as f32 / max_len as f32);
        (similarity * 100.0).max(0.0)
    }

    /// Calculate Levenshtein distance between two strings
    fn levenshtein_distance(s1: &str, s2: &str) -> usize {
        let len1 = s1.len();
        let len2 = s2.len();

        if len1 == 0 {
            return len2;
        }
        if len2 == 0 {
            return len1;
        }

        let mut matrix = vec![vec![0; len2 + 1]; len1 + 1];

        for i in 0..=len1 {
            matrix[i][0] = i;
        }

        for j in 0..=len2 {
            matrix[0][j] = j;
        }

        let s1_chars: Vec<char> = s1.chars().collect();
        let s2_chars: Vec<char> = s2.chars().collect();

        for i in 1..=len1 {
            for j in 1..=len2 {
                let cost = if s1_chars[i - 1] == s2_chars[j - 1] {
                    0
                } else {
                    1
                };

                matrix[i][j] = std::cmp::min(
                    std::cmp::min(
                        matrix[i - 1][j] + 1,      // deletion
                        matrix[i][j - 1] + 1,      // insertion
                    ),
                    matrix[i - 1][j - 1] + cost,   // substitution
                );
            }
        }

        matrix[len1][len2]
    }

    /// Get popularity metrics
    pub fn popularity_metrics(apps: &[RegisteredApp]) -> HashMap<String, f32> {
        let mut metrics = HashMap::new();

        if apps.is_empty() {
            return metrics;
        }

        let avg_rating = apps.iter().map(|a| a.rating).sum::<f32>() / apps.len() as f32;
        let avg_downloads = apps.iter().map(|a| a.download_count).sum::<u32>() as f32 / apps.len() as f32;
        let avg_reviews = apps.iter().map(|a| a.review_count).sum::<u32>() as f32 / apps.len() as f32;

        metrics.insert("avg_rating".to_string(), avg_rating);
        metrics.insert("avg_downloads".to_string(), avg_downloads);
        metrics.insert("avg_reviews".to_string(), avg_reviews);

        metrics
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::{AppManifest, PublisherId};

    fn create_test_app(name: &str, description: &str) -> RegisteredApp {
        let publisher = PublisherId::new();
        let mut manifest = AppManifest::new(
            name.to_string(),
            semver::Version::new(1, 0, 0),
            publisher,
        );
        manifest.icon_url = "icon.png".to_string();
        manifest.description = description.to_string();
        let mut app = RegisteredApp::new(manifest);
        app.rating = 4.5;
        app.download_count = 1000;
        app
    }

    #[test]
    fn test_search_by_name() {
        let app1 = create_test_app("Productivity", "A productivity app");
        let app2 = create_test_app("Games", "A game app");

        let apps = vec![app1, app2];
        let results = SearchEngine::search(&apps, "productivity");

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].app.manifest.name, "Productivity");
    }

    #[test]
    fn test_search_relevance_scoring() {
        let app1 = create_test_app("Editor", "A text editor");
        let app2 = create_test_app("Text Tools", "Text processing app");

        let apps = vec![app1, app2];
        let results = SearchEngine::search(&apps, "text editor");

        assert!(!results.is_empty());
        if results.len() > 1 {
            // First result should have higher relevance
            assert!(results[0].relevance_score >= results[1].relevance_score);
        }
    }

    #[test]
    fn test_fuzzy_search() {
        let app = create_test_app("Calculator", "A calculator app");
        let apps = vec![app];

        // Typo: "calcualtor" instead of "calculator"
        let results = SearchEngine::fuzzy_search(&apps, "calcualtor");
        assert!(!results.is_empty());
    }

    #[test]
    fn test_levenshtein_distance() {
        assert_eq!(SearchEngine::levenshtein_distance("cat", "cat"), 0);
        assert_eq!(SearchEngine::levenshtein_distance("cat", "car"), 1);
        assert_eq!(SearchEngine::levenshtein_distance("", "abc"), 3);
        assert_eq!(SearchEngine::levenshtein_distance("abc", ""), 3);
    }

    #[test]
    fn test_popularity_metrics() {
        let mut app1 = create_test_app("App1", "Description 1");
        app1.rating = 5.0;
        app1.download_count = 1000;
        app1.review_count = 100;

        let mut app2 = create_test_app("App2", "Description 2");
        app2.rating = 3.0;
        app2.download_count = 500;
        app2.review_count = 50;

        let apps = vec![app1, app2];
        let metrics = SearchEngine::popularity_metrics(&apps);

        assert!(metrics.contains_key("avg_rating"));
        assert!(metrics.contains_key("avg_downloads"));
        assert!(metrics.contains_key("avg_reviews"));

        assert!((metrics["avg_rating"] - 4.0).abs() < 0.01);
    }

    #[test]
    fn test_empty_search() {
        let apps = vec![];
        let results = SearchEngine::search(&apps, "test");
        assert_eq!(results.len(), 0);
    }

    #[test]
    fn test_search_no_matches() {
        let app = create_test_app("Games", "A game app");
        let apps = vec![app];

        let results = SearchEngine::search(&apps, "productivity");
        assert_eq!(results.len(), 0);
    }
}
