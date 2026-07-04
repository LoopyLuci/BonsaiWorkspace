//! Capability Registry — agents advertise their skills; managers query for matches.

use std::sync::Arc;

use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::role::{AgentProfile, Capability};

/// Query filter passed to `CapabilityRegistry::find`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CapabilityQuery {
    /// All of these capabilities must be present.
    pub requires_all: Vec<Capability>,
    /// At least one of these capabilities must be present.
    pub requires_any: Vec<Capability>,
    /// Maximum acceptable load (0.0–1.0). None = no limit.
    pub max_load: Option<f64>,
    /// Maximum cost per minute. None = no limit.
    pub max_cost_per_minute: Option<f64>,
    /// Minimum reliability. None = no limit.
    pub min_reliability: Option<f64>,
    /// If true, only local agents (not remote).
    pub local_only: bool,
}

/// Scored match returned from a query.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentMatch {
    pub profile: AgentProfile,
    /// Higher is better. Combines reliability, load, and cost.
    pub score: f64,
}

/// Global capability registry backed by a concurrent hash map.
/// Agents register on spawn and deregister on shutdown.
#[derive(Clone)]
pub struct CapabilityRegistry {
    inner: Arc<DashMap<Uuid, AgentProfile>>,
}

impl CapabilityRegistry {
    pub fn new() -> Self {
        Self { inner: Arc::new(DashMap::new()) }
    }

    /// Register or update an agent's profile.
    pub fn register(&self, profile: AgentProfile) {
        self.inner.insert(profile.agent_id, profile);
    }

    /// Remove an agent from the registry.
    pub fn deregister(&self, agent_id: Uuid) {
        self.inner.remove(&agent_id);
    }

    /// Update the load factor for an agent.
    pub fn update_load(&self, agent_id: Uuid, load: f64) {
        if let Some(mut entry) = self.inner.get_mut(&agent_id) {
            entry.current_load = load.clamp(0.0, 1.0);
        }
    }

    /// Find agents matching the query, sorted by score descending.
    pub fn find(&self, query: &CapabilityQuery) -> Vec<AgentMatch> {
        let mut matches: Vec<AgentMatch> = self
            .inner
            .iter()
            .filter_map(|entry| {
                let p = entry.value();

                // local_only filter
                if query.local_only && p.is_remote {
                    return None;
                }

                // max_load filter
                if let Some(max) = query.max_load {
                    if p.current_load > max {
                        return None;
                    }
                }

                // max_cost filter
                if let Some(max_cost) = query.max_cost_per_minute {
                    if p.cost_per_minute > max_cost {
                        return None;
                    }
                }

                // min_reliability filter
                if let Some(min_rel) = query.min_reliability {
                    if p.reliability < min_rel {
                        return None;
                    }
                }

                // requires_all: every capability in the list must be present
                for cap in &query.requires_all {
                    if !p.capabilities.contains(cap) {
                        return None;
                    }
                }

                // requires_any: at least one must be present (skip check if empty)
                if !query.requires_any.is_empty()
                    && !query.requires_any.iter().any(|c| p.capabilities.contains(c))
                {
                    return None;
                }

                // Score: reliability * (1 - load) / (1 + cost)
                let score = p.reliability * (1.0 - p.current_load) / (1.0 + p.cost_per_minute);

                Some(AgentMatch { profile: p.clone(), score })
            })
            .collect();

        matches.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
        matches
    }

    /// Return the best single match, or None if no agents qualify.
    pub fn best(&self, query: &CapabilityQuery) -> Option<AgentMatch> {
        self.find(query).into_iter().next()
    }

    /// All registered profiles (for dashboard display).
    pub fn list_all(&self) -> Vec<AgentProfile> {
        self.inner.iter().map(|e| e.value().clone()).collect()
    }

    pub fn count(&self) -> usize {
        self.inner.len()
    }
}

impl Default for CapabilityRegistry {
    fn default() -> Self {
        Self::new()
    }
}
