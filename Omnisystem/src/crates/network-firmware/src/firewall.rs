use dashmap::DashMap;
use std::net::IpAddr;
use std::sync::Arc;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuleAction {
    Allow,
    Deny,
    Drop,
}

#[derive(Debug, Clone)]
pub struct FirewallRule {
    pub src_ip: IpAddr,
    pub dst_ip: IpAddr,
    pub port: u16,
    pub action: RuleAction,
}

pub struct FirewallManager {
    rules: Arc<DashMap<String, FirewallRule>>,
}

impl FirewallManager {
    pub fn new() -> Self {
        Self {
            rules: Arc::new(DashMap::new()),
        }
    }

    pub fn add_rule(&self, rule_id: String, rule: FirewallRule) {
        self.rules.insert(rule_id, rule);
    }

    pub fn check_rule(&self, rule_id: &str) -> Option<FirewallRule> {
        self.rules.get(rule_id).map(|r| r.clone())
    }

    pub fn evaluate_packet(&self, src: IpAddr, dst: IpAddr, port: u16) -> RuleAction {
        for entry in self.rules.iter() {
            let rule = entry.value();
            if rule.src_ip == src && rule.dst_ip == dst && rule.port == port {
                return rule.action;
            }
        }
        RuleAction::Deny
    }

    pub fn rule_count(&self) -> usize {
        self.rules.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_firewall_rule() {
        let fw = FirewallManager::new();
        let rule = FirewallRule {
            src_ip: "192.168.1.1".parse().unwrap(),
            dst_ip: "10.0.0.1".parse().unwrap(),
            port: 443,
            action: RuleAction::Allow,
        };
        fw.add_rule("rule1".to_string(), rule);
        assert_eq!(fw.rule_count(), 1);
    }

    #[test]
    fn test_firewall_evaluation() {
        let fw = FirewallManager::new();
        let rule = FirewallRule {
            src_ip: "192.168.1.1".parse().unwrap(),
            dst_ip: "10.0.0.1".parse().unwrap(),
            port: 443,
            action: RuleAction::Allow,
        };
        fw.add_rule("rule1".to_string(), rule);
        let action = fw.evaluate_packet(
            "192.168.1.1".parse().unwrap(),
            "10.0.0.1".parse().unwrap(),
            443,
        );
        assert_eq!(action, RuleAction::Allow);
    }
}
