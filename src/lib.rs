pub mod error;
pub mod parser;
pub mod types;

pub use error::{PdfError, Result};
pub use types::{ObjRef, PdfObject};
