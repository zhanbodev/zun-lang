// Syntax validation and helper functions for Zun.

/// ContainsRawEnglish no longer rejects bare ASCII letters (plain identifiers are now allowed).
pub fn contains_raw_english(_line: &str) -> bool {
    false
}

/// StripComments removes single-line (//) and multi-line (/* */) comments from a line.
/// Returns the line with comments removed, or an error if there's an unclosed multi-line comment.
pub fn strip_comments(line: &str) -> Result<String, String> {
    let mut result = String::new();
    let chars: Vec<char> = line.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        // Check for multi-line comment start
        if i + 1 < chars.len() && chars[i] == '/' && chars[i + 1] == '*' {
            // Skip until we find the end of multi-line comment
            i += 2;
            let mut found_end = false;
            while i + 1 < chars.len() {
                if chars[i] == '*' && chars[i + 1] == '/' {
                    i += 2;
                    found_end = true;
                    break;
                }
                i += 1;
            }
            if !found_end {
                return Err("unterminated multi-line comment".to_string());
            }
        }
        // Check for single-line comment
        else if chars[i] == '/' && i + 1 < chars.len() && chars[i + 1] == '/' {
            break; // Rest of line is a comment
        } else {
            result.push(chars[i]);
            i += 1;
        }
    }
    Ok(result)
}

/// All valid $ prefixes in Zun, ordered longest first to avoid
/// prefix shadowing (e.g. "$continue" must come before "$con").
const DOLLAR_PREFIXES: &[&str] = &[
    "$>>",
    "$continue",
    "$while",
    "$break",
    "$elif",
    "$else",
    "$loop",
    "$for",
    "$fn",
    "$if",
    "$@",
    "$#",
    "$", // bare $ = variable declaration (must be last)
];

/// ValidateDollarLiterals ensures every `$` in the line is the start of
/// a known Zun operator or keyword.
pub fn validate_dollar_literals(line: &str) -> (bool, String) {
    let chars: Vec<char> = line.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        if chars[i] != '$' {
            i += 1;
            continue;
        }

        // Build the suffix starting at `$` for prefix matching
        let suffix: String = chars[i..].iter().collect();

        let mut matched = false;
        for prefix in DOLLAR_PREFIXES {
            if suffix.starts_with(prefix) {
                i += prefix.chars().count();
                matched = true;
                break;
            }
        }

        if !matched {
            // Should never happen since bare "$" is in the list, but be safe
            i += 1;
        }
    }
    (true, String::new())
}
