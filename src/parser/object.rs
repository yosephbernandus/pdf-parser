use std::collections::HashMap;

use crate::error::{PdfError, Result};
use crate::parser::lexer::{Lexer, Token};
use crate::types::{ObjRef, PdfObject};

pub struct Parser<'a> {
    lexer: Lexer<'a>,
    /// Lookahead buffer for handling "42 0 R" vs "42"
    peeked: Vec<Token>,
}

impl<'a> Parser<'a> {
    pub fn new(data: &'a [u8]) -> Self {
        Self {
            lexer: Lexer::new(data),
            peeked: Vec::new(),
        }
    }

    pub fn position(&self) -> usize {
        self.lexer.position()
    }

    pub fn seek(&mut self, pos: usize) {
        self.lexer.seek(pos);
        self.peeked.clear();
    }

    /// Get next token (from buffer or lexer)
    fn next_token(&mut self) -> Result<Option<Token>> {
        if let Some(tok) = self.peeked.pop() {
            Ok(Some(tok))
        } else {
            self.lexer.next_token()
        }
    }

    /// Put token back for later
    fn push_back(&mut self, tok: Token) {
        self.peeked.push(tok);
    }

    /// Parse a single PDF object
    pub fn parse_object(&mut self) -> Result<Option<PdfObject>> {
        let Some(token) = self.next_token()? else {
            return Ok(None);
        };

        match token {
            Token::Null => Ok(Some(PdfObject::Null)),
            Token::True => Ok(Some(PdfObject::Bool(true))),
            Token::False => Ok(Some(PdfObject::Bool(false))),
            Token::Real(f) => Ok(Some(PdfObject::Real(f))),
            Token::String(s) => Ok(Some(PdfObject::String(s))),
            Token::HexString(s) => Ok(Some(PdfObject::String(s))),
            Token::Name(n) => Ok(Some(PdfObject::Name(n))),
            Token::ArrayStart => self.parse_array(),
            Token::DictStart => self.parse_dict_or_stream(),

            Token::Int(n) => {
                // Could be: Int, or start of "42 0 R" reference
                self.parse_int_or_ref(n)
            }

            _ => Err(PdfError::Parse {
                position: self.position(),
                message: format!("Unexpected token: {:?}", token),
            }),
        }
    }

    /// Parse integer or reference (42 vs 42 0 R)
    fn parse_int_or_ref(&mut self, first: i64) -> Result<Option<PdfObject>> {
        // Try to read second integer
        let Some(tok2) = self.next_token()? else {
            return Ok(Some(PdfObject::Int(first)));
        };

        let Token::Int(second) = tok2 else {
            // Not a reference, put token back
            self.push_back(tok2);
            return Ok(Some(PdfObject::Int(first)));
        };

        // Try to read 'R'
        let Some(tok3) = self.next_token()? else {
            self.push_back(Token::Int(second));
            return Ok(Some(PdfObject::Int(first)));
        };

        if tok3 == Token::Ref {
            // It's a reference: "42 0 R"
            Ok(Some(PdfObject::Ref(ObjRef::new(
                first as u32,
                second as u16,
            ))))
        } else {
            // Not a reference, put both tokens back
            self.push_back(tok3);
            self.push_back(Token::Int(second));
            Ok(Some(PdfObject::Int(first)))
        }
    }

    /// Parse array [...]
    fn parse_array(&mut self) -> Result<Option<PdfObject>> {
        let mut items = Vec::new();

        loop {
            let Some(token) = self.next_token()? else {
                return Err(PdfError::Parse {
                    position: self.position(),
                    message: "Unterminated array".into(),
                });
            };

            if token == Token::ArrayEnd {
                break;
            }

            // Put token back and parse as object
            self.push_back(token);
            if let Some(obj) = self.parse_object()? {
                items.push(obj);
            }
        }

        Ok(Some(PdfObject::Array(items)))
    }

    /// Parse dictionary or stream
    fn parse_dict_or_stream(&mut self) -> Result<Option<PdfObject>> {
        let mut dict = HashMap::new();

        loop {
            let Some(token) = self.next_token()? else {
                return Err(PdfError::Parse {
                    position: self.position(),
                    message: "Unterminated dictionary".into(),
                });
            };

            // End of dictionary
            if token == Token::DictEnd {
                break;
            }

            // Key must be a name
            let Token::Name(key) = token else {
                return Err(PdfError::Parse {
                    position: self.position(),
                    message: format!("Dictionary key must be name, got {:?}", token),
                });
            };

            // Value is any object
            let value = self.parse_object()?.ok_or_else(|| PdfError::Parse {
                position: self.position(),
                message: "Missing dictionary value".into(),
            })?;

            dict.insert(key, value);
        }

        // Check if followed by stream
        let pos_after_dict = self.lexer.position();
        if let Some(Token::Stream) = self.next_token()? {
            // It's a stream - read the data
            let data = self.read_stream_data(&dict)?;
            Ok(Some(PdfObject::Stream { dict, data }))
        } else {
            // Just a dictionary, restore position
            self.lexer.seek(pos_after_dict);
            self.peeked.clear();
            Ok(Some(PdfObject::Dict(dict)))
        }
    }

    /// Read stream data after "stream" keyword
    fn read_stream_data(&mut self, dict: &HashMap<String, PdfObject>) -> Result<Vec<u8>> {
        // Skip single newline after "stream"
        self.lexer.skip_whitespace();

        // Get length from dictionary
        let length = match dict.get("Length") {
            Some(PdfObject::Int(n)) => *n as usize,
            Some(PdfObject::Ref(_)) => {
                // Length is indirect - for now, search for endstream
                return self.read_stream_until_endstream();
            }
            _ => {
                return Err(PdfError::Parse {
                    position: self.position(),
                    message: "Stream missing Length".into(),
                });
            }
        };

        // Read exact bytes
        let start = self.lexer.position();
        let end = start + length;

        // Bounds check
        let data = self
            .lexer
            .data()
            .get(start..end)
            .ok_or_else(|| PdfError::Parse {
                position: start,
                message: "Stream data extends past EOF".into(),
            })?;

        let result = data.to_vec();
        self.lexer.seek(end);

        // Expect "endstream"
        self.lexer.skip_whitespace();
        if let Some(Token::EndStream) = self.next_token()? {
            Ok(result)
        } else {
            Err(PdfError::Parse {
                position: self.position(),
                message: "Missing endstream".into(),
            })
        }
    }

    /// Fallback: search for "endstream" marker
    fn read_stream_until_endstream(&mut self) -> Result<Vec<u8>> {
        let start = self.lexer.position();
        let marker = b"endstream";

        // Search for endstream
        let data = self.lexer.data();
        for i in start..data.len().saturating_sub(marker.len()) {
            if &data[i..i + marker.len()] == marker {
                let stream_data = data[start..i].to_vec();
                self.lexer.seek(i + marker.len());
                return Ok(stream_data);
            }
        }

        Err(PdfError::Parse {
            position: start,
            message: "Could not find endstream".into(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_primitives() {
        let mut parser = Parser::new(b"null true false 42 3.14");

        assert_eq!(parser.parse_object().unwrap(), Some(PdfObject::Null));
        assert_eq!(parser.parse_object().unwrap(), Some(PdfObject::Bool(true)));
        assert_eq!(parser.parse_object().unwrap(), Some(PdfObject::Bool(false)));
        assert_eq!(parser.parse_object().unwrap(), Some(PdfObject::Int(42)));
        assert_eq!(parser.parse_object().unwrap(), Some(PdfObject::Real(3.14)));
    }

    #[test]
    fn test_parse_reference() {
        let mut parser = Parser::new(b"5 0 R");
        let obj = parser.parse_object().unwrap().unwrap();
        assert_eq!(obj, PdfObject::Ref(ObjRef::new(5, 0)));
    }

    #[test]
    fn test_parse_array() {
        let mut parser = Parser::new(b"[1 2 3]");
        let obj = parser.parse_object().unwrap().unwrap();

        let expected = PdfObject::Array(vec![
            PdfObject::Int(1),
            PdfObject::Int(2),
            PdfObject::Int(3),
        ]);
        assert_eq!(obj, expected);
    }

    #[test]
    fn test_parse_dict() {
        let mut parser = Parser::new(b"<< /Type /Catalog /Version 1 >>");
        let obj = parser.parse_object().unwrap().unwrap();

        if let PdfObject::Dict(dict) = obj {
            assert_eq!(dict.get("Type"), Some(&PdfObject::Name("Catalog".into())));
            assert_eq!(dict.get("Version"), Some(&PdfObject::Int(1)));
        } else {
            panic!("Expected Dict");
        }
    }

    #[test]
    fn test_parse_nested() {
        let mut parser = Parser::new(b"<< /Kids [1 0 R 2 0 R] >>");
        let obj = parser.parse_object().unwrap().unwrap();

        if let PdfObject::Dict(dict) = obj {
            let kids = dict.get("Kids").unwrap().as_array().unwrap();
            assert_eq!(kids.len(), 2);
            assert_eq!(kids[0], PdfObject::Ref(ObjRef::new(1, 0)));
        } else {
            panic!("Expected Dict");
        }
    }
}
