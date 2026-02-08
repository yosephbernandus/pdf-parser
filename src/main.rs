use std::fs;
use pdf_parser::{classify_spans, elements_to_markdown, elements_to_txt, Document, Table};

fn print_usage(program: &str) {
    eprintln!("Usage: {} <pdf-file> [options]", program);
    eprintln!();
    eprintln!("Options:");
    eprintln!("  --csv       Output as CSV (default)");
    eprintln!("  --tsv       Output as TSV (tab-separated)");
    eprintln!("  --text      Output as aligned text");
    eprintln!("  --txt       Output as plain text (headings, paragraphs, tables)");
    eprintln!("  --md        Output as Markdown");
    eprintln!("  --raw       Output raw text spans with positions");
    eprintln!("  --page N    Extract only page N (1-indexed)");
    eprintln!("  -o FILE     Write output to FILE instead of stdout");
}

fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.len() < 2 {
        print_usage(&args[0]);
        std::process::exit(1);
    }

    let path = &args[1];

    if path == "--help" || path == "-h" {
        print_usage(&args[0]);
        return;
    }

    // Parse options
    let mut format = "csv";
    let mut output_file: Option<String> = None;
    let mut page_filter: Option<usize> = None;

    let mut i = 2;
    while i < args.len() {
        match args[i].as_str() {
            "--csv" => format = "csv",
            "--tsv" => format = "tsv",
            "--text" => format = "text",
            "--txt" => format = "txt",
            "--md" => format = "md",
            "--raw" => format = "raw",
            "--page" => {
                i += 1;
                if i < args.len() {
                    page_filter = args[i].parse().ok();
                }
            }
            "-o" => {
                i += 1;
                if i < args.len() {
                    output_file = Some(args[i].clone());
                }
            }
            _ => {}
        }
        i += 1;
    }

    // Read PDF
    eprintln!("Reading: {}", path);

    let data = match fs::read(path) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("Failed to read file: {}", e);
            std::process::exit(1);
        }
    };

    // Parse PDF
    let mut doc = match Document::parse(&data) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("Error parsing PDF: {}", e);
            std::process::exit(1);
        }
    };

    eprintln!("PDF parsed successfully!");

    let page_count = doc.page_count().unwrap_or(0);
    eprintln!("Page count: {}", page_count);

    // Determine which pages to process
    let pages: Vec<usize> = match page_filter {
        Some(p) if p >= 1 && p <= page_count => vec![p - 1],
        Some(p) => {
            eprintln!("Invalid page number: {} (document has {} pages)", p, page_count);
            std::process::exit(1);
        }
        None => (0..page_count).collect(),
    };

    // Collect output
    let mut output = String::new();

    for page_idx in pages {
        match doc.extract_page_text(page_idx) {
            Ok(spans) => {
                if format == "raw" {
                    // Raw output with positions
                    if !output.is_empty() {
                        output.push_str("\n--- Page {} ---\n");
                    }
                    for span in spans {
                        output.push_str(&format!(
                            "[{:.1}, {:.1}] ({}pt): {}\n",
                            span.x, span.y, span.font_size, span.text
                        ));
                    }
                } else if format == "txt" || format == "md" {
                    // Layout-aware extraction
                    let elements = classify_spans(spans);

                    if !output.is_empty() {
                        output.push('\n');
                    }

                    match format {
                        "txt" => output.push_str(&elements_to_txt(&elements)),
                        "md" => output.push_str(&elements_to_markdown(&elements)),
                        _ => unreachable!(),
                    }
                } else {
                    // Table extraction
                    let table = Table::from_spans(spans);

                    if !output.is_empty() {
                        output.push('\n');
                    }

                    match format {
                        "csv" => output.push_str(&table.to_csv()),
                        "tsv" => output.push_str(&table.to_tsv()),
                        "text" => output.push_str(&table.to_text()),
                        _ => output.push_str(&table.to_csv()),
                    }
                }
            }
            Err(e) => {
                eprintln!("Error extracting page {}: {}", page_idx + 1, e);
            }
        }
    }

    // Write output
    match output_file {
        Some(path) => {
            if let Err(e) = fs::write(&path, &output) {
                eprintln!("Failed to write output: {}", e);
                std::process::exit(1);
            }
            eprintln!("Output written to: {}", path);
        }
        None => {
            println!("{}", output);
        }
    }
}
