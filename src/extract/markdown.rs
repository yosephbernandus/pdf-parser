use crate::extract::layout::PageElement;
use crate::extract::Table;

/// Render page elements as Markdown.
pub fn elements_to_markdown(elements: &[PageElement]) -> String {
    let mut out = String::new();

    for element in elements {
        match element {
            PageElement::Heading { level, text } => {
                let prefix = "#".repeat(*level as usize);
                out.push_str(&prefix);
                out.push(' ');
                out.push_str(text);
                out.push_str("\n\n");
            }
            PageElement::Paragraph { text } => {
                out.push_str(text);
                out.push_str("\n\n");
            }
            PageElement::Table { table } => {
                out.push_str(&table_to_markdown(table));
                out.push_str("\n\n");
            }
        }
    }

    let trimmed = out.trim_end().to_string();
    if trimmed.is_empty() {
        trimmed
    } else {
        trimmed + "\n"
    }
}

/// Convert a Table to a Markdown table string.
fn table_to_markdown(table: &Table) -> String {
    if table.rows.is_empty() {
        return String::new();
    }

    let mut out = String::new();

    // Compute column widths for alignment
    let mut widths: Vec<usize> = vec![3; table.num_columns]; // minimum width 3 for "---"
    for row in &table.rows {
        for (i, cell) in row.iter().enumerate() {
            if i < widths.len() {
                let escaped_len = escape_pipe(cell).chars().count();
                widths[i] = widths[i].max(escaped_len);
            }
        }
    }

    // Header row
    let header = &table.rows[0];
    out.push_str(&format_md_row(header, &widths));
    out.push('\n');

    // Separator row
    let sep: Vec<String> = widths.iter().map(|w| "-".repeat(*w)).collect();
    out.push('|');
    for s in &sep {
        out.push(' ');
        out.push_str(s);
        out.push_str(" |");
    }
    out.push('\n');

    // Data rows
    for row in table.rows.iter().skip(1) {
        out.push_str(&format_md_row(row, &widths));
        out.push('\n');
    }

    // Remove trailing newline (caller adds spacing)
    out.trim_end_matches('\n').to_string()
}

fn format_md_row(row: &[String], widths: &[usize]) -> String {
    let mut out = String::from("|");
    for (i, cell) in row.iter().enumerate() {
        let width = widths.get(i).copied().unwrap_or(3);
        let escaped = escape_pipe(cell);
        out.push_str(&format!(" {:<width$} |", escaped, width = width));
    }
    // Pad missing columns
    for i in row.len()..widths.len() {
        let width = widths[i];
        out.push_str(&format!(" {:<width$} |", "", width = width));
    }
    out
}

fn escape_pipe(s: &str) -> String {
    s.replace('|', "\\|")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_heading_levels() {
        let elements = vec![
            PageElement::Heading {
                level: 1,
                text: "Title".to_string(),
            },
            PageElement::Heading {
                level: 2,
                text: "Subtitle".to_string(),
            },
            PageElement::Heading {
                level: 3,
                text: "Section".to_string(),
            },
        ];

        let md = elements_to_markdown(&elements);
        assert!(md.contains("# Title"));
        assert!(md.contains("## Subtitle"));
        assert!(md.contains("### Section"));
    }

    #[test]
    fn test_paragraph() {
        let elements = vec![PageElement::Paragraph {
            text: "Hello world.".to_string(),
        }];
        let md = elements_to_markdown(&elements);
        assert_eq!(md, "Hello world.\n");
    }

    #[test]
    fn test_markdown_table() {
        let table = Table {
            rows: vec![
                vec!["Name".to_string(), "Age".to_string()],
                vec!["Alice".to_string(), "30".to_string()],
            ],
            num_columns: 2,
        };

        let elements = vec![PageElement::Table { table }];
        let md = elements_to_markdown(&elements);
        assert!(md.contains("| Name"));
        assert!(md.contains("| ---"));
        assert!(md.contains("| Alice"));
    }

    #[test]
    fn test_pipe_escaping() {
        assert_eq!(escape_pipe("a|b"), "a\\|b");
        assert_eq!(escape_pipe("normal"), "normal");
    }

    #[test]
    fn test_empty() {
        let md = elements_to_markdown(&[]);
        assert_eq!(md, "");
    }

    #[test]
    fn test_mixed_content_markdown() {
        let table = Table {
            rows: vec![
                vec!["Col1".to_string(), "Col2".to_string()],
                vec!["A".to_string(), "B".to_string()],
            ],
            num_columns: 2,
        };

        let elements = vec![
            PageElement::Heading {
                level: 1,
                text: "Report".to_string(),
            },
            PageElement::Paragraph {
                text: "Summary of data.".to_string(),
            },
            PageElement::Table { table },
        ];

        let md = elements_to_markdown(&elements);
        assert!(md.starts_with("# Report"));
        assert!(md.contains("Summary of data."));
        assert!(md.contains("| Col1"));
    }
}
