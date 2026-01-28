use std::fs;
use pdf_parser::Document;

fn main() {
    let args: Vec<String> = std::env::args().collect();

    let Some(path) = args.get(1) else {
        eprintln!("Usage: {} <pdf-file> [--raw]", args[0]);
        std::process::exit(1);
    };

    let show_raw = args.get(2).map(|s| s == "--raw").unwrap_or(false);

    println!("Reading: {}", path);

    let data = match fs::read(path) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("Failed to read file: {}", e);
            std::process::exit(1);
        }
    };

    match Document::parse(&data) {
        Ok(mut doc) => {
            println!("PDF parsed successfully!");
            println!("Objects in xref: {}", doc.object_count());

            match doc.page_count() {
                Ok(count) => println!("Page count: {}", count),
                Err(e) => println!("Could not get page count: {}", e),
            }

            // Extract text from all pages
            println!("\n========== EXTRACTED TEXT ==========\n");

            let page_count = doc.page_count().unwrap_or(0);
            for i in 0..page_count {
                println!("--- Page {} ---\n", i + 1);

                if show_raw {
                    // Show raw text spans with positions
                    match doc.extract_page_text(i) {
                        Ok(spans) => {
                            for span in spans {
                                println!(
                                    "[{:.1}, {:.1}] ({}pt): {}",
                                    span.x, span.y, span.font_size, span.text
                                );
                            }
                        }
                        Err(e) => println!("Error extracting page {}: {}", i + 1, e),
                    }
                } else {
                    // Show formatted text
                    match doc.extract_page_text_string(i) {
                        Ok(text) => println!("{}", text),
                        Err(e) => println!("Error extracting page {}: {}", i + 1, e),
                    }
                }

                println!();
            }
        }
        Err(e) => {
            eprintln!("Error parsing PDF: {}", e);
            std::process::exit(1);
        }
    }
}
