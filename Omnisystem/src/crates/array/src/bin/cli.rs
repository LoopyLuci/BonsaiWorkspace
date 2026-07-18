//! Array CLI - evaluates an APL/J-style expression from argv (or a default demo)

use array::AplEval;

fn main() {
    let expr = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "+/ [1, 2, 3, 4]".to_string());

    match AplEval::eval(&expr) {
        Ok(result) => println!("{expr} => {result}"),
        Err(e) => eprintln!("error evaluating {expr:?}: {e}"),
    }
}
