pub fn url_encode(s: &str) -> String {
    let mut result = String::with_capacity(s.len() * 3);
    for c in s.chars() {
        match c {
            ':' | '/' | '?' | '#' | '[' | ']' | '@' | '!' | '$' | '&' | '\'' | '(' | ')' | '*'
            | '+' | ',' | ';' | '=' | '%' => {
                result.push_str(&format!("%{:02X}", c as u8));
            }
            _ => result.push(c),
        }
    }
    result
}
