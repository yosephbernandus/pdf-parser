use wasm_bindgen::prelude::*;

use crate::{classify_spans, elements_to_markdown, elements_to_txt, Document, Table};

/// Initialize panic hook for better error messages in browser console
#[wasm_bindgen(start)]
pub fn init() {
    console_error_panic_hook::set_once();
}

/// Parse a PDF and return CSV string for all pages
#[wasm_bindgen]
pub fn pdf_to_csv(data: &[u8]) -> Result<String, JsValue> {
    let mut doc =
        Document::parse(data).map_err(|e| JsValue::from_str(&format!("Parse error: {}", e)))?;

    let page_count = doc
        .page_count()
        .map_err(|e| JsValue::from_str(&format!("Page count error: {}", e)))?;

    let mut output = String::new();

    for page_idx in 0..page_count {
        let spans = doc
            .extract_page_text(page_idx)
            .map_err(|e| JsValue::from_str(&format!("Page {} error: {}", page_idx + 1, e)))?;

        let table = Table::from_spans(spans);

        if !output.is_empty() {
            output.push('\n');
        }
        output.push_str(&table.to_csv());
    }

    Ok(output)
}

/// Parse a PDF and return TSV string for all pages
#[wasm_bindgen]
pub fn pdf_to_tsv(data: &[u8]) -> Result<String, JsValue> {
    let mut doc =
        Document::parse(data).map_err(|e| JsValue::from_str(&format!("Parse error: {}", e)))?;

    let page_count = doc
        .page_count()
        .map_err(|e| JsValue::from_str(&format!("Page count error: {}", e)))?;

    let mut output = String::new();

    for page_idx in 0..page_count {
        let spans = doc
            .extract_page_text(page_idx)
            .map_err(|e| JsValue::from_str(&format!("Page {} error: {}", page_idx + 1, e)))?;

        let table = Table::from_spans(spans);

        if !output.is_empty() {
            output.push('\n');
        }
        output.push_str(&table.to_tsv());
    }

    Ok(output)
}

/// Get page count from a PDF
#[wasm_bindgen]
pub fn pdf_page_count(data: &[u8]) -> Result<usize, JsValue> {
    let mut doc =
        Document::parse(data).map_err(|e| JsValue::from_str(&format!("Parse error: {}", e)))?;

    doc.page_count()
        .map_err(|e| JsValue::from_str(&format!("Page count error: {}", e)))
}

/// Parse a single page (0-indexed) and return CSV
#[wasm_bindgen]
pub fn pdf_page_to_csv(data: &[u8], page: usize) -> Result<String, JsValue> {
    let mut doc =
        Document::parse(data).map_err(|e| JsValue::from_str(&format!("Parse error: {}", e)))?;

    let spans = doc
        .extract_page_text(page)
        .map_err(|e| JsValue::from_str(&format!("Page {} error: {}", page + 1, e)))?;

    let table = Table::from_spans(spans);
    Ok(table.to_csv())
}

/// Parse a PDF and return plain text (layout-aware) for all pages
#[wasm_bindgen]
pub fn pdf_to_txt(data: &[u8]) -> Result<String, JsValue> {
    let mut doc =
        Document::parse(data).map_err(|e| JsValue::from_str(&format!("Parse error: {}", e)))?;

    let page_count = doc
        .page_count()
        .map_err(|e| JsValue::from_str(&format!("Page count error: {}", e)))?;

    let mut output = String::new();

    for page_idx in 0..page_count {
        let spans = doc
            .extract_page_text(page_idx)
            .map_err(|e| JsValue::from_str(&format!("Page {} error: {}", page_idx + 1, e)))?;

        let elements = classify_spans(spans);

        if !output.is_empty() {
            output.push('\n');
        }
        output.push_str(&elements_to_txt(&elements));
    }

    Ok(output)
}

/// Parse a PDF and return Markdown (layout-aware) for all pages
#[wasm_bindgen]
pub fn pdf_to_md(data: &[u8]) -> Result<String, JsValue> {
    let mut doc =
        Document::parse(data).map_err(|e| JsValue::from_str(&format!("Parse error: {}", e)))?;

    let page_count = doc
        .page_count()
        .map_err(|e| JsValue::from_str(&format!("Page count error: {}", e)))?;

    let mut output = String::new();

    for page_idx in 0..page_count {
        let spans = doc
            .extract_page_text(page_idx)
            .map_err(|e| JsValue::from_str(&format!("Page {} error: {}", page_idx + 1, e)))?;

        let elements = classify_spans(spans);

        if !output.is_empty() {
            output.push('\n');
        }
        output.push_str(&elements_to_markdown(&elements));
    }

    Ok(output)
}
