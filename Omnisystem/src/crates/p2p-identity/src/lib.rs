//! p2p-identity: self-sovereign identity primitives for the p2p mesh.
//!
//! A [`NodeId`] is self-certifying -- a peer's identity *is* its Ed25519
//! public key, with real sign/verify challenge-response proof of
//! ownership (no certificate authority). [`did::DidDocument`] wraps a
//! `NodeId` as a `did:key:` decentralized identifier, and
//! [`vc::VerifiableCredential`] carries claims with a real Ed25519
//! signature that verifies against the issuer's DID document.
//!
//! Distinct from the already-restored `p2p` crate (libp2p-based swarm/
//! onion/WebRTC transport): this crate is the identity/credential layer,
//! not networking.

pub mod did;
pub mod node_id;
pub mod vc;

pub use did::{DidDocument, VerificationMethod};
pub use node_id::NodeId;
pub use vc::{CredentialProof, VerifiableCredential};
