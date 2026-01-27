use std::collections::HashMap;

use crate::decode::decode_stream;
use crate::error::{PdfError, Result};
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

    /// Parse xref table and trailer dictionary
    fn parse_xref_and_trailer(
        data: &[u8],
        offset: usize,
    ) -> Result<(HashMap<u32, XRefEntry>, HashMap<String, PdfObject>)> {
        let mut xref = HashMap::new();

        // Check if this is a traditional xref table or xref stream
        if data[offset..].starts_with(b"xref") {
            // Traditional xref table
            Self::parse_traditional_xref(data, offset, &mut xref)?;

            // Find and parse trailer
            let trailer = Self::find_and_parse_trailer(data, offset)?;

            Ok((xref, trailer))
        } else {
            // Might be an xref stream (PDF 1.5+)
            // TODO: Implement xref stream parsing
            Err(PdfError::InvalidStructure(
                "XRef streams not yet supported".into(),
            ))
        }
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
        // Catalog -> Pages -> Count
        let catalog = self.catalog()?;
        let pages_ref = catalog
            .as_dict()
            .and_then(|d| d.get("Pages"))
            .and_then(|p| p.as_ref())
            .ok_or_else(|| PdfError::InvalidStructure("Missing Pages in catalog".into()))?;

        let pages = self.resolve(pages_ref)?;
        let count = pages
            .as_dict()
            .and_then(|d| d.get("Count"))
            .and_then(|c| c.as_int())
            .ok_or_else(|| PdfError::InvalidStructure("Missing Count in Pages".into()))?;

        Ok(count as usize)
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

        let pages = self.resolve(pages_ref)?.clone();
        let kids = pages
            .as_dict()
            .and_then(|d| d.get("Kids"))
            .and_then(|k| k.as_array())
            .ok_or_else(|| PdfError::InvalidStructure("Missing Kids in Pages".into()))?;

        let page_ref = kids
            .get(index)
            .and_then(|p| p.as_ref())
            .ok_or_else(|| PdfError::InvalidStructure(format!("Page {} not found", index)))?;

        self.resolve(page_ref).cloned()
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
