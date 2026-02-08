pub mod content;
pub mod decode;
pub mod document;
pub mod error;
pub mod extract;
pub mod font;
pub mod parser;
pub mod types;

#[cfg(target_arch = "wasm32")]
pub mod wasm;

pub use content::TextSpan;
pub use decode::decode_stream;
pub use document::Document;
pub use error::{PdfError, Result};
pub use extract::{classify_spans, elements_to_markdown, elements_to_txt, PageElement, Table};
pub use types::{ObjRef, PdfObject};

