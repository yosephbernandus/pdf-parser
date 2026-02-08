use crate::content::TextSpan;
use crate::extract::Table;

/// A classified page element
#[derive(Debug, Clone)]
pub enum PageElement {
    Heading { level: u8, text: String },
    Paragraph { text: String },
    Table { table: Table },
}

/// Classify text spans into structured page elements (headings, paragraphs, tables).
pub fn classify_spans(spans: Vec<TextSpan>) -> Vec<PageElement> {
    let spans: Vec<_> = spans
        .into_iter()
        .filter(|s| !s.text.trim().is_empty())
        .collect();

    if spans.is_empty() {
        return Vec::new();
    }

    let avg_font_size =
        spans.iter().map(|s| s.font_size).sum::<f64>() / spans.len() as f64;
    let row_tolerance = avg_font_size * 0.5;

    // Group spans into lines by Y coordinate
    let lines = cluster_into_lines(spans, row_tolerance);

    // Compute body font size: most frequent font size weighted by character count
    let body_font_size = compute_body_font_size(&lines);

    // Classify each line
    let classified: Vec<ClassifiedLine> = lines
        .into_iter()
        .map(|line| classify_line(line, body_font_size))
        .collect();

    // Merge consecutive lines into elements
    merge_lines(classified, body_font_size)
}

#[derive(Debug)]
enum LineKind {
    Heading { level: u8 },
    TableCandidate,
    Paragraph,
}

#[derive(Debug)]
struct ClassifiedLine {
    kind: LineKind,
    spans: Vec<TextSpan>,
    y: f64,
    text: String,
}

/// Group spans into lines by Y coordinate (same logic as table.rs cluster_into_rows)
fn cluster_into_lines(mut spans: Vec<TextSpan>, tolerance: f64) -> Vec<Vec<TextSpan>> {
    spans.sort_by(|a, b| {
        b.y.partial_cmp(&a.y)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then(a.x.partial_cmp(&b.x).unwrap_or(std::cmp::Ordering::Equal))
    });

    let mut lines: Vec<Vec<TextSpan>> = Vec::new();
    let mut current_line: Vec<TextSpan> = Vec::new();
    let mut current_y: Option<f64> = None;

    for span in spans {
        match current_y {
            Some(y) if (span.y - y).abs() <= tolerance => {
                current_line.push(span);
            }
            _ => {
                if !current_line.is_empty() {
                    lines.push(current_line);
                }
                current_y = Some(span.y);
                current_line = vec![span];
            }
        }
    }

    if !current_line.is_empty() {
        lines.push(current_line);
    }

    lines
}

/// Compute body font size as the most frequent font size weighted by character count
fn compute_body_font_size(lines: &[Vec<TextSpan>]) -> f64 {
    use std::collections::BTreeMap;

    // Quantize font sizes to 0.5pt to group similar sizes
    let mut freq: BTreeMap<i32, usize> = BTreeMap::new();
    for line in lines {
        for span in line {
            let key = (span.font_size * 2.0).round() as i32; // quantize to 0.5
            let char_count = span.text.chars().count();
            *freq.entry(key).or_insert(0) += char_count;
        }
    }

    freq.into_iter()
        .max_by_key(|(_, count)| *count)
        .map(|(key, _)| key as f64 / 2.0)
        .unwrap_or(12.0)
}

/// Count distinct X-position clusters in a line
fn count_x_clusters(spans: &[TextSpan]) -> usize {
    if spans.is_empty() {
        return 0;
    }

    let mut xs: Vec<f64> = spans.iter().map(|s| s.x).collect();
    xs.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

    let tolerance = 10.0;
    let mut clusters = 1;
    let mut last_x = xs[0];

    for &x in &xs[1..] {
        if (x - last_x).abs() > tolerance {
            clusters += 1;
            last_x = x;
        }
    }

    clusters
}

/// Classify a single line based on font size and X-position clustering
fn classify_line(mut spans: Vec<TextSpan>, body_font_size: f64) -> ClassifiedLine {
    spans.sort_by(|a, b| a.x.partial_cmp(&b.x).unwrap_or(std::cmp::Ordering::Equal));

    let y = spans.iter().map(|s| s.y).sum::<f64>() / spans.len() as f64;
    let max_font_size = spans
        .iter()
        .map(|s| s.font_size)
        .fold(0.0_f64, f64::max);
    let x_clusters = count_x_clusters(&spans);
    let text = spans
        .iter()
        .map(|s| s.text.trim().to_string())
        .collect::<Vec<_>>()
        .join(" ");

    let ratio = if body_font_size > 0.0 {
        max_font_size / body_font_size
    } else {
        1.0
    };

    let kind = if ratio >= 1.3 && x_clusters <= 2 {
        let level = if ratio >= 1.8 {
            1
        } else if ratio >= 1.4 {
            2
        } else {
            3
        };
        LineKind::Heading { level }
    } else if x_clusters >= 3 {
        LineKind::TableCandidate
    } else {
        LineKind::Paragraph
    };

    ClassifiedLine {
        kind,
        spans,
        y,
        text,
    }
}

/// Merge consecutive classified lines into page elements
fn merge_lines(lines: Vec<ClassifiedLine>, body_font_size: f64) -> Vec<PageElement> {
    let mut elements: Vec<PageElement> = Vec::new();
    let mut i = 0;

    while i < lines.len() {
        match &lines[i].kind {
            LineKind::Heading { level } => {
                elements.push(PageElement::Heading {
                    level: *level,
                    text: lines[i].text.clone(),
                });
                i += 1;
            }
            LineKind::TableCandidate => {
                // Collect consecutive table candidate lines
                let start = i;
                while i < lines.len() && matches!(lines[i].kind, LineKind::TableCandidate) {
                    i += 1;
                }
                let count = i - start;

                if count >= 2 {
                    // Multiple consecutive table candidates → build a Table
                    let all_spans: Vec<TextSpan> = lines[start..i]
                        .iter()
                        .flat_map(|l| l.spans.clone())
                        .collect();
                    let table = Table::from_spans(all_spans);
                    elements.push(PageElement::Table { table });
                } else {
                    // Single table-candidate line: check column count
                    let x_clusters = count_x_clusters(&lines[start].spans);
                    if x_clusters >= 4 {
                        let table = Table::from_spans(lines[start].spans.clone());
                        elements.push(PageElement::Table { table });
                    } else {
                        elements.push(PageElement::Paragraph {
                            text: lines[start].text.clone(),
                        });
                    }
                }
            }
            LineKind::Paragraph => {
                // Collect consecutive paragraph lines
                let mut paragraph_parts: Vec<String> = Vec::new();
                let mut prev_y = lines[i].y;

                while i < lines.len() && matches!(lines[i].kind, LineKind::Paragraph) {
                    let gap = (prev_y - lines[i].y).abs();
                    // Large Y-gap means paragraph break (> 1.5x body font size)
                    if !paragraph_parts.is_empty() && gap > body_font_size * 1.5 {
                        break;
                    }
                    paragraph_parts.push(lines[i].text.clone());
                    prev_y = lines[i].y;
                    i += 1;
                }

                let text = paragraph_parts.join(" ");
                if !text.trim().is_empty() {
                    elements.push(PageElement::Paragraph { text });
                }
            }
        }
    }

    elements
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_span(text: &str, x: f64, y: f64, font_size: f64) -> TextSpan {
        TextSpan {
            text: text.to_string(),
            x,
            y,
            font_size,
            font_name: None,
        }
    }

    #[test]
    fn test_heading_detection() {
        // Large font = heading, normal font = paragraph
        let spans = vec![
            make_span("Title", 50.0, 700.0, 24.0),
            make_span("Normal text here.", 50.0, 670.0, 12.0),
        ];

        let elements = classify_spans(spans);
        assert_eq!(elements.len(), 2);
        assert!(matches!(&elements[0], PageElement::Heading { level: 1, text } if text == "Title"));
        assert!(matches!(&elements[1], PageElement::Paragraph { text } if text == "Normal text here."));
    }

    #[test]
    fn test_table_detection() {
        // Multiple rows with 3+ X-clusters = table
        let spans = vec![
            make_span("A", 50.0, 500.0, 12.0),
            make_span("B", 200.0, 500.0, 12.0),
            make_span("C", 350.0, 500.0, 12.0),
            make_span("1", 50.0, 480.0, 12.0),
            make_span("2", 200.0, 480.0, 12.0),
            make_span("3", 350.0, 480.0, 12.0),
        ];

        let elements = classify_spans(spans);
        assert_eq!(elements.len(), 1);
        assert!(matches!(&elements[0], PageElement::Table { .. }));
    }

    #[test]
    fn test_paragraph_merging() {
        // Consecutive lines with same font size and close Y = merged paragraph
        let spans = vec![
            make_span("First line of text", 50.0, 500.0, 12.0),
            make_span("second line of text", 50.0, 486.0, 12.0),
            make_span("third line of text", 50.0, 472.0, 12.0),
        ];

        let elements = classify_spans(spans);
        assert_eq!(elements.len(), 1);
        assert!(matches!(&elements[0], PageElement::Paragraph { .. }));
        if let PageElement::Paragraph { text } = &elements[0] {
            assert!(text.contains("First line"));
            assert!(text.contains("third line"));
        }
    }

    #[test]
    fn test_mixed_content() {
        let spans = vec![
            // Heading
            make_span("Document Title", 50.0, 750.0, 24.0),
            // Paragraph
            make_span("Some introductory text.", 50.0, 710.0, 12.0),
            // Table (3 columns, 2 rows)
            make_span("Name", 50.0, 680.0, 12.0),
            make_span("Age", 200.0, 680.0, 12.0),
            make_span("City", 350.0, 680.0, 12.0),
            make_span("Alice", 50.0, 660.0, 12.0),
            make_span("30", 200.0, 660.0, 12.0),
            make_span("NYC", 350.0, 660.0, 12.0),
        ];

        let elements = classify_spans(spans);
        assert!(elements.len() >= 3);
        assert!(matches!(&elements[0], PageElement::Heading { .. }));
        assert!(matches!(&elements[1], PageElement::Paragraph { .. }));
        assert!(matches!(&elements[2], PageElement::Table { .. }));
    }

    #[test]
    fn test_empty_spans() {
        let elements = classify_spans(vec![]);
        assert!(elements.is_empty());
    }

    #[test]
    fn test_body_font_size_detection() {
        let lines = vec![
            vec![make_span("Big Title", 50.0, 700.0, 24.0)],
            vec![make_span("Normal text line one that is quite long.", 50.0, 670.0, 12.0)],
            vec![make_span("Normal text line two also fairly long.", 50.0, 655.0, 12.0)],
            vec![make_span("Normal text line three.", 50.0, 640.0, 12.0)],
        ];
        let body = compute_body_font_size(&lines);
        assert!((body - 12.0).abs() < 0.5);
    }

    #[test]
    fn test_x_cluster_counting() {
        let spans = vec![
            make_span("A", 50.0, 500.0, 12.0),
            make_span("B", 52.0, 500.0, 12.0), // same cluster as A (within 10px)
            make_span("C", 200.0, 500.0, 12.0),
            make_span("D", 350.0, 500.0, 12.0),
        ];
        assert_eq!(count_x_clusters(&spans), 3);
    }
}
