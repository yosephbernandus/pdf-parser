use std::collections::HashMap;

/// Font encoding for translating character codes to Unicode
#[derive(Debug, Clone)]
pub struct FontEncoding {
    /// Map from byte code to Unicode character
    map: HashMap<u8, char>,
    /// Map from two-byte CID to Unicode (for Type0/CID fonts)
    cid_map: HashMap<u16, char>,
}

impl Default for FontEncoding {
    fn default() -> Self {
        Self::identity()
    }
}

impl FontEncoding {
    /// Identity encoding - bytes map to same Unicode code point
    pub fn identity() -> Self {
        let mut map = HashMap::new();
        for i in 0u8..=255 {
            map.insert(i, i as char);
        }
        FontEncoding {
            map,
            cid_map: HashMap::new(),
        }
    }

    /// WinAnsiEncoding - standard Windows encoding
    pub fn win_ansi() -> Self {
        let mut map = HashMap::new();

        // Standard ASCII range
        for i in 0x20u8..=0x7E {
            map.insert(i, i as char);
        }

        // Windows-1252 specific mappings for 0x80-0x9F range
        let high_mappings: [(u8, char); 27] = [
            (0x80, '\u{20AC}'), // Euro sign
            (0x82, '\u{201A}'), // Single Low-9 Quotation Mark
            (0x83, '\u{0192}'), // Latin Small Letter F With Hook
            (0x84, '\u{201E}'), // Double Low-9 Quotation Mark
            (0x85, '\u{2026}'), // Horizontal Ellipsis
            (0x86, '\u{2020}'), // Dagger
            (0x87, '\u{2021}'), // Double Dagger
            (0x88, '\u{02C6}'), // Modifier Letter Circumflex Accent
            (0x89, '\u{2030}'), // Per Mille Sign
            (0x8A, '\u{0160}'), // Latin Capital Letter S With Caron
            (0x8B, '\u{2039}'), // Single Left-Pointing Angle Quotation Mark
            (0x8C, '\u{0152}'), // Latin Capital Ligature OE
            (0x8E, '\u{017D}'), // Latin Capital Letter Z With Caron
            (0x91, '\u{2018}'), // Left Single Quotation Mark
            (0x92, '\u{2019}'), // Right Single Quotation Mark
            (0x93, '\u{201C}'), // Left Double Quotation Mark
            (0x94, '\u{201D}'), // Right Double Quotation Mark
            (0x95, '\u{2022}'), // Bullet
            (0x96, '\u{2013}'), // En Dash
            (0x97, '\u{2014}'), // Em Dash
            (0x98, '\u{02DC}'), // Small Tilde
            (0x99, '\u{2122}'), // Trade Mark Sign
            (0x9A, '\u{0161}'), // Latin Small Letter S With Caron
            (0x9B, '\u{203A}'), // Single Right-Pointing Angle Quotation Mark
            (0x9C, '\u{0153}'), // Latin Small Ligature OE
            (0x9E, '\u{017E}'), // Latin Small Letter Z With Caron
            (0x9F, '\u{0178}'), // Latin Capital Letter Y With Diaeresis
        ];

        for (code, ch) in high_mappings {
            map.insert(code, ch);
        }

        // Latin-1 Supplement (0xA0-0xFF)
        for i in 0xA0u8..=0xFF {
            map.insert(i, char::from_u32(i as u32).unwrap_or('?'));
        }

        FontEncoding {
            map,
            cid_map: HashMap::new(),
        }
    }

    /// MacRomanEncoding
    pub fn mac_roman() -> Self {
        let mut map = HashMap::new();

        // Standard ASCII range
        for i in 0x20u8..=0x7E {
            map.insert(i, i as char);
        }

        // Mac Roman specific mappings
        let mac_mappings: [(u8, char); 128] = [
            (0x80, 'Ä'), (0x81, 'Å'), (0x82, 'Ç'), (0x83, 'É'),
            (0x84, 'Ñ'), (0x85, 'Ö'), (0x86, 'Ü'), (0x87, 'á'),
            (0x88, 'à'), (0x89, 'â'), (0x8A, 'ä'), (0x8B, 'ã'),
            (0x8C, 'å'), (0x8D, 'ç'), (0x8E, 'é'), (0x8F, 'è'),
            (0x90, 'ê'), (0x91, 'ë'), (0x92, 'í'), (0x93, 'ì'),
            (0x94, 'î'), (0x95, 'ï'), (0x96, 'ñ'), (0x97, 'ó'),
            (0x98, 'ò'), (0x99, 'ô'), (0x9A, 'ö'), (0x9B, 'õ'),
            (0x9C, 'ú'), (0x9D, 'ù'), (0x9E, 'û'), (0x9F, 'ü'),
            (0xA0, '†'), (0xA1, '°'), (0xA2, '¢'), (0xA3, '£'),
            (0xA4, '§'), (0xA5, '•'), (0xA6, '¶'), (0xA7, 'ß'),
            (0xA8, '®'), (0xA9, '©'), (0xAA, '™'), (0xAB, '´'),
            (0xAC, '¨'), (0xAD, '≠'), (0xAE, 'Æ'), (0xAF, 'Ø'),
            (0xB0, '∞'), (0xB1, '±'), (0xB2, '≤'), (0xB3, '≥'),
            (0xB4, '¥'), (0xB5, 'µ'), (0xB6, '∂'), (0xB7, '∑'),
            (0xB8, '∏'), (0xB9, 'π'), (0xBA, '∫'), (0xBB, 'ª'),
            (0xBC, 'º'), (0xBD, 'Ω'), (0xBE, 'æ'), (0xBF, 'ø'),
            (0xC0, '¿'), (0xC1, '¡'), (0xC2, '¬'), (0xC3, '√'),
            (0xC4, 'ƒ'), (0xC5, '≈'), (0xC6, '∆'), (0xC7, '«'),
            (0xC8, '»'), (0xC9, '…'), (0xCA, ' '), (0xCB, 'À'),
            (0xCC, 'Ã'), (0xCD, 'Õ'), (0xCE, 'Œ'), (0xCF, 'œ'),
            (0xD0, '–'), (0xD1, '—'), (0xD2, '"'), (0xD3, '"'),
            (0xD4, '\u{2018}'), (0xD5, '\u{2019}'), (0xD6, '÷'), (0xD7, '◊'),
            (0xD8, 'ÿ'), (0xD9, 'Ÿ'), (0xDA, '⁄'), (0xDB, '€'),
            (0xDC, '‹'), (0xDD, '›'), (0xDE, 'ﬁ'), (0xDF, 'ﬂ'),
            (0xE0, '‡'), (0xE1, '·'), (0xE2, '‚'), (0xE3, '„'),
            (0xE4, '‰'), (0xE5, 'Â'), (0xE6, 'Ê'), (0xE7, 'Á'),
            (0xE8, 'Ë'), (0xE9, 'È'), (0xEA, 'Í'), (0xEB, 'Î'),
            (0xEC, 'Ï'), (0xED, 'Ì'), (0xEE, 'Ó'), (0xEF, 'Ô'),
            (0xF0, '\u{F8FF}'), (0xF1, 'Ò'), (0xF2, 'Ú'), (0xF3, 'Û'),
            (0xF4, 'Ù'), (0xF5, 'ı'), (0xF6, 'ˆ'), (0xF7, '˜'),
            (0xF8, '¯'), (0xF9, '˘'), (0xFA, '˙'), (0xFB, '˚'),
            (0xFC, '¸'), (0xFD, '˝'), (0xFE, '˛'), (0xFF, 'ˇ'),
        ];

        for (code, ch) in mac_mappings {
            map.insert(code, ch);
        }

        FontEncoding {
            map,
            cid_map: HashMap::new(),
        }
    }

    /// Create encoding from a CID to Unicode map (for Type0 fonts with ToUnicode)
    pub fn from_cid_map(cid_map: HashMap<u16, char>) -> Self {
        FontEncoding {
            map: HashMap::new(),
            cid_map,
        }
    }

    /// Decode a single byte
    pub fn decode_byte(&self, byte: u8) -> char {
        self.map.get(&byte).copied().unwrap_or(byte as char)
    }

    /// Decode a CID (two bytes)
    pub fn decode_cid(&self, cid: u16) -> Option<char> {
        self.cid_map.get(&cid).copied()
    }

    /// Check if this encoding has CID mappings
    pub fn has_cid_map(&self) -> bool {
        !self.cid_map.is_empty()
    }

    /// Decode a byte string using this encoding
    pub fn decode_bytes(&self, bytes: &[u8]) -> String {
        if self.has_cid_map() {
            // CID font - decode as 2-byte sequences
            let mut result = String::new();
            let mut i = 0;
            while i < bytes.len() {
                if i + 1 < bytes.len() {
                    let cid = ((bytes[i] as u16) << 8) | (bytes[i + 1] as u16);
                    if let Some(ch) = self.decode_cid(cid) {
                        result.push(ch);
                    } else {
                        // Fallback: treat as two separate bytes
                        result.push(self.decode_byte(bytes[i]));
                        result.push(self.decode_byte(bytes[i + 1]));
                    }
                    i += 2;
                } else {
                    // Odd byte at end
                    result.push(self.decode_byte(bytes[i]));
                    i += 1;
                }
            }
            result
        } else {
            // Simple encoding - one byte per character
            bytes.iter().map(|&b| self.decode_byte(b)).collect()
        }
    }

    /// Add a CID mapping
    pub fn add_cid_mapping(&mut self, cid: u16, unicode: char) {
        self.cid_map.insert(cid, unicode);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_win_ansi_basic() {
        let enc = FontEncoding::win_ansi();
        assert_eq!(enc.decode_byte(b'A'), 'A');
        assert_eq!(enc.decode_byte(b'Z'), 'Z');
        assert_eq!(enc.decode_byte(b' '), ' ');
    }

    #[test]
    fn test_win_ansi_special() {
        let enc = FontEncoding::win_ansi();
        assert_eq!(enc.decode_byte(0x80), '\u{20AC}'); // Euro
        assert_eq!(enc.decode_byte(0x99), '\u{2122}'); // TM
    }

    #[test]
    fn test_cid_decode() {
        let mut enc = FontEncoding::from_cid_map(HashMap::new());
        enc.add_cid_mapping(0x0024, 'A');
        enc.add_cid_mapping(0x0003, ' ');

        assert_eq!(enc.decode_cid(0x0024), Some('A'));
        assert_eq!(enc.decode_cid(0x0003), Some(' '));
        assert_eq!(enc.decode_cid(0x9999), None);
    }

    #[test]
    fn test_decode_bytes_cid() {
        let mut enc = FontEncoding::from_cid_map(HashMap::new());
        enc.add_cid_mapping(0x0024, 'A');
        enc.add_cid_mapping(0x0025, 'B');

        let bytes = [0x00, 0x24, 0x00, 0x25];
        assert_eq!(enc.decode_bytes(&bytes), "AB");
    }
}
