//! CLI demo for omnisystem-aether: builds a real schema, exercises the
//! reactive LiveSet collection, and sends a message through an actor
//! mailbox.

use omnisystem_aether::actor::Mailbox;
use omnisystem_aether::database::reactive::{LiveSet, SetDelta};
use omnisystem_aether::database::schema::{EntityType, Field, FieldConstraints, FieldType, Schema};

fn main() {
    let mut schema = Schema::new("demo".to_string());
    schema.add_entity(EntityType {
        name: "User".to_string(),
        fields: vec![Field {
            name: "id".to_string(),
            field_type: FieldType::Uuid,
            nullable: false,
            default_value: None,
            constraints: FieldConstraints {
                unique: true,
                indexed: true,
                vector_index: None,
            },
        }],
        indexes: vec![],
        relationships: vec![],
        temporal: None,
    });
    println!("Schema SQL:\n{}", schema.to_sql());

    let live_users: LiveSet<String> = LiveSet::new(vec!["alice".to_string()]);
    live_users.observe(|delta: SetDelta<String>| {
        println!("LiveSet delta: +{:?} -{:?}", delta.added, delta.removed);
    });
    live_users.notify(SetDelta {
        added: vec!["bob".to_string()],
        removed: vec![],
        modified: vec![],
    });
    println!("LiveSet snapshot: {:?}", live_users.snapshot());

    let mailbox = Mailbox::new(8);
    mailbox.send(b"hello actor".to_vec()).unwrap();
    println!("Mailbox received: {:?}", mailbox.recv().map(|m| String::from_utf8_lossy(&m).to_string()));
}
