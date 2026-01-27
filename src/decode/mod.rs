mod flate;

use crate::error::{PdfError, Result};
use crate::types::PdfObject;
use std::collections::HashMap;

pub use flate::flate_decode;

/// Decode stream data based on Filter(s) in the stream dictionary
pub fn decode_stream(dict: &HashMap<String, PdfObject>, data: &[u8]) -> Result<Vec<u8>> {
    let filters = get_filters(dict)?;

    if filters.is_empty() {
        // No filters - return raw data
        return Ok(data.to_vec());
    }

    let mut result = data.to_vec();

    for filter in filters {
        result = apply_filter(&filter, &result)?;
    }

    Ok(result)
}

/// Extract filter names from dictionary
fn get_filters(dict: &HashMap<String, PdfObject>) -> Result<Vec<String>> {
    match dict.get("Filter") {
        None => Ok(vec![]),
        Some(PdfObject::Name(name)) => Ok(vec![name.clone()]),
        Some(PdfObject::Array(arr)) => arr
            .iter()
            .map(|obj| {
                obj.as_name()
                    .map(|s| s.to_string())
                    .ok_or_else(|| PdfError::InvalidStructure("Filter must be name".into()))
            })
            .collect(),
        _ => Err(PdfError::InvalidStructure("Invalid Filter type".into())),
    }
}

/// Apply a single filter
fn apply_filter(filter: &str, data: &[u8]) -> Result<Vec<u8>> {
    match filter {
        "FlateDecode" => flate_decode(data),
        "ASCIIHexDecode" => ascii_hex_decode(data),
        other => Err(PdfError::UnsupportedFilter(other.to_string())),
    }
}

/// Decode ASCII hex encoded data
fn ascii_hex_decode(data: &[u8]) -> Result<Vec<u8>> {
    let mut result = Vec::new();
    let mut chars = data.iter().filter(|&&b| !b.is_ascii_whitespace());

    loop {
        let Some(&h1) = chars.next() else { break };
        if h1 == b'>' {
            break; // End of data marker
        }

        let h2 = chars.next().copied().unwrap_or(b'0');

        let byte = (hex_val(h1)? << 4) | hex_val(h2)?;
        result.push(byte);
    }

    Ok(result)
}

fn hex_val(b: u8) -> Result<u8> {
    match b {
        b'0'..=b'9' => Ok(b - b'0'),
        b'a'..=b'f' => Ok(b - b'a' + 10),
        b'A'..=b'F' => Ok(b - b'A' + 10),
        _ => Err(PdfError::Parse {
            position: 0,
            message: format!("Invalid hex char: {}", b as char),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_no_filter() {
        let dict = HashMap::new();
        let data = b"raw data";
        let result = decode_stream(&dict, data).unwrap();
        assert_eq!(result, data);
    }

    #[test]
    fn test_ascii_hex_decode() {
        let data = b"48656C6C6F>";  // "Hello"
        let result = ascii_hex_decode(data).unwrap();
        assert_eq!(result, b"Hello");
    }

    #[test]
    fn test_ascii_hex_with_whitespace() {
        let data = b"48 65 6C 6C 6F>";
        let result = ascii_hex_decode(data).unwrap();
        assert_eq!(result, b"Hello");
    }
}
