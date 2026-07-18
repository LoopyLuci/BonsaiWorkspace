//! CLI — build a `BonsaiFrame` from JSON columns, run a couple of
//! transformations, and print its schema + a JSON preview.

use dataframe::{BonsaiFrame, FilterExpr, Scalar};
use serde_json::json;

fn main() {
    let cols = vec![
        (
            "name".to_string(),
            vec![
                json!("Alice"),
                json!("Bob"),
                json!("Carol"),
                json!("Dave"),
            ],
        ),
        (
            "score".to_string(),
            vec![json!(88i64), json!(72i64), json!(95i64), json!(60i64)],
        ),
        (
            "pass".to_string(),
            vec![json!(true), json!(true), json!(true), json!(false)],
        ),
    ];

    let frame = BonsaiFrame::from_json_columns(&cols).expect("build frame from json columns");

    println!("schema: {}", frame.schema_json());

    let passing = frame
        .filter(&FilterExpr::Gt {
            col: "score".into(),
            val: Scalar::Int(80),
        })
        .expect("filter frame");

    println!(
        "rows with score > 80: {}",
        serde_json::to_string_pretty(&passing.to_json_rows().expect("to_json_rows")).unwrap()
    );
}
