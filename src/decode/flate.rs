use flate2::read::ZlibDecoder;
use std::io::Read;

use crate::error::{PdfError, Result};

/// Decompress zlib/deflate data
pub fn flate_decode(data: &[u8]) -> Result<Vec<u8>> {
    let mut decoder = ZlibDecoder::new(data);
    let mut result = Vec::new();

    decoder.read_to_end(&mut result).map_err(|e| {
        PdfError::DecompressError(format!("FlateDecode failed: {}", e))
    })?;

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use flate2::write::ZlibEncoder;
    use flate2::Compression;
    use std::io::Write;

    #[test]
    fn test_flate_decode() {
        // Compress some data
        let original = b"Hello, PDF World! This is a test of FlateDecode.";
        let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(original).unwrap();
        let compressed = encoder.finish().unwrap();

        // Decompress it
        let decoded = flate_decode(&compressed).unwrap();

        assert_eq!(decoded, original);
    }
}
