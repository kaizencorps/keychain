
// checks that a given string contains only lowercase letters and numbers
pub fn is_lowercase_num_no_space(s: &str) -> bool {
    s.chars().all(|c| !c.is_whitespace()  && (c.is_ascii_lowercase() || c.is_ascii_digit()))
}
