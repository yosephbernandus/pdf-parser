use pdf_text_extract::{Document, Table};
use std::fs;

fn main() {
    let data = fs::read("21536462_APR_2025.pdf").expect("Failed to read file");
    let mut doc = Document::parse(&data).expect("Failed to parse PDF");
    
    let page_count = doc.page_count().unwrap();
    println!("Page count: {}\n", page_count);
    
    let spans = doc.extract_page_text(0).expect("Failed to extract text");
    let table = Table::from_spans(spans);
    
    println!("Total rows: {}, Columns: {}\n", table.rows.len(), table.num_columns);
    
    // Find rows with keyword matches
    let keywords = ["tanggal", "jumlah", "keterangan", "date", "amount"];
    for (row_idx, row) in table.rows.iter().enumerate() {
        let row_text = row.join(" ").to_lowercase();
        let matches: Vec<&&str> = keywords.iter().filter(|&&kw| row_text.contains(kw)).collect();
        if !matches.is_empty() {
            println!("=== ROW {} (matches: {:?}) ===", row_idx, matches);
            for (col_idx, cell) in row.iter().enumerate() {
                if !cell.trim().is_empty() {
                    println!("  Col {}: '{}'", col_idx, cell.trim());
                }
            }
            println!();
        }
    }
    
    // Also show first data rows after headers
    println!("=== ALL ROWS 8..20 ===");
    for (row_idx, row) in table.rows.iter().enumerate().skip(8).take(12) {
        let non_empty: Vec<(usize, &String)> = row.iter().enumerate()
            .filter(|(_, c)| !c.trim().is_empty()).collect();
        if !non_empty.is_empty() {
            println!("Row {}: {:?}", row_idx, non_empty.iter().map(|(i, s)| format!("[{}]={}", i, s.trim())).collect::<Vec<_>>());
        }
    }
}
