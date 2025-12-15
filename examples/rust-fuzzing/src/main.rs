//! Simple Hello World app that uses the library.

use rust_fuzzing_example::{parse_user_input, validate_email};

fn main() {
    println!("=== Rust Fuzzing Example ===\n");

    // Demo: Parse user input
    let json = br#"{"name": "Alice", "age": 30, "email": "alice@example.com"}"#;
    match parse_user_input(json) {
        Ok(user) => println!("Parsed user: {:?}", user),
        Err(e) => println!("Parse error: {}", e),
    }

    // Demo: Validate emails
    let emails = ["alice@example.com", "invalid", "bob@test.org"];
    for email in emails {
        let valid = if validate_email(email) { "✓" } else { "✗" };
        println!("{} {}", valid, email);
    }

    println!("\nHello, secure world!");
}
