use crate::extract::layout::PageElement;

/// Render page elements as plain text.
pub fn elements_to_txt(elements: &[PageElement]) -> String {
    let mut out = String::new();

    for element in elements {
        match element {
            PageElement::Heading { text, .. } => {
                out.push_str(text);
                out.push_str("\n\n");
            }
            PageElement::Paragraph { text } => {
                out.push_str(text);
                out.push_str("\n\n");
            }
            PageElement::Table { table } => {
                out.push_str(&table.to_text());
                out.push_str("\n\n");
            }
        }
    }

    // Trim trailing whitespace
    let trimmed = out.trim_end().to_string();
    if trimmed.is_empty() {
        trimmed
    } else {
        trimmed + "\n"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::extract::Table;

    #[test]
    fn test_heading_and_paragraph() {
        let elements = vec![
            PageElement::Heading {
                level: 1,
                text: "Hello World".to_string(),
            },
            PageElement::Paragraph {
                text: "This is a paragraph.".to_string(),
            },
        ];

        let txt = elements_to_txt(&elements);
        assert_eq!(txt, "Hello World\n\nThis is a paragraph.\n");
    }

    #[test]
    fn test_table_element() {
        let table = Table {
            rows: vec![
                vec!["A".to_string(), "B".to_string()],
                vec!["1".to_string(), "2".to_string()],
            ],
            num_columns: 2,
        };

        let elements = vec![PageElement::Table { table }];
        let txt = elements_to_txt(&elements);
        assert!(txt.contains("A"));
        assert!(txt.contains("B"));
    }

    #[test]
    fn test_empty_elements() {
        let txt = elements_to_txt(&[]);
        assert_eq!(txt, "");
    }
}
