//! Decentralized Identifiers (DIDs) for Self-Sovereign Identity

use serde::{Serialize, Deserialize};

/// A minimal DID document for self-certifying identity.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DidDocument {
    pub id: String,
    pub verification_method: Vec<VerificationMethod>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationMethod {
    pub id: String,
    pub controller: String,
    pub public_key_bytes: [u8; 32],
}

impl DidDocument {
    /// Create a DID document from a NodeId.
    pub fn from_node_id(node_id: &crate::NodeId) -> Self {
        let did = format!("did:key:{}", node_id.to_hex());
        Self {
            id: did.clone(),
            verification_method: vec![VerificationMethod {
                id: format!("{}#keys-1", did),
                controller: did,
                public_key_bytes: node_id.0,
            }],
        }
    }

    /// Find a verification method by ID.
    pub fn find_method(&self, method_id: &str) -> Option<&VerificationMethod> {
        self.verification_method.iter().find(|m| m.id == method_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_did_from_node_id() {
        let (node_id, _) = crate::NodeId::generate();
        let did = DidDocument::from_node_id(&node_id);
        assert!(did.id.starts_with("did:key:"));
        assert_eq!(did.verification_method.len(), 1);
    }
}
