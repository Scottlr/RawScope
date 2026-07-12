//! Context-specific escaping for current Markdown evidence writers.

pub(crate) fn escape_table_cell(value: &str) -> String {
    value
        .chars()
        .map(|ch| match ch {
            '|' => "\\|".to_string(),
            '\r' | '\n' => " ".to_string(),
            '`' => "\\`".to_string(),
            _ if ch.is_control() => "�".to_string(),
            _ => ch.to_string(),
        })
        .collect()
}

#[allow(dead_code)]
pub(crate) fn escape_inline_code(value: &str) -> String {
    value.replace('`', "\\`").replace(['\r', '\n'], " ")
}

pub(crate) fn portable_path_label(path: &std::path::Path) -> String {
    path.file_name()
        .and_then(|name| name.to_str())
        .filter(|name| !name.is_empty())
        .unwrap_or("dataset")
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::{escape_inline_code, escape_table_cell};

    #[test]
    fn table_cells_escape_structural_input() {
        assert_eq!(escape_table_cell("a|b`c\nd"), "a\\|b\\`c d");
    }

    #[test]
    fn inline_code_removes_line_breaks() {
        assert_eq!(escape_inline_code("a`\nb"), "a\\` b");
    }
}
