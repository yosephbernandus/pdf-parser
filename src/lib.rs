pub mod decode;
pub mod document;
pub mod error;
pub mod parser;
pub mod types;

pub use decode::decode_stream;
pub use document::Document;
pub use error::{PdfError, Result};
pub use types::{ObjRef, PdfObject};

