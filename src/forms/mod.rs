pub mod categories;
pub mod main;
pub mod price_levels;
pub mod products;
pub mod tags;

fn sanitize_text(text: &str) -> Option<String> {
    let text = text.trim();
    if text.is_empty() {
        None
    } else {
        Some(text.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize_text() {
        assert_eq!(sanitize_text("   test   "), Some("test".to_string()));
        assert!(sanitize_text("").is_none());
        assert!(sanitize_text("    ").is_none());
    }
}
