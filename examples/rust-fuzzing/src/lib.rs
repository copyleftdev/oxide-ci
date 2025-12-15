//! A simple library with functions to fuzz test.

use serde::{Deserialize, Serialize};

/// A user input that we want to parse safely.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct UserInput {
    pub name: String,
    pub age: u8,
    pub email: Option<String>,
}

/// Parse user input from JSON - potential fuzzing target.
pub fn parse_user_input(data: &[u8]) -> Result<UserInput, ParseError> {
    let s = std::str::from_utf8(data).map_err(|_| ParseError::InvalidUtf8)?;
    serde_json::from_str(s).map_err(|e| ParseError::JsonError(e.to_string()))
}

/// Validate email format - potential fuzzing target.
pub fn validate_email(email: &str) -> bool {
    if email.is_empty() || email.len() > 254 {
        return false;
    }
    
    let parts: Vec<&str> = email.split('@').collect();
    if parts.len() != 2 {
        return false;
    }
    
    let local = parts[0];
    let domain = parts[1];
    
    !local.is_empty() 
        && !domain.is_empty() 
        && domain.contains('.')
        && !local.starts_with('.')
        && !local.ends_with('.')
        && !domain.starts_with('.')
        && !domain.ends_with('.')
}

/// Calculate a checksum - potential integer overflow target.
pub fn calculate_checksum(data: &[u8]) -> u32 {
    data.iter().fold(0u32, |acc, &byte| acc.wrapping_add(byte as u32))
}

/// Parse a simple key=value config format.
pub fn parse_config(input: &str) -> Result<Vec<(String, String)>, ParseError> {
    let mut result = Vec::new();
    
    for line in input.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        
        let parts: Vec<&str> = line.splitn(2, '=').collect();
        if parts.len() != 2 {
            return Err(ParseError::InvalidFormat(format!("Invalid line: {}", line)));
        }
        
        result.push((parts[0].trim().to_string(), parts[1].trim().to_string()));
    }
    
    Ok(result)
}

#[derive(Debug, Clone, PartialEq)]
pub enum ParseError {
    InvalidUtf8,
    JsonError(String),
    InvalidFormat(String),
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ParseError::InvalidUtf8 => write!(f, "Invalid UTF-8"),
            ParseError::JsonError(e) => write!(f, "JSON error: {}", e),
            ParseError::InvalidFormat(e) => write!(f, "Invalid format: {}", e),
        }
    }
}

impl std::error::Error for ParseError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_valid_user() {
        let json = br#"{"name": "Alice", "age": 30, "email": "alice@example.com"}"#;
        let user = parse_user_input(json).unwrap();
        assert_eq!(user.name, "Alice");
        assert_eq!(user.age, 30);
    }

    #[test]
    fn test_validate_email_valid() {
        assert!(validate_email("user@example.com"));
        assert!(validate_email("user.name@example.co.uk"));
    }

    #[test]
    fn test_validate_email_invalid() {
        assert!(!validate_email(""));
        assert!(!validate_email("noatsign"));
        assert!(!validate_email("@nodomain"));
        assert!(!validate_email("nolocal@"));
        assert!(!validate_email(".leading@example.com"));
    }

    #[test]
    fn test_checksum() {
        assert_eq!(calculate_checksum(b"hello"), 532);
        assert_eq!(calculate_checksum(b""), 0);
    }

    #[test]
    fn test_parse_config() {
        let config = "key1=value1\nkey2=value2\n# comment\nkey3=value with spaces";
        let parsed = parse_config(config).unwrap();
        assert_eq!(parsed.len(), 3);
        assert_eq!(parsed[0], ("key1".to_string(), "value1".to_string()));
    }
}
