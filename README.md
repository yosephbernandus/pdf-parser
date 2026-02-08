# PDF Parser

A PDF parser written in Rust that can extract text, tables, and structured content from PDF files.

## What it does

Parses PDF files and extracts content as CSV, TSV, plain text, or Markdown. It classifies content into headings, paragraphs, and tables using font size and layout analysis. Handles various PDF formats including bank statements, credit card bills, transaction histories, and general documents.

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
--txt       Output as plain text (headings, paragraphs, tables)
--md        Output as Markdown
--raw       Output raw text with positions
--page N    Extract only page N (1-indexed)
-o FILE     Write output to FILE
```

Examples:
```bash
# Extract as TSV
./target/release/pdf-table statement.pdf --tsv

# Extract as plain text (layout-aware)
./target/release/pdf-table document.pdf --txt

# Extract as Markdown (with headings and tables)
./target/release/pdf-table document.pdf --md

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

Extract tables:
```rust
use pdf_parser::{Document, Table};

let data = std::fs::read("input.pdf")?;
let mut doc = Document::parse(&data)?;
let spans = doc.extract_page_text(0)?;
let table = Table::from_spans(spans);
println!("{}", table.to_csv());
```

Extract as text or Markdown (layout-aware):
```rust
use pdf_parser::{Document, classify_spans, elements_to_txt, elements_to_markdown};

let data = std::fs::read("input.pdf")?;
let mut doc = Document::parse(&data)?;
let spans = doc.extract_page_text(0)?;
let elements = classify_spans(spans);
println!("{}", elements_to_txt(&elements));
println!("{}", elements_to_markdown(&elements));
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
- Layout classification (headings, paragraphs, tables)
- Plain text and Markdown output

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
  extract/       - Table extraction, layout classification, TXT/Markdown renderers
  main.rs        - CLI
```

## Testing

```bash
cargo test
```

Currently 47 tests covering lexer, parser, decoder, content extraction, table extraction, layout classification, and TXT/Markdown rendering.

## Author

Yoseph Bernandus
