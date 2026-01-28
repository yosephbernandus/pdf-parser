use crate::content::TextSpan;

/// Extracted table with rows and columns
#[derive(Debug, Clone)]
pub struct Table {
    pub rows: Vec<Vec<String>>,
    pub num_columns: usize,
}

impl Table {
    /// Build a table from text spans
    pub fn from_spans(spans: Vec<TextSpan>) -> Self {
        // Filter empty spans
        let spans: Vec<_> = spans
            .into_iter()
            .filter(|s| !s.text.trim().is_empty())
            .collect();

        if spans.is_empty() {
            return Table {
                rows: Vec::new(),
                num_columns: 0,
            };
        }

        // Calculate adaptive tolerance based on average font size
        let avg_font_size = spans.iter().map(|s| s.font_size).sum::<f64>() / spans.len() as f64;
        let row_tolerance = avg_font_size * 0.5;

        // Cluster into rows by Y coordinate
        let mut rows = cluster_into_rows(spans, row_tolerance);

        // Sort within each row by X coordinate
        for row in &mut rows {
            row.sort_by(|a, b| a.x.partial_cmp(&b.x).unwrap_or(std::cmp::Ordering::Equal));
        }

        // Detect column boundaries
        let columns = detect_columns(&rows);

        // Assign spans to grid cells
        let grid = assign_to_columns(rows, &columns);

        Table {
            num_columns: columns.len(),
            rows: grid,
        }
    }

    /// Convert table to CSV string
    pub fn to_csv(&self) -> String {
        self.rows
            .iter()
            .map(|row| {
                row.iter()
                    .map(|cell| escape_csv(cell))
                    .collect::<Vec<_>>()
                    .join(",")
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Convert table to TSV (tab-separated) string
    pub fn to_tsv(&self) -> String {
        self.rows
            .iter()
            .map(|row| {
                row.iter()
                    .map(|cell| cell.replace('\t', " "))
                    .collect::<Vec<_>>()
                    .join("\t")
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Convert table to plain text with aligned columns
    pub fn to_text(&self) -> String {
        if self.rows.is_empty() {
            return String::new();
        }

        // Calculate column widths
        let mut widths: Vec<usize> = vec![0; self.num_columns];
        for row in &self.rows {
            for (i, cell) in row.iter().enumerate() {
                if i < widths.len() {
                    widths[i] = widths[i].max(cell.chars().count());
                }
            }
        }

        // Build output with padding
        self.rows
            .iter()
            .map(|row| {
                row.iter()
                    .enumerate()
                    .map(|(i, cell)| {
                        let width = widths.get(i).copied().unwrap_or(0);
                        format!("{:<width$}", cell, width = width)
                    })
                    .collect::<Vec<_>>()
                    .join("  ")
                    .trim_end()
                    .to_string()
            })
            .collect::<Vec<_>>()
            .join("\n")
    }
}

/// Group spans into rows by Y coordinate
fn cluster_into_rows(mut spans: Vec<TextSpan>, tolerance: f64) -> Vec<Vec<TextSpan>> {
    // Sort by Y descending (top to bottom), then X ascending
    spans.sort_by(|a, b| {
        b.y.partial_cmp(&a.y)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then(a.x.partial_cmp(&b.x).unwrap_or(std::cmp::Ordering::Equal))
    });

    let mut rows: Vec<Vec<TextSpan>> = Vec::new();
    let mut current_row: Vec<TextSpan> = Vec::new();
    let mut current_y: Option<f64> = None;

    for span in spans {
        match current_y {
            Some(y) if (span.y - y).abs() <= tolerance => {
                // Same row
                current_row.push(span);
            }
            _ => {
                // New row
                if !current_row.is_empty() {
                    rows.push(current_row);
                }
                current_y = Some(span.y);
                current_row = vec![span];
            }
        }
    }

    if !current_row.is_empty() {
        rows.push(current_row);
    }

    rows
}

/// Detect column boundaries from X positions
fn detect_columns(rows: &[Vec<TextSpan>]) -> Vec<f64> {
    // Collect all X positions
    let mut x_positions: Vec<f64> = rows
        .iter()
        .flat_map(|row| row.iter().map(|s| s.x))
        .collect();

    if x_positions.is_empty() {
        return Vec::new();
    }

    x_positions.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

    // Cluster X positions
    let tolerance = 10.0;
    let mut columns: Vec<f64> = Vec::new();
    let mut cluster: Vec<f64> = Vec::new();

    for x in x_positions {
        if cluster.is_empty() {
            cluster.push(x);
        } else {
            let last = cluster.last().unwrap();
            if (x - last).abs() <= tolerance {
                cluster.push(x);
            } else {
                // End cluster, take average as column position
                let avg = cluster.iter().sum::<f64>() / cluster.len() as f64;
                columns.push(avg);
                cluster = vec![x];
            }
        }
    }

    // Don't forget last cluster
    if !cluster.is_empty() {
        let avg = cluster.iter().sum::<f64>() / cluster.len() as f64;
        columns.push(avg);
    }

    columns
}

/// Assign spans to grid cells based on nearest column
fn assign_to_columns(rows: Vec<Vec<TextSpan>>, columns: &[f64]) -> Vec<Vec<String>> {
    let num_cols = columns.len();

    rows.into_iter()
        .map(|row| {
            // Create row with empty cells
            let mut cells: Vec<String> = vec![String::new(); num_cols];

            for span in row {
                // Find nearest column
                let col_idx = columns
                    .iter()
                    .enumerate()
                    .min_by(|(_, a), (_, b)| {
                        let diff_a = (span.x - **a).abs();
                        let diff_b = (span.x - **b).abs();
                        diff_a.partial_cmp(&diff_b).unwrap_or(std::cmp::Ordering::Equal)
                    })
                    .map(|(i, _)| i)
                    .unwrap_or(0);

                // Append to cell (may have multiple spans in same cell)
                if !cells[col_idx].is_empty() {
                    cells[col_idx].push(' ');
                }
                cells[col_idx].push_str(&span.text);
            }

            cells
        })
        .collect()
}

/// Escape a string for CSV output
fn escape_csv(s: &str) -> String {
    if s.contains(',') || s.contains('"') || s.contains('\n') || s.contains('\r') {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_span(text: &str, x: f64, y: f64) -> TextSpan {
        TextSpan {
            text: text.to_string(),
            x,
            y,
            font_size: 12.0,
            font_name: None,
        }
    }

    #[test]
    fn test_simple_table() {
        let spans = vec![
            make_span("A", 0.0, 100.0),
            make_span("B", 50.0, 100.0),
            make_span("1", 0.0, 80.0),
            make_span("2", 50.0, 80.0),
        ];

        let table = Table::from_spans(spans);

        assert_eq!(table.num_columns, 2);
        assert_eq!(table.rows.len(), 2);
        assert_eq!(table.rows[0], vec!["A", "B"]);
        assert_eq!(table.rows[1], vec!["1", "2"]);
    }

    #[test]
    fn test_csv_output() {
        let spans = vec![
            make_span("Name", 0.0, 100.0),
            make_span("Value", 50.0, 100.0),
            make_span("Test, Item", 0.0, 80.0),
            make_span("123", 50.0, 80.0),
        ];

        let table = Table::from_spans(spans);
        let csv = table.to_csv();

        assert!(csv.contains("Name,Value"));
        assert!(csv.contains("\"Test, Item\",123"));
    }

    #[test]
    fn test_row_clustering() {
        let spans = vec![
            make_span("A", 0.0, 100.0),
            make_span("B", 50.0, 100.5), // Slightly different Y
            make_span("C", 0.0, 80.0),
        ];

        let rows = cluster_into_rows(spans, 6.0); // tolerance based on font size

        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].len(), 2); // A and B in same row
        assert_eq!(rows[1].len(), 1); // C in separate row
    }

    #[test]
    fn test_tsv_output() {
        let spans = vec![
            make_span("Col1", 0.0, 100.0),
            make_span("Col2", 50.0, 100.0),
            make_span("Data1", 0.0, 80.0),
            make_span("Data2", 50.0, 80.0),
        ];

        let table = Table::from_spans(spans);
        let tsv = table.to_tsv();

        assert!(tsv.contains("Col1\tCol2"));
        assert!(tsv.contains("Data1\tData2"));
    }
}
