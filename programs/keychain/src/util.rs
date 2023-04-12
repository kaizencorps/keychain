
// checks that a given string contains only lowercase letters and numbers, with a few special characters
pub fn is_valid_name(s: &str) -> bool {
    s.chars().all(|c| !c.is_whitespace()  && (c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' || c == '_'))
}


