pub const TEMPLATE_SEPARATOR: &str = "|~";
pub const NO_LOCALIZATION_MARKER: &str = "[[NoI18n]]";
pub const SANITIZATION_MARKER: &str = "\u{E000}"; //Guaranteed reserved character

pub fn dont_localize(s: String) -> String {
    return format!("{NO_LOCALIZATION_MARKER}{s}");
}

pub fn dont_reformat(s: String) -> String {
    return s;
}

pub fn is_sanitized(s: &str) -> bool {
    return !s.contains(TEMPLATE_SEPARATOR);
}

pub fn sanitize_string(s: String) -> String {
    return s.replace(SANITIZATION_MARKER, "").replace(TEMPLATE_SEPARATOR, SANITIZATION_MARKER);
}

pub fn validate_chat_language_code(language_code: &str) -> Result<(), String> {
    if !(2..=16).contains(&language_code.len()) || !language_code.bytes().all(|b| b.is_ascii_alphabetic() || b == b'-') {
        return Err("Invalid chat language code".into());
    }

    Ok(())
}
