pub mod content;
pub mod decode;
pub mod document;
pub mod error;
pub mod extract;
pub mod font;
pub mod parser;
pub mod types;

#[cfg(feature = "wasm")]
pub mod wasm;

pub use content::TextSpan;
pub use decode::decode_stream;
pub use document::Document;
pub use error::{PdfError, Result};
pub use extract::{classify_spans, elements_to_markdown, elements_to_txt, PageElement, Table};
pub use types::{ObjRef, PdfObject};

/// Extract all text from a PDF as plain text (layout-aware)
pub fn pdf_to_text(data: &[u8]) -> Result<String> {
    let mut doc = Document::parse(data)?;
    let page_count = doc.page_count()?;
    let mut output = String::new();

    for page_idx in 0..page_count {
        let spans = doc.extract_page_text(page_idx)?;
        let elements = classify_spans(spans);

        if !output.is_empty() {
            output.push('\n');
        }
        output.push_str(&elements_to_txt(&elements));
    }

    Ok(output)
}

/// Extract all text from a PDF as Markdown (layout-aware)
pub fn pdf_to_markdown(data: &[u8]) -> Result<String> {
    let mut doc = Document::parse(data)?;
    let page_count = doc.page_count()?;
    let mut output = String::new();

    for page_idx in 0..page_count {
        let spans = doc.extract_page_text(page_idx)?;
        let elements = classify_spans(spans);

        if !output.is_empty() {
            output.push('\n');
        }
        output.push_str(&elements_to_markdown(&elements));
    }

    Ok(output)
}

/// Extract all text from a PDF as CSV
pub fn pdf_to_csv(data: &[u8]) -> Result<String> {
    let mut doc = Document::parse(data)?;
    let page_count = doc.page_count()?;
    let mut output = String::new();

    for page_idx in 0..page_count {
        let spans = doc.extract_page_text(page_idx)?;
        let table = Table::from_spans(spans);

        if !output.is_empty() {
            output.push('\n');
        }
        output.push_str(&table.to_csv());
    }

    Ok(output)
}
