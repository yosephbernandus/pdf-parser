use std::collections::HashMap;
use crate::error::Result;

/// Parse a ToUnicode CMap and return a mapping from CID to Unicode char
pub fn parse_tounicode_cmap(data: &[u8]) -> Result<HashMap<u16, char>> {
    let text = String::from_utf8_lossy(data);
    let mut map = HashMap::new();

    // Find and parse beginbfchar sections
    parse_bfchar_sections(&text, &mut map);

    // Find and parse beginbfrange sections
    parse_bfrange_sections(&text, &mut map);

    Ok(map)
}

/// Parse beginbfchar...endbfchar sections
fn parse_bfchar_sections(text: &str, map: &mut HashMap<u16, char>) {
    let mut remaining = text;

    while let Some(start_idx) = remaining.find("beginbfchar") {
        remaining = &remaining[start_idx + 11..];

        if let Some(end_idx) = remaining.find("endbfchar") {
            let section = &remaining[..end_idx];
            parse_bfchar_entries(section, map);
            remaining = &remaining[end_idx + 9..];
        } else {
            break;
        }
    }
}

/// Parse individual bfchar entries: <srcCode><dstString>
fn parse_bfchar_entries(section: &str, map: &mut HashMap<u16, char>) {
    let mut chars = section.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '<' {
            // Parse source code
            let src = parse_hex_value(&mut chars);

            // Skip to next <
            while chars.peek() != Some(&'<') && chars.peek().is_some() {
                chars.next();
            }

            if chars.next() == Some('<') {
                // Parse destination code
                let dst = parse_hex_value(&mut chars);

                if let Some(ch) = char::from_u32(dst as u32) {
                    map.insert(src, ch);
                }
            }
        }
    }
}

/// Parse beginbfrange...endbfrange sections
fn parse_bfrange_sections(text: &str, map: &mut HashMap<u16, char>) {
    let mut remaining = text;

    while let Some(start_idx) = remaining.find("beginbfrange") {
        remaining = &remaining[start_idx + 12..];

        if let Some(end_idx) = remaining.find("endbfrange") {
            let section = &remaining[..end_idx];
            parse_bfrange_entries(section, map);
            remaining = &remaining[end_idx + 10..];
        } else {
            break;
        }
    }
}

/// Parse individual bfrange entries: <srcCodeLo><srcCodeHi><dstCodeLo>
fn parse_bfrange_entries(section: &str, map: &mut HashMap<u16, char>) {
    let mut chars = section.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '<' {
            // Parse source code low
            let src_lo = parse_hex_value(&mut chars);

            // Skip to next <
            while chars.peek() != Some(&'<') && chars.peek().is_some() {
                chars.next();
            }

            if chars.next() != Some('<') {
                continue;
            }

            // Parse source code high
            let src_hi = parse_hex_value(&mut chars);

            // Skip to next < or [
            while chars.peek() != Some(&'<') && chars.peek() != Some(&'[') && chars.peek().is_some() {
                chars.next();
            }

            match chars.next() {
                Some('<') => {
                    // Single destination - increment from this value
                    let dst_lo = parse_hex_value(&mut chars);

                    for i in 0..=(src_hi.saturating_sub(src_lo)) {
                        let src = src_lo + i;
                        let dst = dst_lo + i;
                        if let Some(ch) = char::from_u32(dst as u32) {
                            map.insert(src, ch);
                        }
                    }
                }
                Some('[') => {
                    // Array of destinations
                    let mut dst_values = Vec::new();

                    loop {
                        // Skip whitespace
                        while matches!(chars.peek(), Some(&' ') | Some(&'\n') | Some(&'\r') | Some(&'\t')) {
                            chars.next();
                        }

                        match chars.peek() {
                            Some(&'<') => {
                                chars.next();
                                dst_values.push(parse_hex_value(&mut chars));
                            }
                            Some(&']') => {
                                chars.next();
                                break;
                            }
                            _ => break,
                        }
                    }

                    for (i, &dst) in dst_values.iter().enumerate() {
                        let src = src_lo + i as u16;
                        if src <= src_hi {
                            if let Some(ch) = char::from_u32(dst as u32) {
                                map.insert(src, ch);
                            }
                        }
                    }
                }
                _ => continue,
            }
        }
    }
}

/// Parse a hex value from < until >
fn parse_hex_value(chars: &mut std::iter::Peekable<std::str::Chars>) -> u16 {
    let mut hex_str = String::new();

    while let Some(&c) = chars.peek() {
        if c == '>' {
            chars.next();
            break;
        }
        if c.is_ascii_hexdigit() {
            hex_str.push(c);
        }
        chars.next();
    }

    u16::from_str_radix(&hex_str, 16).unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_bfrange() {
        let cmap = r#"
beginbfrange
<0003><0003><0020>
<0024><0024><0041>
endbfrange
"#;
        let map = parse_tounicode_cmap(cmap.as_bytes()).unwrap();
        assert_eq!(map.get(&0x0003), Some(&' '));
        assert_eq!(map.get(&0x0024), Some(&'A'));
    }

    #[test]
    fn test_parse_bfrange_sequence() {
        let cmap = r#"
beginbfrange
<0024><0026><0041>
endbfrange
"#;
        let map = parse_tounicode_cmap(cmap.as_bytes()).unwrap();
        assert_eq!(map.get(&0x0024), Some(&'A'));
        assert_eq!(map.get(&0x0025), Some(&'B'));
        assert_eq!(map.get(&0x0026), Some(&'C'));
    }

    #[test]
    fn test_parse_bfchar() {
        let cmap = r#"
beginbfchar
<0003><0020>
<0024><0041>
endbfchar
"#;
        let map = parse_tounicode_cmap(cmap.as_bytes()).unwrap();
        assert_eq!(map.get(&0x0003), Some(&' '));
        assert_eq!(map.get(&0x0024), Some(&'A'));
    }
}
