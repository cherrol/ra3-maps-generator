//! Small, range-preserving XML reader for map asset structural checks.
//! This validates the document envelope; it is not a general XSD processor.
use crate::{Result, err};
use std::{collections::BTreeMap, ops::Range};

#[derive(Debug)]
pub struct Element<'a> {
    pub name: &'a str,
    pub namespace: String,
    pub attributes: BTreeMap<&'a str, String>,
    pub children: Vec<usize>,
    pub range: Range<usize>,
    pub content: Range<usize>,
}
impl Element<'_> {
    pub fn local_name(&self) -> &str {
        self.name.rsplit(':').next().unwrap()
    }
    pub fn attribute(&self, name: &str) -> Option<&str> {
        self.attributes.get(name).map(String::as_str)
    }
}

#[derive(Debug)]
pub struct Document<'a> {
    pub elements: Vec<Element<'a>>,
    pub root: usize,
}

fn name_end(bytes: &[u8], mut pos: usize) -> usize {
    while bytes.get(pos).is_some_and(|c| {
        c.is_ascii_alphanumeric() || matches!(c, b'_' | b':' | b'-' | b'.') || *c >= 128
    }) {
        pos += 1;
    }
    pos
}
fn qualified_name(name: &str) -> bool {
    let parts = name.split(':').collect::<Vec<_>>();
    parts.len() <= 2
        && parts.iter().all(|part| {
            part.chars()
                .next()
                .is_some_and(|c| c == '_' || c.is_alphabetic())
        })
}
fn space(bytes: &[u8], pos: &mut usize) {
    while bytes.get(*pos).is_some_and(u8::is_ascii_whitespace) {
        *pos += 1;
    }
}
fn decoded(text: &str) -> Result<String> {
    let mut out = String::new();
    let mut rest = text;
    while let Some(start) = rest.find('&') {
        out.push_str(&rest[..start]);
        rest = &rest[start + 1..];
        let end = rest
            .find(';')
            .ok_or_else(|| err("unterminated XML entity"))?;
        let entity = &rest[..end];
        let character = match entity {
            "amp" => '&',
            "lt" => '<',
            "gt" => '>',
            "quot" => '"',
            "apos" => '\'',
            _ => {
                let number = if let Some(hex) = entity.strip_prefix("#x") {
                    u32::from_str_radix(hex, 16)?
                } else if let Some(decimal) = entity.strip_prefix('#') {
                    decimal.parse()?
                } else {
                    return Err(err(format!("unsupported XML entity &{entity};")));
                };
                char::from_u32(number).ok_or_else(|| err("invalid XML character reference"))?
            }
        };
        if !valid_character(character) {
            return Err(err("invalid XML character"));
        }
        out.push(character);
        rest = &rest[end + 1..];
    }
    out.push_str(rest);
    Ok(out)
}
fn valid_character(c: char) -> bool {
    matches!(c, '\t' | '\n' | '\r') || (c >= ' ' && c != '\u{fffe}' && c != '\u{ffff}')
}

impl<'a> Document<'a> {
    pub fn parse(xml: &'a str) -> Result<Self> {
        if xml.chars().any(|c| !valid_character(c)) {
            return Err(err("invalid XML character"));
        }
        let bytes = xml.as_bytes();
        let document_start = if xml.starts_with('\u{feff}') { 3 } else { 0 };
        let mut pos = document_start;
        let mut elements: Vec<Element<'a>> = Vec::new();
        let mut stack: Vec<(usize, BTreeMap<String, String>)> = Vec::new();
        let mut root = None;
        while pos < bytes.len() {
            if bytes[pos] != b'<' {
                let end = xml[pos..].find('<').map_or(xml.len(), |n| pos + n);
                let text = &xml[pos..end];
                if stack.is_empty() && !text.trim().is_empty() {
                    return Err(err("text outside XML root"));
                }
                if text.contains("]]>") {
                    return Err(err("CDATA terminator in XML text"));
                }
                if text.contains('&') {
                    decoded(text)?;
                }
                pos = end;
                continue;
            }
            let start = pos;
            if xml[pos..].starts_with("<!--") {
                let end = xml[pos + 4..]
                    .find("-->")
                    .ok_or_else(|| err("unterminated XML comment"))?
                    + pos
                    + 4;
                if xml[pos + 4..end].contains("--") || xml[pos + 4..end].ends_with('-') {
                    return Err(err("invalid XML comment"));
                }
                pos = end + 3;
                continue;
            }
            if xml[pos..].starts_with("<![CDATA[") {
                if stack.is_empty() {
                    return Err(err("CDATA outside XML root"));
                }
                pos = xml[pos + 9..]
                    .find("]]>")
                    .ok_or_else(|| err("unterminated XML CDATA"))?
                    + pos
                    + 12;
                continue;
            }
            if xml[pos..].starts_with("<?") {
                let target_start = pos + 2;
                let target_end = name_end(bytes, target_start);
                let target = &xml[target_start..target_end];
                if !qualified_name(target)
                    || !matches!(bytes.get(target_end), Some(b'?'))
                        && !bytes.get(target_end).is_some_and(u8::is_ascii_whitespace)
                {
                    return Err(err("invalid XML processing instruction target"));
                }
                if target.eq_ignore_ascii_case("xml") && (target != "xml" || pos != document_start)
                {
                    return Err(err("XML declaration must be at the document start"));
                }
                pos = xml[target_end..]
                    .find("?>")
                    .ok_or_else(|| err("unterminated XML processing instruction"))?
                    + target_end
                    + 2;
                continue;
            }
            if xml[pos..].starts_with("<!") {
                return Err(err("XML document type declarations are unsupported"));
            }
            let closing = bytes.get(pos + 1) == Some(&b'/');
            pos += if closing { 2 } else { 1 };
            let end = name_end(bytes, pos);
            if end == pos || bytes[pos].is_ascii_digit() || matches!(bytes[pos], b'-' | b'.') {
                return Err(err("invalid XML element name"));
            }
            let name = &xml[pos..end];
            if !qualified_name(name) {
                return Err(err("invalid XML qualified name"));
            }
            pos = end;
            if closing {
                space(bytes, &mut pos);
                if bytes.get(pos) != Some(&b'>') {
                    return Err(err("invalid XML closing tag"));
                }
                let (index, _) = stack
                    .pop()
                    .ok_or_else(|| err("unmatched XML closing tag"))?;
                if elements[index].name != name {
                    return Err(err(format!("mismatched XML closing tag {name}")));
                }
                elements[index].content.end = start;
                elements[index].range.end = pos + 1;
                pos += 1;
                continue;
            }
            let mut attributes = BTreeMap::new();
            loop {
                let before_space = pos;
                space(bytes, &mut pos);
                if matches!(bytes.get(pos), Some(b'/' | b'>')) {
                    break;
                }
                if before_space == pos {
                    return Err(err("XML attributes require whitespace"));
                }
                let end = name_end(bytes, pos);
                if end == pos {
                    return Err(err("invalid XML attribute name"));
                }
                let key = &xml[pos..end];
                if !qualified_name(key) {
                    return Err(err("invalid XML attribute name"));
                }
                pos = end;
                space(bytes, &mut pos);
                if bytes.get(pos) != Some(&b'=') {
                    return Err(err("XML attribute requires a value"));
                }
                pos += 1;
                space(bytes, &mut pos);
                let quote = *bytes.get(pos).ok_or_else(|| err("missing XML attribute"))?;
                if quote != b'\'' && quote != b'"' {
                    return Err(err("unquoted XML attribute"));
                }
                pos += 1;
                let value_start = pos;
                while bytes.get(pos).is_some_and(|c| *c != quote) {
                    if bytes[pos] == b'<' {
                        return Err(err("unescaped XML attribute"));
                    }
                    pos += 1;
                }
                if bytes.get(pos) != Some(&quote) {
                    return Err(err("unterminated XML attribute"));
                }
                if attributes
                    .insert(key, decoded(&xml[value_start..pos])?)
                    .is_some()
                {
                    return Err(err(format!("duplicate XML attribute {key}")));
                }
                pos += 1;
            }
            let empty = bytes.get(pos) == Some(&b'/');
            if empty {
                pos += 1;
            }
            if bytes.get(pos) != Some(&b'>') {
                return Err(err("unterminated XML opening tag"));
            }
            pos += 1;
            let mut namespaces = stack.last().map(|(_, ns)| ns.clone()).unwrap_or_default();
            namespaces.insert("xml".into(), "http://www.w3.org/XML/1998/namespace".into());
            for (key, value) in &attributes {
                if *key == "xmlns" {
                    namespaces.insert(String::new(), value.clone());
                } else if let Some(prefix) = key.strip_prefix("xmlns:") {
                    if prefix == "xmlns"
                        || value.is_empty()
                        || (prefix == "xml" && value != "http://www.w3.org/XML/1998/namespace")
                        || (prefix != "xml" && value == "http://www.w3.org/XML/1998/namespace")
                    {
                        return Err(err("invalid XML namespace declaration"));
                    }
                    namespaces.insert(prefix.into(), value.clone());
                }
            }
            let mut expanded_attributes = std::collections::BTreeSet::new();
            for key in attributes.keys() {
                if *key == "xmlns" || key.starts_with("xmlns:") {
                    continue;
                }
                let (namespace, local) = if let Some((prefix, local)) = key.split_once(':') {
                    (
                        namespaces
                            .get(prefix)
                            .ok_or_else(|| err("unbound XML attribute namespace"))?
                            .as_str(),
                        local,
                    )
                } else {
                    ("", *key)
                };
                if !expanded_attributes.insert((namespace, local)) {
                    return Err(err("duplicate expanded XML attribute"));
                }
            }
            let namespace = match name.split_once(':') {
                Some((prefix, local)) if !local.contains(':') => namespaces
                    .get(prefix)
                    .cloned()
                    .ok_or_else(|| err("unbound XML namespace prefix"))?,
                Some(_) => return Err(err("invalid XML qualified name")),
                None => namespaces.get("").cloned().unwrap_or_default(),
            };
            let index = elements.len();
            if let Some((parent, _)) = stack.last() {
                elements[*parent].children.push(index);
            } else if root.replace(index).is_some() {
                return Err(err("multiple XML roots"));
            }
            elements.push(Element {
                name,
                namespace,
                attributes,
                children: Vec::new(),
                range: start..pos,
                content: pos..pos,
            });
            if !empty {
                if stack.len() >= 256 {
                    return Err(err("XML nesting limit exceeded"));
                }
                stack.push((index, namespaces));
            }
        }
        if !stack.is_empty() {
            return Err(err("unterminated XML element"));
        }
        Ok(Self {
            elements,
            root: root.ok_or_else(|| err("missing XML root"))?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn reads_namespaces_entities_cdata_and_exact_source_ranges() {
        let xml = "\u{feff}<?xml version='1.0'?><a:root xmlns:a='urn:test'><!--x--><a:item id='a&amp;b' n='&#x31;'/><![CDATA[<raw>&text]]></a:root>";
        let doc = Document::parse(xml).unwrap();
        let item = &doc.elements[doc.elements[doc.root].children[0]];
        assert_eq!(item.namespace, "urn:test");
        assert_eq!(item.local_name(), "item");
        assert_eq!(item.attribute("id"), Some("a&b"));
        assert_eq!(item.attribute("n"), Some("1"));
        assert_eq!(
            &xml[item.range.clone()],
            "<a:item id='a&amp;b' n='&#x31;'/>"
        );
    }
    #[test]
    fn rejects_malformed_documents_instead_of_counting_tag_text() {
        for xml in [
            "<a><b></a>",
            "<a/><b/>",
            "<a x='1' x='2'/>",
            "<a>&unknown;</a>",
            "<a x='bad<value'/>",
            "<a:b/>",
            "<a>\0</a>",
            "<!DOCTYPE a><a/>",
            "<a><!-- unclosed</a>",
            "<a x='1'y='2'/>",
            "<a bad:flag='1'/>",
            "<a 1bad='x'/>",
            "<a xmlns:x='same' xmlns:y='same' x:flag='1' y:flag='2'/>",
            "<a xmlns:xml='wrong'/>",
            "<a><?xml version='1.0'?></a>",
            "<a/><?xml version='1.0'?>",
            " <?xml version='1.0'?><a/>",
            "<?XML version='1.0'?><a/>",
            "<??><a/>",
            "<a><!-- invalid---></a>",
        ] {
            assert!(Document::parse(xml).is_err(), "{xml}");
        }
    }
}
