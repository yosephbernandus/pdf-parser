use std::collections::HashMap;

use crate::content::{ContentParser, TextSpan};
use crate::decode::decode_stream;
use crate::error::{PdfError, Result};
use crate::font::{parse_tounicode_cmap, FontEncoding};
use crate::parser::Parser;
use crate::types::{ObjRef, PdfObject};

/// Entry in the cross-reference table
#[derive(Debug, Clone)]
pub struct XRefEntry {
    pub offset: usize,
    pub generation: u16,
    pub in_use: bool,
}

/// Parsed PDF document
pub struct Document<'a> {
    data: &'a [u8],
    /// Object number -> xref entry
    xref: HashMap<u32, XRefEntry>,
    /// Trailer dictionary
    trailer: HashMap<String, PdfObject>,
    /// Cache of parsed objects
    cache: HashMap<ObjRef, PdfObject>,
}

impl<'a> Document<'a> {
    /// Parse a PDF document from bytes
    pub fn parse(data: &'a [u8]) -> Result<Self> {
        // Verify PDF header
        if !data.starts_with(b"%PDF-") {
            return Err(PdfError::MissingHeader);
        }

        // Find startxref position
        let startxref_pos = Self::find_startxref(data)?;

        // Parse xref offset
        let xref_offset = Self::parse_startxref(data, startxref_pos)?;

        // Parse xref table and trailer
        let (xref, trailer) = Self::parse_xref_and_trailer(data, xref_offset)?;

        Ok(Document {
            data,
            xref,
            trailer,
            cache: HashMap::new(),
        })
    }

    /// Find "startxref" by searching backwards from EOF
    fn find_startxref(data: &[u8]) -> Result<usize> {
        let search = b"startxref";
        let search_region = data.len().saturating_sub(1024); // Last 1KB

        for i in (search_region..data.len().saturating_sub(search.len())).rev() {
            if &data[i..i + search.len()] == search {
                return Ok(i);
            }
        }

        Err(PdfError::MissingEof)
    }

    /// Parse the xref offset after "startxref"
    fn parse_startxref(data: &[u8], pos: usize) -> Result<usize> {
        let mut parser = Parser::new(data);

        // Skip "startxref" keyword
        let after_keyword = pos + b"startxref".len();
        parser.seek(after_keyword);

        // Parse the offset number
        match parser.parse_object()? {
            Some(PdfObject::Int(offset)) => Ok(offset as usize),
            _ => Err(PdfError::Parse {
                position: pos,
                message: "Expected xref offset after startxref".into(),
            }),
        }
    }

    /// Parse xref table and trailer dictionary, following Prev chain
    fn parse_xref_and_trailer(
        data: &[u8],
        offset: usize,
    ) -> Result<(HashMap<u32, XRefEntry>, HashMap<String, PdfObject>)> {
        let mut xref = HashMap::new();
        let mut current_offset = offset;
        let mut final_trailer: Option<HashMap<String, PdfObject>> = None;

        // Follow the Prev chain to collect all xref entries
        loop {
            // Check if this is a traditional xref table or xref stream
            if current_offset < data.len() && data[current_offset..].starts_with(b"xref") {
                // Traditional xref table
                Self::parse_traditional_xref(data, current_offset, &mut xref)?;

                // Find and parse trailer
                let trailer = Self::find_and_parse_trailer(data, current_offset)?;

                // Keep the most recent trailer (first one we encounter)
                if final_trailer.is_none() {
                    final_trailer = Some(trailer.clone());
                }

                // Check for Prev pointer to follow the chain
                if let Some(prev_offset) = trailer.get("Prev").and_then(|p| p.as_int()) {
                    current_offset = prev_offset as usize;
                } else {
                    break;
                }
            } else {
                // Might be an xref stream (PDF 1.5+)
                // TODO: Implement xref stream parsing
                if final_trailer.is_some() {
                    // We have at least one valid xref, continue
                    break;
                }
                return Err(PdfError::InvalidStructure(
                    "XRef streams not yet supported".into(),
                ));
            }
        }

        let trailer = final_trailer.ok_or_else(|| {
            PdfError::InvalidStructure("No valid trailer found".into())
        })?;

        Ok((xref, trailer))
    }

    /// Parse traditional xref table
    fn parse_traditional_xref(
        data: &[u8],
        offset: usize,
        xref: &mut HashMap<u32, XRefEntry>,
    ) -> Result<()> {
        let mut pos = offset + b"xref".len();

        // Skip whitespace after "xref"
        while pos < data.len() && matches!(data[pos], b' ' | b'\t' | b'\n' | b'\r') {
            pos += 1;
        }

        // Parse subsections
        loop {
            // Check if we hit "trailer"
            if pos + 7 <= data.len() && &data[pos..pos + 7] == b"trailer" {
                break;
            }

            // Check bounds
            if pos >= data.len() {
                break;
            }

            // Parse subsection header: "start_obj count"
            // Find end of line
            let line_end = data[pos..]
                .iter()
                .position(|&b| b == b'\n' || b == b'\r')
                .map(|p| pos + p)
                .unwrap_or(data.len());

            let header_line = std::str::from_utf8(&data[pos..line_end])
                .map_err(|_| PdfError::InvalidXref)?;

            let parts: Vec<&str> = header_line.split_whitespace().collect();
            if parts.len() != 2 {
                break; // Not a valid subsection header, probably hit trailer
            }

            let start_obj: u32 = parts[0].parse().map_err(|_| PdfError::InvalidXref)?;
            let count: u32 = parts[1].parse().map_err(|_| PdfError::InvalidXref)?;

            // Move past the header line
            pos = line_end;
            // Skip line ending (LF or CRLF)
            if pos < data.len() && data[pos] == b'\r' {
                pos += 1;
            }
            if pos < data.len() && data[pos] == b'\n' {
                pos += 1;
            }

            // Parse entries - each entry is on its own line
            for i in 0..count {
                // Find end of this entry line
                let entry_end = data[pos..]
                    .iter()
                    .position(|&b| b == b'\n' || b == b'\r')
                    .map(|p| pos + p)
                    .unwrap_or(data.len());

                if entry_end <= pos {
                    return Err(PdfError::InvalidXref);
                }

                let entry_line = &data[pos..entry_end];

                // Entry format: "nnnnnnnnnn ggggg f" or "nnnnnnnnnn ggggg n"
                // Minimum 18 bytes (10 + 1 + 5 + 1 + 1)
                if entry_line.len() < 17 {
                    return Err(PdfError::InvalidXref);
                }

                // Parse offset (first 10 chars)
                let offset_str = std::str::from_utf8(&entry_line[0..10])
                    .map_err(|_| PdfError::InvalidXref)?;
                let entry_offset: usize = offset_str
                    .trim()
                    .parse()
                    .map_err(|_| PdfError::InvalidXref)?;

                // Parse generation (chars 11-15)
                let gen_str = std::str::from_utf8(&entry_line[11..16])
                    .map_err(|_| PdfError::InvalidXref)?;
                let generation: u16 = gen_str
                    .trim()
                    .parse()
                    .map_err(|_| PdfError::InvalidXref)?;

                // Parse in-use flag (char 17)
                let flag = entry_line[17];
                let in_use = flag == b'n';

                if in_use {
                    xref.insert(
                        start_obj + i,
                        XRefEntry {
                            offset: entry_offset,
                            generation,
                            in_use,
                        },
                    );
                }

                // Move to next line
                pos = entry_end;
                // Skip line ending
                if pos < data.len() && data[pos] == b'\r' {
                    pos += 1;
                }
                if pos < data.len() && data[pos] == b'\n' {
                    pos += 1;
                }
            }
        }

        Ok(())
    }

    /// Find and parse trailer dictionary
    fn find_and_parse_trailer(
        data: &[u8],
        xref_offset: usize,
    ) -> Result<HashMap<String, PdfObject>> {
        // Search for "trailer" after xref
        let search = b"trailer";
        let mut pos = xref_offset;

        while pos + search.len() < data.len() {
            if &data[pos..pos + search.len()] == search {
                break;
            }
            pos += 1;
        }

        if pos + search.len() >= data.len() {
            return Err(PdfError::InvalidStructure("Missing trailer".into()));
        }

        // Parse trailer dictionary
        let mut parser = Parser::new(data);
        parser.seek(pos + search.len());

        match parser.parse_object()? {
            Some(PdfObject::Dict(dict)) => Ok(dict),
            _ => Err(PdfError::InvalidStructure(
                "Trailer must be dictionary".into(),
            )),
        }
    }

    /// Get the trailer dictionary
    pub fn trailer(&self) -> &HashMap<String, PdfObject> {
        &self.trailer
    }

    /// Get number of objects in xref
    pub fn object_count(&self) -> usize {
        self.xref.len()
    }

    /// Resolve an object reference
    pub fn resolve(&mut self, obj_ref: ObjRef) -> Result<&PdfObject> {
        // Check cache first
        if self.cache.contains_key(&obj_ref) {
            return Ok(self.cache.get(&obj_ref).unwrap());
        }

        // Find in xref
        let entry = self.xref.get(&obj_ref.obj_num).ok_or_else(|| {
            PdfError::ObjectNotFound(obj_ref.obj_num, obj_ref.gen_num)
        })?;

        let entry_offset = entry.offset;

        // Parse object at offset
        let mut parser = Parser::new(self.data);
        parser.seek(entry_offset);

        // Expect: obj_num gen_num obj <content> endobj
        // Parse object number
        match parser.parse_object()? {
            Some(PdfObject::Int(n)) if n as u32 == obj_ref.obj_num => {}
            _ => {
                return Err(PdfError::Parse {
                    position: entry_offset,
                    message: "Expected object number".into(),
                });
            }
        };

        // Parse generation number
        match parser.parse_object()? {
            Some(PdfObject::Int(_)) => {}
            _ => {
                return Err(PdfError::Parse {
                    position: entry_offset,
                    message: "Expected generation number".into(),
                });
            }
        };

        // Parse "obj" keyword and the actual content
        // parse_object() handles Token::Obj by recursively parsing
        let parsed_obj = parser.parse_object()?.ok_or_else(|| PdfError::Parse {
            position: parser.position(),
            message: "Expected object content".into(),
        })?;

        // Cache and return
        self.cache.insert(obj_ref, parsed_obj);
        Ok(self.cache.get(&obj_ref).unwrap())
    }

    /// Get an object, resolving references automatically
    pub fn get_object(&mut self, obj: &PdfObject) -> Result<PdfObject> {
        match obj {
            PdfObject::Ref(r) => self.resolve(*r).cloned(),
            other => Ok(other.clone()),
        }
    }

    /// Get document catalog
    pub fn catalog(&mut self) -> Result<PdfObject> {
        let root_ref = self
            .trailer
            .get("Root")
            .ok_or_else(|| PdfError::InvalidStructure("Missing Root in trailer".into()))?
            .as_ref()
            .ok_or_else(|| PdfError::InvalidStructure("Root must be reference".into()))?;

        self.resolve(root_ref).cloned()
    }

    /// Get page count
    pub fn page_count(&mut self) -> Result<usize> {
        let catalog = self.catalog()?;
        let pages_ref = catalog
            .as_dict()
            .and_then(|d| d.get("Pages"))
            .and_then(|p| p.as_ref())
            .ok_or_else(|| PdfError::InvalidStructure("Missing Pages in catalog".into()))?;

        // Use recursive collection to count actual pages instead of relying on Count field
        let mut all_pages = Vec::new();
        self.collect_pages(pages_ref, &mut all_pages)?;
        Ok(all_pages.len())
    }

    /// Get decoded stream content from an object reference
    pub fn get_stream_data(&mut self, obj_ref: ObjRef) -> Result<Vec<u8>> {
        let obj = self.resolve(obj_ref)?.clone();

        match obj {
            PdfObject::Stream { dict, data } => decode_stream(&dict, &data),
            _ => Err(PdfError::InvalidStructure("Expected stream object".into())),
        }
    }

    /// Get a page by index (0-based)
    pub fn get_page(&mut self, index: usize) -> Result<PdfObject> {
        let catalog = self.catalog()?;
        let pages_ref = catalog
            .as_dict()
            .and_then(|d| d.get("Pages"))
            .and_then(|p| p.as_ref())
            .ok_or_else(|| PdfError::InvalidStructure("Missing Pages in catalog".into()))?;

        // Collect all pages recursively
        let mut all_pages = Vec::new();
        self.collect_pages(pages_ref, &mut all_pages)?;

        all_pages
            .get(index)
            .cloned()
            .ok_or_else(|| PdfError::InvalidStructure(format!("Page {} not found", index)))
    }

    /// Recursively collect all Page objects from a Pages tree
    fn collect_pages(&mut self, node_ref: ObjRef, pages: &mut Vec<PdfObject>) -> Result<()> {
        let node = self.resolve(node_ref)?.clone();
        let dict = node
            .as_dict()
            .ok_or_else(|| PdfError::InvalidStructure("Expected dict in page tree".into()))?;

        // Check the Type
        let type_name = dict
            .get("Type")
            .and_then(|t| t.as_name())
            .unwrap_or("");

        match type_name {
            "Page" => {
                // It's a leaf page
                pages.push(node.clone());
            }
            "Pages" => {
                // It's an intermediate node - recurse into Kids
                let kids = dict
                    .get("Kids")
                    .and_then(|k| k.as_array())
                    .ok_or_else(|| PdfError::InvalidStructure("Pages node missing Kids".into()))?;

                for kid in kids {
                    if let Some(kid_ref) = kid.as_ref() {
                        self.collect_pages(kid_ref, pages)?;
                    }
                }
            }
            _ => {
                // Unknown type - try to treat as page
                if dict.contains_key("Contents") || dict.contains_key("MediaBox") {
                    pages.push(node.clone());
                }
            }
        }

        Ok(())
    }

    /// Get content stream(s) from a page
    pub fn get_page_contents(&mut self, page: &PdfObject) -> Result<Vec<u8>> {
        let contents = page
            .as_dict()
            .and_then(|d| d.get("Contents"))
            .ok_or_else(|| PdfError::InvalidStructure("Page has no Contents".into()))?;

        match contents {
            PdfObject::Ref(r) => self.get_stream_data(*r),
            PdfObject::Array(arr) => {
                // Multiple content streams - concatenate
                let mut result = Vec::new();
                for item in arr {
                    if let Some(r) = item.as_ref() {
                        let data = self.get_stream_data(r)?;
                        result.extend(data);
                        result.push(b'\n'); // Separate streams
                    }
                }
                Ok(result)
            }
            _ => Err(PdfError::InvalidStructure("Invalid Contents type".into())),
        }
    }

    /// Extract text spans from a page (0-indexed)
    pub fn extract_page_text(&mut self, page_index: usize) -> Result<Vec<TextSpan>> {
        let page = self.get_page(page_index)?;
        let content = self.get_page_contents(&page)?;

        // Load font encodings from page resources
        let font_encodings = self.load_font_encodings(&page)?;

        let parser = ContentParser::with_fonts(&content, font_encodings);
        parser.parse()
    }

    /// Load font encodings from page resources
    fn load_font_encodings(&mut self, page: &PdfObject) -> Result<HashMap<String, FontEncoding>> {
        let mut encodings = HashMap::new();

        // Get Resources dictionary
        let resources = match page.as_dict().and_then(|d| d.get("Resources")) {
            Some(r) => self.get_object(r)?,
            None => return Ok(encodings),
        };

        // Get Font dictionary from Resources
        let fonts = match resources.as_dict().and_then(|d| d.get("Font")) {
            Some(f) => self.get_object(f)?,
            None => return Ok(encodings),
        };

        // Iterate over fonts
        if let Some(font_dict) = fonts.as_dict() {
            for (font_name, font_ref) in font_dict {
                if let Ok(encoding) = self.load_single_font_encoding(font_ref) {
                    encodings.insert(font_name.clone(), encoding);
                }
            }
        }

        Ok(encodings)
    }

    /// Load encoding for a single font
    fn load_single_font_encoding(&mut self, font_ref: &PdfObject) -> Result<FontEncoding> {
        let font = self.get_object(font_ref)?;
        let font_dict = font.as_dict().ok_or_else(|| {
            PdfError::InvalidStructure("Font is not a dictionary".into())
        })?;

        // Check for ToUnicode CMap first (most accurate)
        if let Some(tounicode_ref) = font_dict.get("ToUnicode") {
            if let Some(obj_ref) = tounicode_ref.as_ref() {
                if let Ok(cmap_data) = self.get_stream_data(obj_ref) {
                    if let Ok(cid_map) = parse_tounicode_cmap(&cmap_data) {
                        return Ok(FontEncoding::from_cid_map(cid_map));
                    }
                }
            }
        }

        // Check Encoding
        if let Some(encoding) = font_dict.get("Encoding") {
            match encoding {
                PdfObject::Name(name) => {
                    return Ok(match name.as_str() {
                        "WinAnsiEncoding" => FontEncoding::win_ansi(),
                        "MacRomanEncoding" => FontEncoding::mac_roman(),
                        _ => FontEncoding::win_ansi(), // Default to WinAnsi
                    });
                }
                PdfObject::Dict(enc_dict) => {
                    // Custom encoding with Differences array
                    // Start with base encoding
                    let encoding = if let Some(base) = enc_dict.get("BaseEncoding") {
                        match base.as_name() {
                            Some("WinAnsiEncoding") => FontEncoding::win_ansi(),
                            Some("MacRomanEncoding") => FontEncoding::mac_roman(),
                            _ => FontEncoding::win_ansi(),
                        }
                    } else {
                        FontEncoding::win_ansi()
                    };

                    // TODO: Apply Differences array
                    return Ok(encoding);
                }
                _ => {}
            }
        }

        // Default: WinAnsi encoding
        Ok(FontEncoding::win_ansi())
    }

    /// Extract all text from a page as a single string
    pub fn extract_page_text_string(&mut self, page_index: usize) -> Result<String> {
        let spans = self.extract_page_text(page_index)?;

        // Sort by y (descending) then x (ascending)
        let mut spans = spans;
        spans.sort_by(|a, b| {
            b.y.partial_cmp(&a.y)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then(a.x.partial_cmp(&b.x).unwrap_or(std::cmp::Ordering::Equal))
        });

        // Group into lines by y position
        let mut lines: Vec<Vec<&TextSpan>> = Vec::new();
        let mut current_line: Vec<&TextSpan> = Vec::new();
        let mut current_y: Option<f64> = None;
        let tolerance = 3.0;

        for span in &spans {
            match current_y {
                Some(y) if (span.y - y).abs() <= tolerance => {
                    current_line.push(span);
                }
                _ => {
                    if !current_line.is_empty() {
                        lines.push(current_line);
                    }
                    current_y = Some(span.y);
                    current_line = vec![span];
                }
            }
        }
        if !current_line.is_empty() {
            lines.push(current_line);
        }

        // Build text output
        let text: String = lines
            .iter()
            .map(|line| {
                line.iter()
                    .map(|span| span.text.as_str())
                    .collect::<Vec<_>>()
                    .join(" ")
            })
            .collect::<Vec<_>>()
            .join("\n");

        Ok(text)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_startxref() {
        let data = b"%PDF-1.4\n%%EOF\nstartxref\n1234\n%%EOF";
        let pos = Document::find_startxref(data).unwrap();
        assert!(data[pos..].starts_with(b"startxref"));
    }
}
