use crate::error::{PdfError, Result};

#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    // Literals
    Null,
    True,
    False,
    Int(i64),
    Real(f64),

    // Strings
    String(Vec<u8>),
    HexString(Vec<u8>),

    // Names
    Name(String),

    // Delimiters
    ArrayStart, // [
    ArrayEnd,   // ]
    DictStart,  // <<
    DictEnd,    // >>

    // Object makers
    Obj,       // obj
    EndObj,    // endobj
    Stream,    // stream
    EndStream, // endstream

    // Reference
    Ref,       // R
    Trailer,   // trailer
    StartXRef, // startxref
    XRef,      // xref <- referenced in read_keyword() but not in enum
}

pub struct Lexer<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> Lexer<'a> {
    pub fn new(data: &'a [u8]) -> Self {
        Self { data, pos: 0 }
    }

    pub fn data(&self) -> &'a [u8] {
        self.data
    }

    pub fn position(&self) -> usize {
        self.pos
    }

    pub fn seek(&mut self, pos: usize) {
        self.pos = pos;
    }

    /// Peek at current byte without consuming
    fn peek(&self) -> Option<u8> {
        self.data.get(self.pos).copied()
    }

    /// Read and consume one byte
    fn read_byte(&mut self) -> Result<u8> {
        let b = self.peek().ok_or_else(|| PdfError::Parse {
            position: self.pos,
            message: "Unexpected end of file".into(),
        })?;
        self.pos += 1;
        Ok(b)
    }

    /// Skip whitespace and comments
    pub fn skip_whitespace(&mut self) {
        while self.pos < self.data.len() {
            match self.data[self.pos] {
                // PDF whitespace characters
                b' ' | b'\t' | b'\n' | b'\r' | 0x0C | 0x00 => {
                    self.pos += 1;
                }
                // Comment - skip to end of line
                b'%' => {
                    self.pos += 1;
                    while self.pos < self.data.len() {
                        let b = self.data[self.pos];
                        self.pos += 1;
                        if b == b'\n' || b == b'\r' {
                            break;
                        }
                    }
                }
                _ => break,
            }
        }
    }

    /// Main entry point - get next token
    pub fn next_token(&mut self) -> Result<Option<Token>> {
        self.skip_whitespace();

        let Some(b) = self.peek() else {
            return Ok(None); // EOF
        };

        match b {
            b'[' => {
                self.pos += 1;
                Ok(Some(Token::ArrayStart))
            }
            b']' => {
                self.pos += 1;
                Ok(Some(Token::ArrayEnd))
            }
            b'<' => {
                self.pos += 1;
                if self.peek() == Some(b'<') {
                    self.pos += 1;
                    Ok(Some(Token::DictStart))
                } else {
                    self.read_hex_string().map(|s| Some(Token::HexString(s)))
                }
            }
            b'>' => {
                self.pos += 1;
                if self.peek() == Some(b'>') {
                    self.pos += 1;
                    Ok(Some(Token::DictEnd))
                } else {
                    Err(PdfError::Parse {
                        position: self.pos,
                        message: "Unexpected '>'".into(),
                    })
                }
            }
            b'(' => self.read_literal_string().map(|s| Some(Token::String(s))),
            b'/' => self.read_name().map(|n| Some(Token::Name(n))),
            b'+' | b'-' | b'.' | b'0'..=b'9' => self.read_number().map(Some),
            b'a'..=b'z' | b'A'..=b'Z' => self.read_keyword().map(Some),
            _ => Err(PdfError::Parse {
                position: self.pos,
                message: format!("Unexpected byte: 0x{:02X}", b),
            }),
        }
    }

    /// Read integer or real number
    fn read_number(&mut self) -> Result<Token> {
        let start = self.pos;
        let mut has_decimal = false;

        // Optional sign
        if matches!(self.peek(), Some(b'+') | Some(b'-')) {
            self.pos += 1;
        }

        // Digits and optional decimal point
        while let Some(b) = self.peek() {
            match b {
                b'0'..=b'9' => self.pos += 1,
                b'.' if !has_decimal => {
                    has_decimal = true;
                    self.pos += 1;
                }
                _ => break,
            }
        }

        let num_str =
            std::str::from_utf8(&self.data[start..self.pos]).map_err(|_| PdfError::Parse {
                position: start,
                message: "Invalid number encoding".into(),
            })?;

        if has_decimal {
            let f: f64 = num_str.parse().map_err(|_| PdfError::Parse {
                position: start,
                message: format!("Invalid real number: {}", num_str),
            })?;
            Ok(Token::Real(f))
        } else {
            let n: i64 = num_str.parse().map_err(|_| PdfError::Parse {
                position: start,
                message: format!("Invalid integer: {}", num_str),
            })?;
            Ok(Token::Int(n))
        }
    }

    /// Read keyword (null, true, false, obj, etc.)
    fn read_keyword(&mut self) -> Result<Token> {
        let start = self.pos;

        while let Some(b'a'..=b'z' | b'A'..=b'Z') = self.peek() {
            self.pos += 1;
        }

        let keyword =
            std::str::from_utf8(&self.data[start..self.pos]).map_err(|_| PdfError::InvalidUtf8)?;

        match keyword {
            "null" => Ok(Token::Null),
            "true" => Ok(Token::True),
            "false" => Ok(Token::False),
            "obj" => Ok(Token::Obj),
            "endobj" => Ok(Token::EndObj),
            "stream" => Ok(Token::Stream),
            "endstream" => Ok(Token::EndStream),
            "R" => Ok(Token::Ref),
            "xref" => Ok(Token::XRef),
            "trailer" => Ok(Token::Trailer),
            "startxref" => Ok(Token::StartXRef),
            _ => Err(PdfError::Parse {
                position: start,
                message: format!("Unknown keyword: {}", keyword),
            }),
        }
    }

    /// Read literal string (...)
    fn read_literal_string(&mut self) -> Result<Vec<u8>> {
        self.pos += 1; // Skip opening '('

        let mut result = Vec::new();
        let mut depth = 1;

        while depth > 0 {
            let b = self.read_byte()?;

            match b {
                b'(' => {
                    depth += 1;
                    result.push(b);
                }
                b')' => {
                    depth -= 1;
                    if depth > 0 {
                        result.push(b);
                    }
                }
                b'\\' => {
                    let escaped = self.read_byte()?;
                    match escaped {
                        b'n' => result.push(b'\n'),
                        b'r' => result.push(b'\r'),
                        b't' => result.push(b'\t'),
                        b'b' => result.push(0x08),
                        b'f' => result.push(0x0C),
                        b'(' => result.push(b'('),
                        b')' => result.push(b')'),
                        b'\\' => result.push(b'\\'),
                        b'0'..=b'7' => {
                            // Octal escape (1-3 digits)
                            let mut octal = (escaped - b'0') as u32;
                            for _ in 0..2 {
                                if let Some(d @ b'0'..=b'7') = self.peek() {
                                    self.pos += 1;
                                    octal = octal * 8 + (d - b'0') as u32;
                                } else {
                                    break;
                                }
                            }
                            result.push(octal as u8);
                        }
                        b'\r' => {
                            // Line continuation - skip \r and optional \n
                            if self.peek() == Some(b'\n') {
                                self.pos += 1;
                            }
                        }
                        b'\n' => {
                            // Line continuation - just skip
                        }
                        _ => result.push(escaped),
                    }
                }
                _ => result.push(b),
            }
        }

        Ok(result)
    }

    /// Read hex string <...>
    fn read_hex_string(&mut self) -> Result<Vec<u8>> {
        // Opening '<' already consumed
        let mut hex_chars = Vec::new();

        loop {
            // Skip whitespace inside hex string
            while matches!(self.peek(), Some(b' ' | b'\t' | b'\n' | b'\r')) {
                self.pos += 1;
            }

            let b = self.read_byte()?;
            match b {
                b'>' => break,
                b'0'..=b'9' | b'a'..=b'f' | b'A'..=b'F' => {
                    hex_chars.push(b);
                }
                _ => {
                    return Err(PdfError::Parse {
                        position: self.pos - 1,
                        message: format!("Invalid hex char: 0x{:02X}", b),
                    });
                }
            }
        }

        // Pad with 0 if odd length
        if hex_chars.len() % 2 == 1 {
            hex_chars.push(b'0');
        }

        // Convert hex pairs to bytes
        hex_chars
            .chunks(2)
            .map(|pair| {
                let high = hex_value(pair[0]);
                let low = hex_value(pair[1]);
                Ok((high << 4) | low)
            })
            .collect()
    }

    /// Read name /...
    fn read_name(&mut self) -> Result<String> {
        self.pos += 1; // Skip '/'

        let mut name = Vec::new();

        while let Some(b) = self.peek() {
            match b {
                // Delimiters end the name
                b' ' | b'\t' | b'\n' | b'\r' | 0x0C | 0x00 | b'(' | b')' | b'<' | b'>' | b'['
                | b']' | b'{' | b'}' | b'/' | b'%' => break,

                // # introduces hex escape
                b'#' => {
                    self.pos += 1;
                    let h1 = self.read_byte()?;
                    let h2 = self.read_byte()?;
                    name.push((hex_value(h1) << 4) | hex_value(h2));
                }

                _ => {
                    self.pos += 1;
                    name.push(b);
                }
            }
        }

        String::from_utf8(name).map_err(|_| PdfError::InvalidUtf8)
    }
}

/// Convert hex digit to value
fn hex_value(b: u8) -> u8 {
    match b {
        b'0'..=b'9' => b - b'0',
        b'a'..=b'f' => b - b'a' + 10,
        b'A'..=b'F' => b - b'A' + 10,
        _ => 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_tokens() {
        let mut lexer = Lexer::new(b"42 3.14 true null");

        assert_eq!(lexer.next_token().unwrap(), Some(Token::Int(42)));
        assert_eq!(lexer.next_token().unwrap(), Some(Token::Real(3.14)));
        assert_eq!(lexer.next_token().unwrap(), Some(Token::True));
        assert_eq!(lexer.next_token().unwrap(), Some(Token::Null));
        assert_eq!(lexer.next_token().unwrap(), None);
    }

    #[test]
    fn test_string_with_escapes() {
        let mut lexer = Lexer::new(b"(Hello\\nWorld)");
        let token = lexer.next_token().unwrap().unwrap();
        assert_eq!(token, Token::String(b"Hello\nWorld".to_vec()));
    }

    #[test]
    fn test_nested_parens() {
        let mut lexer = Lexer::new(b"(a(b)c)");
        let token = lexer.next_token().unwrap().unwrap();
        assert_eq!(token, Token::String(b"a(b)c".to_vec()));
    }

    #[test]
    fn test_hex_string() {
        let mut lexer = Lexer::new(b"<48656C6C6F>");
        let token = lexer.next_token().unwrap().unwrap();
        assert_eq!(token, Token::HexString(b"Hello".to_vec()));
    }

    #[test]
    fn test_dictionary() {
        let mut lexer = Lexer::new(b"<< /Type /Catalog >>");

        assert_eq!(lexer.next_token().unwrap(), Some(Token::DictStart));
        assert_eq!(
            lexer.next_token().unwrap(),
            Some(Token::Name("Type".into()))
        );
        assert_eq!(
            lexer.next_token().unwrap(),
            Some(Token::Name("Catalog".into()))
        );
        assert_eq!(lexer.next_token().unwrap(), Some(Token::DictEnd));
    }

    #[test]
    fn test_name_with_hex_escape() {
        let mut lexer = Lexer::new(b"/Font#20Name");
        let token = lexer.next_token().unwrap().unwrap();
        assert_eq!(token, Token::Name("Font Name".into()));
    }
}
