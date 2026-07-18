use colored::*;

pub fn print_message(msg: &str) {
    println!("{}", msg.green());
}

pub fn print_error(msg: &str) {
    eprintln!("{}", msg.red());
}

pub fn print_warning(msg: &str) {
    println!("{}", msg.yellow());
}

pub fn print_info(msg: &str) {
    println!("{}", msg.cyan());
}

pub fn print_success(msg: &str) {
    println!("{} {}", "✓".green(), msg.green());
}

pub fn print_failure(msg: &str) {
    println!("{} {}", "✗".red(), msg.red());
}

pub fn print_table(headers: &[&str], rows: Vec<Vec<String>>) {
    let col_width = 20;
    let header_line = headers
        .iter()
        .map(|h| format!("{:<width$}", h, width = col_width))
        .collect::<Vec<_>>()
        .join("|");

    println!("{}", header_line.bold());
    println!("{}", "-".repeat(header_line.len()));

    for row in rows {
        let row_line = row
            .iter()
            .map(|cell| format!("{:<width$}", cell, width = col_width))
            .collect::<Vec<_>>()
            .join("|");
        println!("{}", row_line);
    }
}
