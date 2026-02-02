# PDF Parser

A PDF parser written in Rust that can extract text and tables from PDF files.

## What it does

Parses PDF files and extracts tabular data. It handles various PDF formats including bank statements, credit card bills, and transaction histories.

## Building

```bash
cargo build --release
```

The binary will be at `target/release/pdf-table`

## Usage

Basic usage:
```bash
./target/release/pdf-table input.pdf
```

This outputs the extracted table as CSV to stdout.

Available options:
```bash
--csv       Output as CSV (default)
--tsv       Output as TSV (tab-separated)
--text      Output as aligned text
--raw       Output raw text with positions
--page N    Extract only page N (1-indexed)
-o FILE     Write output to FILE
```

Examples:
```bash
# Extract as TSV
./target/release/pdf-table statement.pdf --tsv

# Save to file
./target/release/pdf-table statement.pdf -o output.csv

# Extract specific page
./target/release/pdf-table statement.pdf --page 1

# See raw text positions
./target/release/pdf-table statement.pdf --raw
```

## Using as a library

Add to your Cargo.toml:
```toml
[dependencies]
pdf-parser = { path = "../pdf-parser" }
```

Example code:
```rust
use pdf_parser::Document;

let data = std::fs::read("input.pdf")?;
let mut doc = Document::parse(&data)?;

let page_count = doc.page_count()?;
let text_spans = doc.extract_page_text(0)?;

for span in text_spans {
    println!("Text at ({}, {}): {}", span.x, span.y, span.text);
}
```

## What it supports

- PDF 1.4 format with traditional xref tables
- Incrementally updated PDFs (follows Prev chain)
- FlateDecode and ASCIIHexDecode stream compression
- WinAnsiEncoding and MacRomanEncoding
- Type0 CID fonts with ToUnicode CMaps
- Nested page trees
- Text extraction with coordinates
- Table detection from positioned text

## What it doesn't support

- xref streams (PDF 1.5+)
- Encrypted PDFs
- Embedded images
- Complex font subsetting
- Interactive forms

## Project structure

```
src/
  error.rs       - Error types
  types/         - PDF objects (Int, String, Dict, Array, Stream, etc)
  parser/        - Tokenizer and object parser
  document.rs    - PDF document (xref, pages, object resolution)
  decode/        - Stream decoders
  font/          - Font encodings and ToUnicode CMap parser
  content/       - Content stream parser (extracts text)
  extract/       - Table extraction
  main.rs        - CLI
```

## Testing

```bash
cargo test
```

Currently 31 tests covering lexer, parser, decoder, content extraction, and table extraction.

## Author

Yoseph Bernandus
