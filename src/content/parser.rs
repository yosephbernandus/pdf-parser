use std::collections::HashMap;
use crate::error::{PdfError, Result};
use crate::font::FontEncoding;

/// Extracted text with position information
#[derive(Debug, Clone)]
pub struct TextSpan {
    pub text: String,
    pub x: f64,
    pub y: f64,
    pub font_size: f64,
    pub font_name: Option<String>,
}

/// Graphics state for text positioning
#[derive(Debug, Clone)]
struct GraphicsState {
    // Text matrix components [a, b, c, d, e, f]
    // Maps text space to user space
    text_matrix: [f64; 6],
    // Line matrix - reset at start of each line
    line_matrix: [f64; 6],
    // Current font size
    font_size: f64,
    // Current font name
    font_name: Option<String>,
    // Text leading (line spacing)
    leading: f64,
    // Character spacing
    char_spacing: f64,
    // Word spacing
    word_spacing: f64,
}

impl Default for GraphicsState {
    fn default() -> Self {
        Self {
            text_matrix: [1.0, 0.0, 0.0, 1.0, 0.0, 0.0],
            line_matrix: [1.0, 0.0, 0.0, 1.0, 0.0, 0.0],
            font_size: 12.0,
            font_name: None,
            leading: 0.0,
            char_spacing: 0.0,
            word_spacing: 0.0,
        }
    }
}

impl GraphicsState {
    /// Get current x position
    fn x(&self) -> f64 {
        self.text_matrix[4]
    }

    /// Get current y position
    fn y(&self) -> f64 {
        self.text_matrix[5]
    }
}

/// Content stream parser
pub struct ContentParser<'a> {
    data: &'a [u8],
    pos: usize,
    state: GraphicsState,
    state_stack: Vec<GraphicsState>,
    spans: Vec<TextSpan>,
    /// Font name -> encoding mapping
    font_encodings: HashMap<String, FontEncoding>,
}

impl<'a> ContentParser<'a> {
    pub fn new(data: &'a [u8]) -> Self {
        Self {
            data,
            pos: 0,
            state: GraphicsState::default(),
            state_stack: Vec::new(),
            spans: Vec::new(),
            font_encodings: HashMap::new(),
        }
    }

    /// Create parser with font encodings
    pub fn with_fonts(data: &'a [u8], font_encodings: HashMap<String, FontEncoding>) -> Self {
        Self {
            data,
            pos: 0,
            state: GraphicsState::default(),
            state_stack: Vec::new(),
            spans: Vec::new(),
            font_encodings,
        }
    }

    /// Parse content stream and extract text spans
    pub fn parse(mut self) -> Result<Vec<TextSpan>> {
        while self.pos < self.data.len() {
            self.skip_whitespace();

            if self.pos >= self.data.len() {
                break;
            }

            // Parse operands and operator
            let mut operands: Vec<Operand> = Vec::new();

            loop {
                self.skip_whitespace();
                if self.pos >= self.data.len() {
                    break;
                }

                let b = self.data[self.pos];

                // Check if this is an operator (alphabetic)
                if b.is_ascii_alphabetic() || b == b'\'' || b == b'"' {
                    let operator = self.read_operator();
                    self.execute_operator(&operator, &operands)?;
                    break;
                }

                // Parse operand
                if let Some(operand) = self.parse_operand()? {
                    operands.push(operand);
                } else {
                    break;
                }
            }
        }

        // Merge adjacent spans on the same line
        Ok(self.merge_adjacent_spans())
    }

    /// Merge adjacent text spans that are on the same line and close together
    fn merge_adjacent_spans(&self) -> Vec<TextSpan> {
        if self.spans.is_empty() {
            return Vec::new();
        }

        // Sort spans by y (descending = top to bottom) then x (ascending = left to right)
        let mut sorted_spans = self.spans.clone();
        sorted_spans.sort_by(|a, b| {
            // First compare y (with tolerance for same line)
            let y_diff = b.y - a.y;
            if y_diff.abs() > a.font_size * 0.3 {
                return y_diff.partial_cmp(&0.0).unwrap_or(std::cmp::Ordering::Equal);
            }
            // Same line - sort by x
            a.x.partial_cmp(&b.x).unwrap_or(std::cmp::Ordering::Equal)
        });

        let mut merged: Vec<TextSpan> = Vec::new();

        for span in sorted_spans {
            if let Some(last) = merged.last_mut() {
                // Check if this span is on the same line (within tolerance)
                let y_tolerance = last.font_size * 0.3;
                let same_line = (span.y - last.y).abs() <= y_tolerance;

                if same_line && last.font_name == span.font_name {
                    // Estimate expected position based on accumulated text length
                    // Use font_size * 0.5 as average character width estimate
                    let char_width = last.font_size * 0.5;
                    let expected_x = last.x + (last.text.len() as f64 * char_width);
                    let gap = span.x - expected_x;

                    // If gap is small, merge without space
                    // If gap is moderate (word boundary), merge with space
                    // If gap is large, start new span
                    if gap < char_width * 0.8 && gap > -char_width * 2.0 {
                        // Small gap - just append
                        last.text.push_str(&span.text);
                    } else if gap < char_width * 2.0 {
                        // Word boundary - append with space
                        last.text.push(' ');
                        last.text.push_str(&span.text);
                    } else {
                        // Large gap - new span
                        merged.push(span);
                    }
                } else {
                    // Different line or font - new span
                    merged.push(span);
                }
            } else {
                merged.push(span);
            }
        }

        merged
    }

    fn skip_whitespace(&mut self) {
        while self.pos < self.data.len() {
            match self.data[self.pos] {
                b' ' | b'\t' | b'\n' | b'\r' | 0x0C | 0x00 => self.pos += 1,
                b'%' => {
                    // Skip comment
                    while self.pos < self.data.len() && self.data[self.pos] != b'\n' {
                        self.pos += 1;
                    }
                }
                _ => break,
            }
        }
    }

    fn read_operator(&mut self) -> String {
        let start = self.pos;
        while self.pos < self.data.len() {
            let b = self.data[self.pos];
            if b.is_ascii_alphabetic() || b == b'*' || b == b'\'' || b == b'"' {
                self.pos += 1;
            } else {
                break;
            }
        }
        String::from_utf8_lossy(&self.data[start..self.pos]).to_string()
    }

    fn parse_operand(&mut self) -> Result<Option<Operand>> {
        self.skip_whitespace();

        if self.pos >= self.data.len() {
            return Ok(None);
        }

        let b = self.data[self.pos];

        match b {
            // Number (int or real)
            b'+' | b'-' | b'.' | b'0'..=b'9' => {
                let num = self.read_number()?;
                Ok(Some(Operand::Number(num)))
            }
            // Literal string
            b'(' => {
                let s = self.read_string()?;
                Ok(Some(Operand::String(s)))
            }
            // Hex string
            b'<' => {
                self.pos += 1;
                if self.pos < self.data.len() && self.data[self.pos] == b'<' {
                    // It's a dictionary - skip it
                    self.skip_dict()?;
                    Ok(None)
                } else {
                    let s = self.read_hex_string()?;
                    Ok(Some(Operand::String(s)))
                }
            }
            // Name
            b'/' => {
                let name = self.read_name();
                Ok(Some(Operand::Name(name)))
            }
            // Array
            b'[' => {
                let arr = self.read_array()?;
                Ok(Some(Operand::Array(arr)))
            }
            // End array or other delimiter - not an operand
            b']' | b'>' => Ok(None),
            // Alphabetic - it's an operator, not operand
            _ if b.is_ascii_alphabetic() => Ok(None),
            // Unknown
            _ => {
                self.pos += 1;
                Ok(None)
            }
        }
    }

    fn read_number(&mut self) -> Result<f64> {
        let start = self.pos;

        // Optional sign
        if self.pos < self.data.len() && matches!(self.data[self.pos], b'+' | b'-') {
            self.pos += 1;
        }

        // Digits and decimal point
        while self.pos < self.data.len() {
            match self.data[self.pos] {
                b'0'..=b'9' | b'.' => self.pos += 1,
                _ => break,
            }
        }

        let num_str = std::str::from_utf8(&self.data[start..self.pos])
            .map_err(|_| PdfError::Parse {
                position: start,
                message: "Invalid number".into(),
            })?;

        num_str.parse().map_err(|_| PdfError::Parse {
            position: start,
            message: format!("Invalid number: {}", num_str),
        })
    }

    fn read_string(&mut self) -> Result<Vec<u8>> {
        self.pos += 1; // Skip '('
        let mut result = Vec::new();
        let mut depth = 1;

        while self.pos < self.data.len() && depth > 0 {
            let b = self.data[self.pos];
            self.pos += 1;

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
                b'\\' if self.pos < self.data.len() => {
                    let escaped = self.data[self.pos];
                    self.pos += 1;
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
                            // Octal
                            let mut val = (escaped - b'0') as u8;
                            for _ in 0..2 {
                                if self.pos < self.data.len() {
                                    let d = self.data[self.pos];
                                    if matches!(d, b'0'..=b'7') {
                                        self.pos += 1;
                                        val = val * 8 + (d - b'0');
                                    } else {
                                        break;
                                    }
                                }
                            }
                            result.push(val);
                        }
                        b'\r' | b'\n' => {
                            // Line continuation
                            if escaped == b'\r' && self.pos < self.data.len() && self.data[self.pos] == b'\n' {
                                self.pos += 1;
                            }
                        }
                        _ => result.push(escaped),
                    }
                }
                _ => result.push(b),
            }
        }

        Ok(result)
    }

    fn read_hex_string(&mut self) -> Result<Vec<u8>> {
        let mut hex_chars = Vec::new();

        while self.pos < self.data.len() {
            let b = self.data[self.pos];
            self.pos += 1;

            match b {
                b'>' => break,
                b'0'..=b'9' | b'a'..=b'f' | b'A'..=b'F' => hex_chars.push(b),
                b' ' | b'\t' | b'\n' | b'\r' => continue,
                _ => continue,
            }
        }

        // Pad if odd
        if hex_chars.len() % 2 == 1 {
            hex_chars.push(b'0');
        }

        let result: Vec<u8> = hex_chars
            .chunks(2)
            .map(|pair| {
                let h = hex_val(pair[0]);
                let l = hex_val(pair[1]);
                (h << 4) | l
            })
            .collect();

        Ok(result)
    }

    fn read_name(&mut self) -> String {
        self.pos += 1; // Skip '/'
        let start = self.pos;

        while self.pos < self.data.len() {
            let b = self.data[self.pos];
            if b.is_ascii_alphanumeric() || b == b'_' || b == b'-' || b == b'+' || b == b'.' {
                self.pos += 1;
            } else {
                break;
            }
        }

        String::from_utf8_lossy(&self.data[start..self.pos]).to_string()
    }

    fn read_array(&mut self) -> Result<Vec<Operand>> {
        self.pos += 1; // Skip '['
        let mut items = Vec::new();

        loop {
            self.skip_whitespace();
            if self.pos >= self.data.len() || self.data[self.pos] == b']' {
                self.pos += 1; // Skip ']'
                break;
            }

            if let Some(operand) = self.parse_operand()? {
                items.push(operand);
            } else {
                self.pos += 1; // Skip unknown
            }
        }

        Ok(items)
    }

    fn skip_dict(&mut self) -> Result<()> {
        self.pos += 1; // Skip second '<'
        let mut depth = 1;

        while self.pos < self.data.len() && depth > 0 {
            if self.pos + 1 < self.data.len() {
                if self.data[self.pos] == b'<' && self.data[self.pos + 1] == b'<' {
                    depth += 1;
                    self.pos += 2;
                    continue;
                }
                if self.data[self.pos] == b'>' && self.data[self.pos + 1] == b'>' {
                    depth -= 1;
                    self.pos += 2;
                    continue;
                }
            }
            self.pos += 1;
        }

        Ok(())
    }

    fn execute_operator(&mut self, op: &str, operands: &[Operand]) -> Result<()> {
        match op {
            // Graphics state
            "q" => {
                self.state_stack.push(self.state.clone());
            }
            "Q" => {
                if let Some(state) = self.state_stack.pop() {
                    self.state = state;
                }
            }

            // Text state operators
            "BT" => {
                // Begin text - reset text matrix
                self.state.text_matrix = [1.0, 0.0, 0.0, 1.0, 0.0, 0.0];
                self.state.line_matrix = [1.0, 0.0, 0.0, 1.0, 0.0, 0.0];
            }
            "ET" => {
                // End text
            }

            // Font: /FontName size Tf
            "Tf" => {
                if operands.len() >= 2 {
                    if let Operand::Name(name) = &operands[operands.len() - 2] {
                        self.state.font_name = Some(name.clone());
                    }
                    if let Operand::Number(size) = &operands[operands.len() - 1] {
                        self.state.font_size = *size;
                    }
                }
            }

            // Text leading: leading TL
            "TL" => {
                if let Some(Operand::Number(leading)) = operands.last() {
                    self.state.leading = *leading;
                }
            }

            // Character spacing: spacing Tc
            "Tc" => {
                if let Some(Operand::Number(spacing)) = operands.last() {
                    self.state.char_spacing = *spacing;
                }
            }

            // Word spacing: spacing Tw
            "Tw" => {
                if let Some(Operand::Number(spacing)) = operands.last() {
                    self.state.word_spacing = *spacing;
                }
            }

            // Text positioning: tx ty Td
            "Td" => {
                if operands.len() >= 2 {
                    if let (Operand::Number(tx), Operand::Number(ty)) =
                        (&operands[operands.len() - 2], &operands[operands.len() - 1])
                    {
                        // Translate from line matrix
                        self.state.line_matrix[4] += tx;
                        self.state.line_matrix[5] += ty;
                        self.state.text_matrix = self.state.line_matrix;
                    }
                }
            }

            // Text positioning with leading: tx ty TD
            "TD" => {
                if operands.len() >= 2 {
                    if let (Operand::Number(tx), Operand::Number(ty)) =
                        (&operands[operands.len() - 2], &operands[operands.len() - 1])
                    {
                        self.state.leading = -ty;
                        self.state.line_matrix[4] += tx;
                        self.state.line_matrix[5] += ty;
                        self.state.text_matrix = self.state.line_matrix;
                    }
                }
            }

            // Set text matrix: a b c d e f Tm
            "Tm" => {
                if operands.len() >= 6 {
                    let nums: Vec<f64> = operands
                        .iter()
                        .filter_map(|o| {
                            if let Operand::Number(n) = o {
                                Some(*n)
                            } else {
                                None
                            }
                        })
                        .collect();

                    if nums.len() >= 6 {
                        self.state.text_matrix = [nums[0], nums[1], nums[2], nums[3], nums[4], nums[5]];
                        self.state.line_matrix = self.state.text_matrix;
                    }
                }
            }

            // Move to next line: T*
            "T*" => {
                self.state.line_matrix[4] = self.state.line_matrix[4];
                self.state.line_matrix[5] -= self.state.leading;
                self.state.text_matrix = self.state.line_matrix;
            }

            // Show text: (string) Tj
            "Tj" => {
                if let Some(Operand::String(bytes)) = operands.last() {
                    self.add_text_span(bytes);
                }
            }

            // Show text with spacing: [(string) num (string) ...] TJ
            "TJ" => {
                if let Some(Operand::Array(items)) = operands.last() {
                    // Collect consecutive strings with small adjustments
                    let mut combined_bytes: Vec<u8> = Vec::new();
                    let mut span_start_x = self.state.x();
                    let mut span_start_y = self.state.y();
                    let mut has_content = false;

                    for item in items {
                        match item {
                            Operand::String(bytes) => {
                                if !has_content {
                                    span_start_x = self.state.x();
                                    span_start_y = self.state.y();
                                    has_content = true;
                                }
                                combined_bytes.extend(bytes);
                                // Advance text position
                                let advance = bytes.len() as f64 * self.state.font_size * 0.5;
                                self.state.text_matrix[4] += advance;
                            }
                            Operand::Number(n) => {
                                // Adjust position (negative = move right)
                                let adjust = -n / 1000.0 * self.state.font_size;

                                // If adjustment is large (> 200 units = word space), flush current span
                                if n.abs() > 200.0 && has_content {
                                    self.add_text_span_at(&combined_bytes, span_start_x, span_start_y);
                                    combined_bytes.clear();
                                    has_content = false;
                                }

                                self.state.text_matrix[4] += adjust;
                            }
                            _ => {}
                        }
                    }

                    // Flush remaining content
                    if has_content && !combined_bytes.is_empty() {
                        self.add_text_span_at(&combined_bytes, span_start_x, span_start_y);
                    }
                }
            }

            // Move to next line and show: (string) '
            "'" => {
                // T* then Tj
                self.state.line_matrix[5] -= self.state.leading;
                self.state.text_matrix = self.state.line_matrix;

                if let Some(Operand::String(bytes)) = operands.last() {
                    self.add_text_span(bytes);
                }
            }

            // Set spacing, move, and show: aw ac (string) "
            "\"" => {
                if operands.len() >= 3 {
                    if let Operand::Number(aw) = &operands[0] {
                        self.state.word_spacing = *aw;
                    }
                    if let Operand::Number(ac) = &operands[1] {
                        self.state.char_spacing = *ac;
                    }
                }

                self.state.line_matrix[5] -= self.state.leading;
                self.state.text_matrix = self.state.line_matrix;

                if let Some(Operand::String(bytes)) = operands.last() {
                    self.add_text_span(bytes);
                }
            }

            _ => {
                // Unknown operator - ignore
            }
        }

        Ok(())
    }

    fn add_text_span(&mut self, bytes: &[u8]) {
        let x = self.state.x();
        let y = self.state.y();
        self.add_text_span_at(bytes, x, y);

        // Advance text position (simplified - doesn't account for actual glyph widths)
        let advance = bytes.len() as f64 * self.state.font_size * 0.5;
        self.state.text_matrix[4] += advance;
    }

    fn add_text_span_at(&mut self, bytes: &[u8], x: f64, y: f64) {
        // Decode bytes using font encoding if available
        let text = if let Some(font_name) = &self.state.font_name {
            if let Some(encoding) = self.font_encodings.get(font_name) {
                encoding.decode_bytes(bytes)
            } else {
                self.decode_default(bytes)
            }
        } else {
            self.decode_default(bytes)
        };

        let text = text.trim().to_string();

        if !text.is_empty() {
            self.spans.push(TextSpan {
                text,
                x,
                y,
                font_size: self.state.font_size,
                font_name: self.state.font_name.clone(),
            });
        }
    }

    /// Default decoding for bytes (Latin-1)
    fn decode_default(&self, bytes: &[u8]) -> String {
        bytes
            .iter()
            .map(|&b| {
                if b >= 32 && b < 127 {
                    b as char
                } else if b >= 160 {
                    // Latin-1 supplement
                    char::from_u32(b as u32).unwrap_or('?')
                } else {
                    ' '
                }
            })
            .collect()
    }
}

/// Operand types in content stream
#[derive(Debug, Clone)]
enum Operand {
    Number(f64),
    String(Vec<u8>),
    Name(String),
    Array(Vec<Operand>),
}

fn hex_val(b: u8) -> u8 {
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
    fn test_simple_text() {
        let content = b"BT /F1 12 Tf 100 700 Td (Hello World) Tj ET";
        let parser = ContentParser::new(content);
        let spans = parser.parse().unwrap();

        assert_eq!(spans.len(), 1);
        assert_eq!(spans[0].text, "Hello World");
        assert_eq!(spans[0].x, 100.0);
        assert_eq!(spans[0].y, 700.0);
        assert_eq!(spans[0].font_size, 12.0);
    }

    #[test]
    fn test_multiple_spans() {
        let content = b"BT /F1 10 Tf 50 500 Td (First) Tj 0 -20 Td (Second) Tj ET";
        let parser = ContentParser::new(content);
        let spans = parser.parse().unwrap();

        assert_eq!(spans.len(), 2);
        assert_eq!(spans[0].text, "First");
        assert_eq!(spans[1].text, "Second");
        assert_eq!(spans[1].y, 480.0); // 500 - 20
    }

    #[test]
    fn test_tj_array() {
        let content = b"BT /F1 12 Tf 100 700 Td [(Hello) -100 (World)] TJ ET";
        let parser = ContentParser::new(content);
        let spans = parser.parse().unwrap();

        // Small adjustment (-100) causes spans to be merged
        assert_eq!(spans.len(), 1);
        assert_eq!(spans[0].text, "HelloWorld");
    }

    #[test]
    fn test_tj_array_with_large_gap() {
        // Large adjustment (-2000) creates separate spans
        let content = b"BT /F1 12 Tf 100 700 Td [(Hello) -2000 (World)] TJ ET";
        let parser = ContentParser::new(content);
        let spans = parser.parse().unwrap();

        assert_eq!(spans.len(), 2);
        assert_eq!(spans[0].text, "Hello");
        assert_eq!(spans[1].text, "World");
    }
}
