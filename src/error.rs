use thiserror::Error;

#[derive(Error, Debug)]
pub enum PdfError {
    #[error("Invalid PDF: missing %PDF header")]
    MissingHeader,

    #[error("Invalid PDF: missing %%EOF marker")]
    MissingEof,

    #[error("Parse error at byte {position}: {message}")]
    Parse { position: usize, message: String },

    #[error("Invalid xref table")]
    InvalidXref,

    #[error("Object not found: {0} {1} R")]
    ObjectNotFound(u32, u16),

    #[error("Invalid document structure: {0}")]
    InvalidStructure(String),

    #[error("Unsupported filter: {0}")]
    UnsupportedFilter(String),

    #[error("Decompression failed: {0}")]
    DecompressError(String),

    #[error("Invalid UTF-8 in string")]
    InvalidUtf8,

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

pub type Result<T> = std::result::Result<T, PdfError>;
