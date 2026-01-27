use std::fs;
use pdf_parser::Document;

fn main() {
    let args: Vec<String> = std::env::args().collect();

    let Some(path) = args.get(1) else {
        eprintln!("Usage: {} <pdf-file>", args[0]);
        std::process::exit(1);
    };

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

            // Show trailer keys
            println!("\nTrailer keys:");
            for key in doc.trailer().keys() {
                println!("  - {}", key);
            }

            // Try to get page count
            match doc.page_count() {
                Ok(count) => println!("\nPage count: {}", count),
                Err(e) => println!("\nCould not get page count: {}", e),
            }
        }
        Err(e) => {
            eprintln!("Error parsing PDF: {}", e);
            std::process::exit(1);
        }
    }
}
