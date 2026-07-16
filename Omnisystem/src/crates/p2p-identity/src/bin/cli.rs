//! CLI that exercises NodeId identity proof, DID documents, and
//! verifiable credentials end to end.

use p2p_identity::{DidDocument, NodeId};

fn main() {
    let (node_id, secret) = NodeId::generate();
    println!("NodeId: {node_id}");

    let challenge = b"prove you own this identity";
    let proof = node_id.prove(&secret, challenge);
    println!(
        "Proof verifies: {}",
        node_id.verify_proof(challenge, &proof)
    );
    println!(
        "Tampered proof verifies: {}",
        node_id.verify_proof(challenge, b"not a real signature")
    );

    let did = DidDocument::from_node_id(&node_id);
    println!("DID: {}", did.id);
    println!("Verification methods: {}", did.verification_method.len());
}
