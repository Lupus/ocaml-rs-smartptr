use regex::Regex;

use crate::registry;

/// Helper function to extract the core type name.
fn extract_type_name(type_str: &str) -> &str {
    let re = Regex::new(r"::(\w+)(<|$)").unwrap();
    if let Some(captures) = re.captures(type_str) {
        return captures.get(1).unwrap().as_str();
    }
    let segments: Vec<&str> = type_str.split(':').collect();
    segments.last().copied().unwrap_or(type_str)
}

/// Helper function to capture segments until the core type.
fn capture_segments(type_str: &str) -> Vec<&str> {
    let re = Regex::new(r"[^:<]+").unwrap();
    let segments: Vec<&str> = re.find_iter(type_str).map(|mat| mat.as_str()).collect();

    let core_type = extract_type_name(type_str);
    let index = segments
        .iter()
        .position(|&s| s == core_type)
        .unwrap_or(segments.len() - 1);

    segments[..=index].to_vec()
}

/// Convert a module path to snake_case.
fn convert_to_snake_case(segment: &str) -> String {
    segment
        .chars()
        .enumerate()
        .map(|(i, c)| {
            if i > 0 && c.is_uppercase() {
                format!("_{}", c.to_lowercase())
            } else {
                c.to_lowercase().to_string()
            }
        })
        .collect::<String>()
}

/// Function to capitalize the first letter.
fn capitalize_first_letter(s: &str) -> String {
    let mut chars = s.chars();
    if let Some(first_char) = chars.next() {
        format!("{}{}", first_char.to_uppercase(), chars.collect::<String>())
    } else {
        String::new()
    }
}

/// Function to return the core type name.
pub(crate) fn get_type_name<T: ?Sized + 'static>() -> String {
    let type_info = registry::get_type_info::<T>();
    extract_type_name(type_info.fq_name).to_string()
}

/// Function to return the fully qualified name as Snake_cased with the first letter capitalized.
pub(crate) fn snake_case_of_fully_qualified_name(type_str: &str) -> String {
    let segments = capture_segments(type_str);
    let snake_cased = segments
        .into_iter()
        .map(convert_to_snake_case)
        .collect::<Vec<String>>()
        .join("_");
    capitalize_first_letter(&snake_cased)
}
