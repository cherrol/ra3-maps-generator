//! Offline, reproducible extraction of the pinned EA SDK resource metadata.
//! This is an authoring tool, never a production runtime dependency. It follows
//! actual includes from StaticGameObjects and the verified object-source registry;
//! it deliberately does not pick a definition by scanning all SDK object IDs.
use ra3maps::formats::sha256;
use serde_json::{Map, Value};
use std::{
    collections::{BTreeMap, BTreeSet},
    env, fs,
    path::{Component, Path, PathBuf},
};

const VERSION: &str = "ea-catalog-v2";
const REVISION: &str = "3aad8e412961ab543192ae70f8503dc8e178e095";
const ASSET_NS: &str = "uri:ea.com:eala:asset";
const XI_NS: &str = "http://www.w3.org/2001/XInclude";
const JOIN: &str = "{uri:ea.com:eala:asset:instance}joinAction";
const XML_NS: &str = "http://www.w3.org/XML/1998/namespace";
/// The binding pins the extractor itself, so these sources must stay self-contained.
const EXTRACTOR: &str = "xtask/import_ea_catalog.rs";
const SUPPORT: &str = "src/formats/sha256.rs";

type Fallible<T> = Result<T, String>;

/// `?` for I/O and JSON errors inside the string-based authoring tool.
trait Stringify<T> {
    fn stringify(self) -> Fallible<T>;
}
impl<T, E: std::fmt::Display> Stringify<T> for Result<T, E> {
    fn stringify(self) -> Fallible<T> {
        self.map_err(|error| error.to_string())
    }
}

// ---------------------------------------------------------------------------
// Owned XML tree: EA includes are fragments and need merging, not just checks.
// ---------------------------------------------------------------------------
#[derive(Clone, Debug, Default)]
struct Node {
    tag: String,
    attributes: BTreeMap<String, String>,
    children: Vec<Node>,
    text: String,
}
impl Node {
    fn local(&self) -> &str {
        self.tag.rsplit('}').next().unwrap_or(&self.tag)
    }
    fn attr(&self, name: &str) -> Option<&str> {
        self.attributes.get(name).map(String::as_str)
    }
    fn child(&self, name: &str) -> Option<&Node> {
        self.children.iter().find(|child| child.local() == name)
    }
    fn children_named(&self, name: &str) -> Vec<&Node> {
        self.children
            .iter()
            .filter(|child| child.local() == name)
            .collect()
    }
    fn walk<'a>(&'a self, out: &mut Vec<&'a Node>) {
        out.push(self);
        for child in &self.children {
            child.walk(out);
        }
    }
}

struct XmlParser<'a> {
    text: &'a str,
    pos: usize,
}
impl<'a> XmlParser<'a> {
    fn parse_root(text: &'a str) -> Fallible<Node> {
        let mut parser = Self {
            text,
            pos: text.strip_prefix('\u{feff}').map_or(0, str::len),
        };
        let mut root = None;
        loop {
            parser.skip_misc()?;
            if parser.pos >= parser.text.len() {
                break;
            }
            if root.is_some() {
                return Err("junk after XML root".into());
            }
            if parser.text.as_bytes()[parser.pos] != b'<' {
                return Err("text outside XML root".into());
            }
            let mut scopes = vec![BTreeMap::from([(
                String::from("xml"),
                String::from(XML_NS),
            )])];
            root = Some(parser.element(&mut scopes)?);
        }
        root.ok_or_else(|| "missing XML root".to_string())
    }
    fn byte(&self) -> Option<u8> {
        self.text.as_bytes().get(self.pos).copied()
    }
    fn skip_space(&mut self) {
        while self.byte().is_some_and(|byte| byte.is_ascii_whitespace()) {
            self.pos += 1;
        }
    }
    fn skip_until(&mut self, pattern: &str) -> Fallible<()> {
        let end = self.text[self.pos..]
            .find(pattern)
            .ok_or_else(|| format!("unterminated XML construct {pattern}"))?;
        self.pos += end + pattern.len();
        Ok(())
    }
    fn skip_misc(&mut self) -> Fallible<()> {
        loop {
            self.skip_space();
            let rest = &self.text[self.pos..];
            if rest.starts_with("<!--") {
                self.pos += 4;
                self.skip_until("-->")?;
            } else if rest.starts_with("<?") {
                self.pos += 2;
                self.skip_until("?>")?;
            } else if rest.starts_with("<!") {
                self.skip_declaration()?;
            } else {
                return Ok(());
            }
        }
    }
    fn skip_declaration(&mut self) -> Fallible<()> {
        let mut quote = None;
        let mut depth = 0usize;
        while let Some(byte) = self.byte() {
            self.pos += 1;
            match (quote, byte) {
                (Some(active), byte) if byte == active => quote = None,
                (Some(_), _) => {}
                (None, b'"' | b'\'') => quote = Some(byte),
                (None, b'[') => depth += 1,
                (None, b']') => depth = depth.saturating_sub(1),
                (None, b'>') if depth == 0 => return Ok(()),
                (None, _) => {}
            }
        }
        Err("unterminated XML declaration".into())
    }
    fn element(&mut self, scopes: &mut Vec<BTreeMap<String, String>>) -> Fallible<Node> {
        self.pos += 1; // '<'
        let start = self.pos;
        while self
            .byte()
            .is_some_and(|byte| !matches!(byte, b' ' | b'\t' | b'\r' | b'\n' | b'/' | b'>' | b'='))
        {
            self.pos += 1;
        }
        if start == self.pos {
            return Err("invalid XML element name".into());
        }
        let name = self.text[start..self.pos].to_string();
        let mut scope = scopes.last().cloned().unwrap_or_default();
        let mut raw = Vec::new();
        loop {
            let before = self.pos;
            self.skip_space();
            if matches!(self.byte(), Some(b'/' | b'>')) {
                break;
            }
            if before == self.pos {
                return Err("XML attributes require whitespace".into());
            }
            let attr_start = self.pos;
            while self.byte().is_some_and(|byte| {
                !matches!(byte, b' ' | b'\t' | b'\r' | b'\n' | b'=' | b'/' | b'>')
            }) {
                self.pos += 1;
            }
            if attr_start == self.pos {
                return Err("invalid XML attribute name".into());
            }
            let attr_name = self.text[attr_start..self.pos].to_string();
            self.skip_space();
            if self.byte() != Some(b'=') {
                return Err("XML attribute requires a value".into());
            }
            self.pos += 1;
            self.skip_space();
            let quote = self.byte().ok_or("unterminated XML attribute")?;
            if !matches!(quote, b'"' | b'\'') {
                return Err("unquoted XML attribute".into());
            }
            self.pos += 1;
            let value_start = self.pos;
            loop {
                match self.byte() {
                    None => return Err("unterminated XML attribute".into()),
                    Some(b'<') => return Err("unescaped XML attribute".into()),
                    Some(byte) if byte == quote => break,
                    Some(_) => self.pos += 1,
                }
            }
            let value = decode_entities(&self.text[value_start..self.pos], true)?;
            self.pos += 1;
            raw.push((attr_name, value));
        }
        for (key, value) in &raw {
            if key == "xmlns" {
                scope.insert(String::new(), value.clone());
            } else if let Some(prefix) = key.strip_prefix("xmlns:") {
                scope.insert(prefix.to_string(), value.clone());
            }
        }
        let tag = expand_name(&name, &scope, true)?;
        let mut attributes = BTreeMap::new();
        for (key, value) in raw {
            if key == "xmlns" || key.starts_with("xmlns:") {
                continue;
            }
            let expanded = expand_name(&key, &scope, false)?;
            if attributes.insert(expanded.clone(), value).is_some() {
                return Err(format!("duplicate XML attribute {expanded}"));
            }
        }
        let empty = self.byte() == Some(b'/');
        if empty {
            self.pos += 1;
        }
        if self.byte() != Some(b'>') {
            return Err("unterminated XML opening tag".into());
        }
        self.pos += 1;
        let mut node = Node {
            tag,
            attributes,
            children: Vec::new(),
            text: String::new(),
        };
        if empty {
            return Ok(node);
        }
        scopes.push(scope);
        loop {
            match self.byte() {
                None => {
                    scopes.pop();
                    return Err("unterminated XML element".into());
                }
                Some(b'<') => {
                    let rest = &self.text[self.pos..];
                    if rest.starts_with("</") {
                        let close_start = self.pos + 2;
                        let mut end = close_start;
                        while self
                            .text
                            .as_bytes()
                            .get(end)
                            .is_some_and(|byte| !byte.is_ascii_whitespace() && *byte != b'>')
                        {
                            end += 1;
                        }
                        if &self.text[close_start..end] != name.as_str() {
                            scopes.pop();
                            return Err(format!(
                                "mismatched XML closing tag {}",
                                &self.text[close_start..end]
                            ));
                        }
                        self.pos = end;
                        self.skip_space();
                        if self.byte() != Some(b'>') {
                            scopes.pop();
                            return Err("invalid XML closing tag".into());
                        }
                        self.pos += 1;
                        scopes.pop();
                        return Ok(node);
                    }
                    if rest.starts_with("<!--") {
                        self.pos += 4;
                        self.skip_until("-->")?;
                        continue;
                    }
                    if rest.starts_with("<![CDATA[") {
                        self.pos += 9;
                        let end = self.text[self.pos..]
                            .find("]]>")
                            .ok_or("unterminated XML CDATA")?;
                        let data = &self.text[self.pos..self.pos + end];
                        if node.children.is_empty() {
                            node.text.push_str(data);
                        }
                        self.pos += end + 3;
                        continue;
                    }
                    if rest.starts_with("<?") {
                        self.pos += 2;
                        self.skip_until("?>")?;
                        continue;
                    }
                    if rest.starts_with("<!") {
                        self.skip_declaration()?;
                        continue;
                    }
                    let child = self.element(scopes)?;
                    node.children.push(child);
                }
                Some(_) => {
                    let start = self.pos;
                    while self.byte().is_some_and(|byte| byte != b'<') {
                        self.pos += 1;
                    }
                    let text = decode_entities(&self.text[start..self.pos], false)?;
                    if node.children.is_empty() {
                        node.text.push_str(&text);
                    }
                }
            }
        }
    }
}

fn expand_name(name: &str, scope: &BTreeMap<String, String>, element: bool) -> Fallible<String> {
    match name.split_once(':') {
        Some((prefix, local)) => {
            let namespace = scope
                .get(prefix)
                .ok_or_else(|| format!("unbound XML namespace prefix {prefix}"))?;
            Ok(format!("{{{namespace}}}{local}"))
        }
        None if element => match scope.get("") {
            Some(namespace) if !namespace.is_empty() => Ok(format!("{{{namespace}}}{name}")),
            _ => Ok(name.to_string()),
        },
        None => Ok(name.to_string()),
    }
}

fn decode_entities(text: &str, normalize_whitespace: bool) -> Fallible<String> {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(start) = rest.find('&') {
        for character in rest[..start].chars() {
            out.push(normalize(character, normalize_whitespace));
        }
        rest = &rest[start + 1..];
        let end = rest
            .find(';')
            .ok_or_else(|| "unterminated XML entity".to_string())?;
        let entity = &rest[..end];
        let character = match entity {
            "amp" => '&',
            "lt" => '<',
            "gt" => '>',
            "quot" => '"',
            "apos" => '\'',
            _ => {
                let number = if let Some(hex) = entity.strip_prefix("#x") {
                    u32::from_str_radix(hex, 16).ok()
                } else if let Some(decimal) = entity.strip_prefix('#') {
                    decimal.parse::<u32>().ok()
                } else {
                    None
                }
                .ok_or_else(|| format!("unsupported XML entity &{entity};"))?;
                char::from_u32(number)
                    .ok_or_else(|| "invalid XML character reference".to_string())?
            }
        };
        out.push(character);
        rest = &rest[end + 1..];
    }
    for character in rest.chars() {
        out.push(normalize(character, normalize_whitespace));
    }
    Ok(out)
}
fn normalize(character: char, normalize_whitespace: bool) -> char {
    if normalize_whitespace && matches!(character, '\t' | '\n' | '\r') {
        ' '
    } else {
        character
    }
}

fn strip_declarations(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(start) = rest.find("<?xml") {
        out.push_str(&rest[..start]);
        match rest[start..].find("?>") {
            Some(end) => rest = &rest[start + end + 2..],
            None => {
                rest = "";
                break;
            }
        }
    }
    out.push_str(rest);
    out
}

// ---------------------------------------------------------------------------
// Value merges and bitfield expressions.
// ---------------------------------------------------------------------------
fn flags(parent: &[String], value: &str) -> Vec<String> {
    let tokens = value.split_whitespace().collect::<Vec<_>>();
    let mut set = if tokens.iter().any(|token| token.starts_with(['+', '-'])) {
        parent.iter().cloned().collect::<BTreeSet<_>>()
    } else {
        BTreeSet::new()
    };
    for token in tokens {
        if let Some(removed) = token.strip_prefix('-') {
            set.remove(removed);
        } else {
            set.insert(token.trim_start_matches('+').to_string());
        }
    }
    set.into_iter().collect()
}

fn merge(base: &Node, update: &Node) -> Node {
    let mut result = base.clone();
    for (key, value) in &update.attributes {
        result.attributes.insert(key.clone(), value.clone());
    }
    for bitfield in ["KindOf", "Surfaces"] {
        if let Some(value) = update.attributes.get(bitfield) {
            let parent = flags(&[], base.attr(bitfield).unwrap_or(""));
            result
                .attributes
                .insert(bitfield.to_string(), flags(&parent, value).join(" "));
        }
    }
    if !update.text.is_empty() && !update.text.trim().is_empty() {
        result.text = update.text.clone();
    }
    let mut groups: Vec<(String, Vec<&Node>)> = Vec::new();
    for node in &update.children {
        let name = node.local().to_string();
        match groups.iter_mut().find(|(group, _)| *group == name) {
            Some((_, nodes)) => nodes.push(node),
            None => groups.push((name, vec![node])),
        }
    }
    for (name, updates) in groups {
        let previous = result
            .children
            .iter()
            .enumerate()
            .filter(|(_, child)| child.local() == name)
            .map(|(index, _)| index)
            .collect::<Vec<_>>();
        if matches!(name.as_str(), "Shape" | "ContactPoint")
            && updates.iter().all(|node| node.attr("Name").is_none())
        {
            result
                .children
                .retain(|child| child.local() != name.as_str());
            result
                .children
                .extend(updates.iter().map(|node| (*node).clone()));
            continue;
        }
        for node in updates.iter().copied() {
            let key = if node.attr("id").is_some() {
                "id"
            } else {
                "Name"
            };
            let value = node.attr(key);
            let mut matches = result
                .children
                .iter()
                .enumerate()
                .filter(|(_, old)| old.local() == name && value.is_some() && old.attr(key) == value)
                .map(|(index, _)| index)
                .collect::<Vec<_>>();
            if name == "LocomotorSet" && value.is_none() {
                matches = result
                    .children
                    .iter()
                    .enumerate()
                    .filter(|(_, old)| {
                        old.local() == name
                            && old.attr("Condition") == node.attr("Condition")
                            && (node.attr("Locomotor").is_none()
                                || old.attr("Locomotor") == node.attr("Locomotor"))
                    })
                    .map(|(index, _)| index)
                    .collect();
                if matches.len() > 1 {
                    matches.clear();
                }
            }
            if value.is_none() && updates.len() == 1 && previous.len() == 1 {
                matches = previous.clone();
            }
            let action = node.attr(JOIN).unwrap_or("Merge");
            match matches.first() {
                Some(&position) => {
                    if action == "Remove" {
                        result.children.remove(position);
                    } else {
                        let replacement = if action == "Replace" {
                            (*node).clone()
                        } else {
                            merge(&result.children[position], node)
                        };
                        result.children.remove(position);
                        result.children.insert(position, replacement);
                    }
                }
                None if action != "Remove" => result.children.push((*node).clone()),
                None => {}
            }
        }
    }
    result
}

#[derive(Debug)]
enum Expr {
    Literal(String),
    Name(String),
    Add(Box<Expr>, Box<Expr>),
}
struct ExprParser {
    chars: Vec<char>,
    pos: usize,
}
impl ExprParser {
    fn new(text: &str) -> Self {
        Self {
            chars: text.chars().collect(),
            pos: 0,
        }
    }
    fn parse(mut self) -> Fallible<Expr> {
        let mut left = self.term()?;
        loop {
            self.skip_space();
            if self.chars.get(self.pos) != Some(&'+') {
                break;
            }
            self.pos += 1;
            let right = self.term()?;
            left = Expr::Add(Box::new(left), Box::new(right));
        }
        self.skip_space();
        if self.pos != self.chars.len() {
            return Err("unsupported bitfield expression".into());
        }
        Ok(left)
    }
    fn skip_space(&mut self) {
        while self
            .chars
            .get(self.pos)
            .is_some_and(|character| character.is_whitespace())
        {
            self.pos += 1;
        }
    }
    fn term(&mut self) -> Fallible<Expr> {
        self.skip_space();
        match self.chars.get(self.pos).copied() {
            Some(quote @ ('\'' | '"')) => {
                self.pos += 1;
                let mut literal = String::new();
                loop {
                    let character = *self
                        .chars
                        .get(self.pos)
                        .ok_or_else(|| "unterminated string literal".to_string())?;
                    self.pos += 1;
                    if character == quote {
                        return Ok(Expr::Literal(literal));
                    }
                    if character == '\\' {
                        let escaped = *self
                            .chars
                            .get(self.pos)
                            .ok_or_else(|| "unterminated string escape".to_string())?;
                        self.pos += 1;
                        literal.push(match escaped {
                            'n' => '\n',
                            't' => '\t',
                            'r' => '\r',
                            other => other,
                        });
                    } else {
                        literal.push(character);
                    }
                }
            }
            Some(character) if character.is_ascii_alphabetic() || character == '_' => {
                let start = self.pos;
                while self
                    .chars
                    .get(self.pos)
                    .is_some_and(|c| c.is_ascii_alphanumeric() || *c == '_')
                {
                    self.pos += 1;
                }
                Ok(Expr::Name(self.chars[start..self.pos].iter().collect()))
            }
            _ => Err("unsupported bitfield expression".into()),
        }
    }
}

fn substitute_macros(text: &str) -> String {
    let chars = text.chars().collect::<Vec<_>>();
    let mut out = String::new();
    let mut index = 0;
    while index < chars.len() {
        if chars[index] != '$' {
            out.push(chars[index]);
            index += 1;
            continue;
        }
        let start = index + 1;
        let mut end = start;
        if chars
            .get(start)
            .is_some_and(|c| c.is_ascii_alphabetic() || *c == '_')
        {
            end += 1;
            while chars
                .get(end)
                .is_some_and(|c| c.is_ascii_alphanumeric() || *c == '_')
            {
                end += 1;
            }
            out.extend(&chars[start..end]);
            index = end;
        } else {
            out.push('$');
            index += 1;
        }
    }
    out
}

fn evaluate(
    expr: &Expr,
    defines: &BTreeMap<String, String>,
    conflicts: &BTreeSet<String>,
    stack: &[String],
) -> Fallible<String> {
    match expr {
        Expr::Literal(text) => Ok(text.clone()),
        Expr::Name(name) => {
            if stack.iter().any(|previous| previous == name) {
                return Err(format!("cyclic macro: {name}"));
            }
            if !defines.contains_key(name) || conflicts.contains(name) {
                return Err(format!("missing or ambiguous macro: {name}"));
            }
            let mut next = stack.to_vec();
            next.push(name.clone());
            resolve_bitfield(defines[name].as_str(), defines, conflicts, &next)
        }
        Expr::Add(left, right) => Ok(format!(
            "{}{}",
            evaluate(left, defines, conflicts, stack)?,
            evaluate(right, defines, conflicts, stack)?
        )),
    }
}

fn resolve_bitfield(
    text: &str,
    defines: &BTreeMap<String, String>,
    conflicts: &BTreeSet<String>,
    stack: &[String],
) -> Fallible<String> {
    let text = text.trim();
    if !text.starts_with('=') && !text.contains('$') {
        return Ok(text.to_string());
    }
    let expression = substitute_macros(text.trim_start_matches('=').trim());
    evaluate(
        &ExprParser::new(&expression).parse()?,
        defines,
        conflicts,
        stack,
    )
}

// ---------------------------------------------------------------------------
// Importer
// ---------------------------------------------------------------------------
#[derive(Debug, Clone)]
struct ManifestRow {
    bytes: u64,
    sha256: String,
}
#[derive(Debug, Clone)]
struct Definition {
    kind: String,
    name: String,
    path: PathBuf,
    node: Node,
}
type ResolvedDefinitions = BTreeMap<(String, String, PathBuf), (Node, Vec<String>, Vec<String>)>;

struct Importer {
    root: PathBuf,
    mirror: PathBuf,
    files: BTreeMap<String, PathBuf>,
    inputs: BTreeMap<String, String>,
    manifest: BTreeMap<String, ManifestRow>,
    documents: BTreeMap<PathBuf, Option<Node>>,
    dependencies: Vec<Value>,
    edges: BTreeMap<PathBuf, BTreeSet<PathBuf>>,
    definitions: Vec<Definition>,
    defines: BTreeMap<String, String>,
    define_conflicts: BTreeSet<String>,
    diagnostics: Vec<String>,
    resolved: ResolvedDefinitions,
    vector_defaults: BTreeMap<usize, BTreeMap<String, String>>,
    object_defaults: BTreeMap<String, String>,
}

fn posix(path: &Path) -> String {
    path.components()
        .filter_map(|component| match component {
            Component::Normal(part) => Some(part.to_string_lossy().into_owned()),
            Component::CurDir => None,
            Component::ParentDir => Some("..".to_string()),
            Component::RootDir | Component::Prefix(_) => None,
        })
        .collect::<Vec<_>>()
        .join("/")
}

impl Importer {
    fn new(root: &Path) -> Fallible<Self> {
        let root =
            fs::canonicalize(root).map_err(|error| format!("invalid project root: {error}"))?;
        let mirror = root.join("data/ea-modding/red-alert-3");
        let mut files = BTreeMap::new();
        walk_files(&mirror, &mut |path| {
            let key = posix(
                path.strip_prefix(&mirror)
                    .map_err(|_| "SDK file outside the mirror".to_string())?,
            )
            .to_lowercase();
            if files.insert(key.clone(), path.to_path_buf()).is_some() {
                return Err(format!("ambiguous case-insensitive SDK filename: {key}"));
            }
            Ok(())
        })?;
        let manifest_path = root.join("data/ea-modding/manifest.jsonl");
        let manifest_bytes =
            fs::read(&manifest_path).map_err(|error| format!("missing SDK manifest: {error}"))?;
        let mut inputs = BTreeMap::new();
        inputs.insert(
            posix(
                manifest_path
                    .strip_prefix(&root)
                    .map_err(|_| "manifest outside the project root".to_string())?,
            ),
            sha256::hex(&manifest_bytes),
        );
        let mut manifest = BTreeMap::new();
        for line in String::from_utf8(manifest_bytes)
            .map_err(|_| "SDK manifest is not UTF-8".to_string())?
            .lines()
            .filter(|line| !line.trim().is_empty())
        {
            let row: Value = serde_json::from_str(line)
                .map_err(|error| format!("invalid manifest row: {error}"))?;
            let path = row["path"]
                .as_str()
                .ok_or_else(|| "manifest row without a path".to_string())?;
            let bytes = row["bytes"]
                .as_u64()
                .ok_or_else(|| "manifest row without a byte count".to_string())?;
            let digest = row["sha256"]
                .as_str()
                .ok_or_else(|| "manifest row without a digest".to_string())?;
            manifest.insert(
                path.to_lowercase(),
                ManifestRow {
                    bytes,
                    sha256: digest.to_string(),
                },
            );
        }
        Ok(Self {
            root,
            mirror,
            files,
            inputs,
            manifest,
            documents: BTreeMap::new(),
            dependencies: Vec::new(),
            edges: BTreeMap::new(),
            definitions: Vec::new(),
            defines: BTreeMap::new(),
            define_conflicts: BTreeSet::new(),
            diagnostics: Vec::new(),
            resolved: BTreeMap::new(),
            vector_defaults: BTreeMap::new(),
            object_defaults: BTreeMap::new(),
        })
    }
    fn label(&self, path: &Path) -> Fallible<String> {
        path.strip_prefix(&self.mirror)
            .map(posix)
            .map_err(|_| format!("path outside the SDK mirror: {}", path.display()))
    }
    fn read(&mut self, path: &Path) -> Fallible<Vec<u8>> {
        let data = fs::read(path).map_err(|error| format!("{}: {error}", path.display()))?;
        let digest = sha256::hex(&data);
        if let Ok(relative) = path.strip_prefix(&self.mirror) {
            let name = posix(relative);
            let expected = self.manifest.get(&name.to_lowercase()).ok_or_else(|| {
                format!("SDK input differs from the pinned mirror manifest: {name}")
            })?;
            if expected.sha256 != digest || expected.bytes != data.len() as u64 {
                return Err(format!(
                    "SDK input differs from the pinned mirror manifest: {name}"
                ));
            }
        }
        let relative = path
            .strip_prefix(&self.root)
            .map_err(|_| format!("input outside the project root: {}", path.display()))?;
        self.inputs.insert(posix(relative), digest);
        Ok(data)
    }
    fn locate(&self, reference: &str, source: Option<&Path>) -> Option<PathBuf> {
        let key = match reference.split_once(':') {
            Some((namespace, relative)) if namespace.eq_ignore_ascii_case("DATA") => {
                format!("Xml/{}", relative.replace('\\', "/"))
            }
            Some(_) => return None,
            None => match source {
                Some(source) => {
                    let parent = source.parent()?;
                    let relative = parent.strip_prefix(&self.mirror).unwrap_or(Path::new(""));
                    posix(&relative.join(reference.replace('\\', "/")))
                }
                None => reference.to_string(),
            },
        };
        self.files.get(&key.to_lowercase()).cloned()
    }
    fn parse(&mut self, path: &Path) -> Fallible<Node> {
        let data = self.read(path)?;
        let text = std::str::from_utf8(&data).map_err(|_| "XML is not valid UTF-8".to_string())?;
        match XmlParser::parse_root(text) {
            Ok(node) => Ok(node),
            Err(_) => {
                // Some includes are one/multiple standalone behavior fragments, or
                // have comments before an XML declaration. Do not repair their data.
                let cleaned = strip_declarations(text.strip_prefix('\u{feff}').unwrap_or(text));
                XmlParser::parse_root(&format!(
                    "<AssetDeclaration xmlns=\"{ASSET_NS}\" xmlns:xi=\"{XI_NS}\">{cleaned}</AssetDeclaration>"
                ))
            }
        }
    }
    fn load(&mut self, path: &Path) -> Fallible<()> {
        if self.documents.contains_key(path) {
            return Ok(());
        }
        self.documents.insert(path.to_path_buf(), None);
        self.edges.insert(path.to_path_buf(), BTreeSet::new());
        let document = match self.parse(path) {
            Ok(document) => document,
            Err(error) => {
                let label = self
                    .label(path)
                    .unwrap_or_else(|_| path.display().to_string());
                self.diagnostics
                    .push(format!("unparsed XML {label}: {error}"));
                return Ok(());
            }
        };
        self.documents
            .insert(path.to_path_buf(), Some(document.clone()));
        let mut nodes = Vec::new();
        document.walk(&mut nodes);
        for node in nodes.iter().copied() {
            let (reference, kind) = if node.local() == "Include" {
                match node.attr("source") {
                    Some(source) => (
                        source.to_string(),
                        node.attr("type").unwrap_or("").to_string(),
                    ),
                    None => continue,
                }
            } else if node.tag == format!("{{{XI_NS}}}include") {
                match node.attr("href") {
                    Some(href) => (href.to_string(), String::from("xinclude")),
                    None => continue,
                }
            } else {
                continue;
            };
            let target = self.locate(&reference, Some(path));
            self.dependencies.push(serde_json::json!({
                "source": self.label(path)?,
                "target": reference,
                "kind": kind,
                "resolved": target.as_ref().map(|target| self.label(target)).transpose()?,
            }));
            if let Some(target) = target
                && target
                    .extension()
                    .is_some_and(|extension| extension.eq_ignore_ascii_case("xml"))
            {
                self.edges
                    .entry(path.to_path_buf())
                    .or_default()
                    .insert(target.clone());
                self.load(&target)?;
            }
        }
        for node in nodes {
            let name = node.local();
            if name == "Define"
                && let Some(define) = node.attr("name")
            {
                let value = node.attr("value").unwrap_or("");
                match self.defines.get(define) {
                    None => {
                        self.defines.insert(define.to_string(), value.to_string());
                    }
                    Some(old) if old == value => {}
                    Some(_) => {
                        self.define_conflicts.insert(define.to_string());
                    }
                }
            }
            if matches!(name, "GameObject" | "LocomotorTemplate")
                && let Some(id) = node.attr("id")
            {
                self.definitions.push(Definition {
                    kind: name.to_string(),
                    name: id.to_lowercase(),
                    path: path.to_path_buf(),
                    node: node.clone(),
                });
            }
        }
        Ok(())
    }
    fn closure(&self, start: &Path) -> BTreeSet<PathBuf> {
        let mut found = BTreeSet::new();
        let mut stack = vec![start.to_path_buf()];
        while let Some(current) = stack.pop() {
            if found.insert(current.clone())
                && let Some(edges) = self.edges.get(&current)
            {
                stack.extend(edges.iter().cloned());
            }
        }
        found
    }
    fn candidates(&self, kind: &str, name: &str) -> Vec<&Definition> {
        let name = name.to_lowercase();
        self.definitions
            .iter()
            .filter(|definition| definition.kind == kind && definition.name == name)
            .collect()
    }
    fn choose(
        &self,
        kind: &str,
        name: &str,
        source: Option<&Path>,
        scope: Option<&BTreeSet<PathBuf>>,
    ) -> Option<Definition> {
        let mut candidates = self.candidates(kind, name);
        if let Some(source) = source {
            let local = candidates
                .iter()
                .filter(|candidate| candidate.path == source)
                .collect::<Vec<_>>();
            if local.len() == 1 {
                return Some((*local[0]).clone());
            }
            let reachable = self.closure(source);
            let scoped = candidates
                .iter()
                .filter(|candidate| reachable.contains(&candidate.path))
                .collect::<Vec<_>>();
            if scoped.len() == 1 {
                return Some((*scoped[0]).clone());
            }
        }
        if let Some(scope) = scope {
            candidates.retain(|candidate| scope.contains(&candidate.path));
        }
        if candidates.len() == 1 {
            Some(candidates[0].clone())
        } else {
            None
        }
    }
    fn expand(
        &self,
        node: &Node,
        source: &Path,
        pending: &mut Vec<String>,
        active: &[PathBuf],
    ) -> Node {
        let mut result = node.clone();
        for index in (0..result.children.len()).rev() {
            let include = result.children[index].tag == format!("{{{XI_NS}}}include");
            if !include {
                let expanded = self.expand(&result.children[index], source, pending, active);
                result.children[index] = expanded;
                continue;
            }
            let href = result.children[index]
                .attr("href")
                .unwrap_or("")
                .to_string();
            let pointer = result.children[index].attr("xpointer").map(str::to_string);
            let target = self.locate(&href, Some(source));
            let usable = target.as_ref().is_some_and(|target| {
                self.documents
                    .get(target)
                    .is_some_and(|document| document.is_some())
                    && !active.contains(target)
            });
            if !usable {
                pending.push(format!(
                    "unresolved XInclude: {}",
                    result.children[index].attr("href").unwrap_or("None")
                ));
                result.children.remove(index);
                continue;
            }
            let target = target.unwrap();
            let document = self.documents[&target].as_ref().unwrap();
            let mut next = active.to_vec();
            next.push(target.clone());
            let expanded = self.expand(document, &target, pending, &next);
            let values = match pointer {
                Some(pointer) => match pointer_selection(&pointer) {
                    Some(name) if expanded.local() == name => expanded.children.clone(),
                    _ => {
                        pending.push(format!("unsupported XInclude selector: {pointer}"));
                        Vec::new()
                    }
                },
                None if expanded.local() == "AssetDeclaration" => expanded.children.clone(),
                None => vec![expanded],
            };
            result.children.remove(index);
            for (offset, value) in values.into_iter().enumerate() {
                result.children.insert(index + offset, value);
            }
        }
        result
    }
    fn definition(
        &mut self,
        kind: &str,
        name: &str,
        source: Option<&Path>,
        scope: Option<&BTreeSet<PathBuf>>,
        stack: &[(String, String, PathBuf)],
    ) -> (Option<Node>, Vec<String>, Vec<String>) {
        let Some(selected) = self.choose(kind, name, source, scope) else {
            return (
                None,
                Vec::new(),
                vec![format!("missing or ambiguous {kind}: {name}")],
            );
        };
        let key = (kind.to_string(), name.to_lowercase(), selected.path.clone());
        if let Some((node, lineage, pending)) = self.resolved.get(&key) {
            return (Some(node.clone()), lineage.clone(), pending.clone());
        }
        if stack.contains(&key) {
            return (None, Vec::new(), vec![format!("inheritance cycle: {name}")]);
        }
        let mut pending = Vec::new();
        let mut lineage = vec![format!(
            "{}#{}",
            self.label(&selected.path)
                .unwrap_or_else(|_| selected.path.display().to_string()),
            selected.node.attr("id").unwrap_or("None")
        )];
        let mut own = self.expand(&selected.node, &selected.path, &mut pending, &[]);
        for field in ["KindOf", "Surfaces"] {
            if let Some(value) = own.attributes.get(field).cloned() {
                match self.bitfield(&value, &mut pending, &format!("{name}.{field}")) {
                    // Keep known inherited bits when a child cannot be resolved;
                    // pending still prevents that metadata from qualifying.
                    None => {
                        own.attributes.remove(field);
                    }
                    Some(expanded) => {
                        own.attributes.insert(field.to_string(), expanded);
                    }
                }
            }
        }
        if let Some(parent) = selected.node.attr("inheritFrom").map(str::to_string) {
            let parent = parent.rsplit(':').next().unwrap_or(&parent).to_string();
            let mut next = stack.to_vec();
            next.push(key.clone());
            let (inherited, chain, errors) =
                self.definition(kind, &parent, Some(&selected.path), scope, &next);
            pending.extend(errors);
            lineage.extend(chain);
            if let Some(inherited) = inherited {
                own = merge(&inherited, &own);
            }
        }
        self.resolved
            .insert(key, (own.clone(), lineage.clone(), pending.clone()));
        (Some(own), lineage, pending)
    }
    fn classification(&mut self, name: &str, scope: &BTreeSet<PathBuf>) -> Option<Value> {
        let candidates = self
            .candidates("GameObject", name)
            .into_iter()
            .filter(|candidate| scope.contains(&candidate.path))
            .map(|candidate| candidate.path.clone())
            .collect::<BTreeSet<_>>();
        if candidates.len() < 2 {
            return None;
        }
        let mut definitions = Vec::new();
        for source in &candidates {
            let (node, _, pending) =
                self.definition("GameObject", name, Some(source), Some(scope), &[]);
            let node = node?;
            if !pending.is_empty() {
                return None;
            }
            definitions.push((
                node.attr("id").map(str::to_string),
                flags(&[], node.attr("KindOf").unwrap_or("")),
            ));
        }
        let (first_id, first_bits) = definitions.first()?.clone();
        if definitions.iter().any(|(_, bits)| *bits != first_bits) {
            return None;
        }
        let mut fact = Map::new();
        fact.insert("id".into(), first_id.map_or(Value::Null, Value::String));
        fact.insert(
            "kind_of".into(),
            Value::Array(first_bits.into_iter().map(Value::String).collect()),
        );
        fact.insert(
            "equivalent_sources".into(),
            Value::Array(
                candidates
                    .iter()
                    .map(|path| self.label(path).map(Value::String))
                    .collect::<Fallible<Vec<_>>>()
                    .ok()?,
            ),
        );
        fact.insert(
            "scope".into(),
            Value::String(
                "classification-only consensus across all scoped definitions; full source and geometry remain ambiguous"
                    .into(),
            ),
        );
        Some(Value::Object(fact))
    }
    fn defaults(&mut self, relative: &str, typename: &str) -> Fallible<BTreeMap<String, String>> {
        let path = self
            .locate(relative, None)
            .ok_or_else(|| format!("schema not found: {relative}"))?;
        let document = self.parse(&path)?;
        for node in &document.children {
            if node.attr("name") != Some(typename) {
                continue;
            }
            let mut nodes = Vec::new();
            node.walk(&mut nodes);
            let mut defaults = BTreeMap::new();
            for attribute in nodes {
                if attribute.local() != "attribute" {
                    continue;
                }
                let (Some(name), Some(default)) =
                    (attribute.attr("name"), attribute.attr("default"))
                else {
                    continue;
                };
                defaults.insert(name.to_string(), default.to_string());
            }
            return Ok(defaults);
        }
        Err(format!("schema has no {typename}: {relative}"))
    }
    fn bitfield(&self, value: &str, pending: &mut Vec<String>, context: &str) -> Option<String> {
        let result = resolve_bitfield(value, &self.defines, &self.define_conflicts, &[]).and_then(
            |result| {
                if result.contains(['$', '=', '\'', '"']) {
                    Err("unexpanded bitfield expression".to_string())
                } else {
                    Ok(result)
                }
            },
        );
        match result {
            Ok(result) => Some(result),
            Err(error) => {
                pending.push(format!("unresolved bitfield {context}: {error}"));
                None
            }
        }
    }
    fn number(
        &self,
        value: &str,
        pending: &mut Vec<String>,
        context: &str,
        stack: &[String],
    ) -> f64 {
        fn unresolved(pending: &mut Vec<String>, context: &str, value: &str) -> f64 {
            pending.push(format!("unresolved numeric value {context}: {value}"));
            0.0
        }
        let mut text = value.trim();
        if let Some(stripped) = text.strip_prefix('=') {
            text = stripped;
        }
        if let Some(name) = text.strip_prefix('$')
            && self.defines.contains_key(name)
        {
            if stack.iter().any(|previous| previous == name) || self.define_conflicts.contains(name)
            {
                return unresolved(pending, context, value);
            }
            let mut next = stack.to_vec();
            next.push(name.to_string());
            return self.number(&self.defines[name], pending, context, &next);
        }
        match text.parse::<f64>() {
            Ok(number) if number.is_finite() => number,
            _ => unresolved(pending, context, value),
        }
    }
    fn angle(&self, value: &str, pending: &mut Vec<String>, context: &str) -> f64 {
        let text = value.trim();
        let suffix = text
            .chars()
            .last()
            .filter(|character| matches!(character, 'd' | 'r'));
        let number = match suffix {
            Some(_) => self.number(&text[..text.len() - 1], pending, context, &[]),
            None => self.number(text, pending, context, &[]),
        };
        if suffix == Some('d') {
            number * (std::f64::consts::PI / 180.0)
        } else {
            number
        }
    }
    fn vector(
        &self,
        node: Option<&Node>,
        pending: &mut Vec<String>,
        context: &str,
        count: usize,
    ) -> Vec<f64> {
        let defaults = self.vector_defaults.get(&count);
        ["x", "y", "z"]
            .iter()
            .take(count)
            .map(|axis| {
                let fallback = defaults
                    .and_then(|defaults| defaults.get(*axis))
                    .map(String::as_str)
                    .unwrap_or("0");
                let value = node
                    .and_then(|node| node.attr(axis))
                    .unwrap_or(fallback)
                    .to_string();
                self.number(&value, pending, &format!("{context}.{axis}"), &[])
            })
            .collect()
    }
    fn shape(
        &self,
        node: &Node,
        defaults: &BTreeMap<String, String>,
        pending: &mut Vec<String>,
    ) -> Fallible<Value> {
        let mut values = defaults.clone();
        for (key, value) in &node.attributes {
            values.insert(key.clone(), value.clone());
        }
        let required = |key: &str| {
            values
                .get(key)
                .cloned()
                .ok_or_else(|| format!("schema default missing: {key}"))
        };
        let kind = required("Type")?;
        let active = required("Active")?.to_lowercase() == "true";
        let major_radius = self.number(&required("MajorRadius")?, pending, "MajorRadius", &[]);
        let minor_radius = self.number(&required("MinorRadius")?, pending, "MinorRadius", &[]);
        let height = self.number(&required("Height")?, pending, "Height", &[]);
        let offset = self.vector(node.child("Offset"), pending, "Offset", 3);
        let front = values
            .get("FrontAngle")
            .map(|value| self.angle(value, pending, "FrontAngle"));
        if !matches!(kind.as_str(), "BOX" | "CYLINDER" | "SPHERE" | "RING") {
            pending.push(format!("unknown shape type: {kind}"));
        }
        if let Some(other) = values.get("Other")
            && !other.is_empty()
        {
            pending.push(format!("shape has unresolved Other semantics: {other}"));
        }
        let mut shape = Map::new();
        shape.insert("kind".into(), Value::String(kind));
        shape.insert(
            "name".into(),
            values
                .get("Name")
                .map_or(Value::Null, |value| Value::String(value.clone())),
        );
        shape.insert("major_radius".into(), Value::from(major_radius));
        shape.insert("minor_radius".into(), Value::from(minor_radius));
        shape.insert("height".into(), Value::from(height));
        shape.insert(
            "offset".into(),
            Value::Array(offset.into_iter().map(Value::from).collect()),
        );
        shape.insert("active".into(), Value::Bool(active));
        shape.insert(
            "front_angle_radians".into(),
            front.map_or(Value::Null, Value::from),
        );
        Ok(Value::Object(shape))
    }
    fn object(
        &mut self,
        name: &str,
        selected_source: Option<&Path>,
        scope: Option<&BTreeSet<PathBuf>>,
        inputs: &Inputs,
    ) -> Fallible<Option<Value>> {
        let (node, lineage, mut pending) =
            self.definition("GameObject", name, selected_source, scope, &[]);
        let Some(node) = node else {
            self.diagnostics.extend(pending);
            return Ok(None);
        };
        let source = lineage
            .first()
            .and_then(|lineage| lineage.rsplit_once('#'))
            .map_or(String::new(), |(source, _)| source.to_string());
        let geometry = node.child("Geometry");
        let shapes = match geometry {
            Some(geometry) => geometry
                .children_named("Shape")
                .into_iter()
                .map(|shape| self.shape(shape, &inputs.shape, &mut pending))
                .collect::<Fallible<Vec<_>>>()?,
            None => Vec::new(),
        };
        let placement = geometry.and_then(|geometry| geometry.child("PlacementShape"));
        let anchor = geometry.and_then(|geometry| geometry.child("RotationAnchorOffset"));
        let mut nodes = Vec::new();
        node.walk(&mut nodes);
        let production = nodes
            .iter()
            .copied()
            .filter(|module| module.local() == "QueueProductionExitUpdate")
            .collect::<Vec<_>>();
        if production.len() > 1 {
            pending.push("multiple production exit modules".into());
        }
        let production_exit = match production.first() {
            Some(module) => {
                let mut values = inputs.exit.clone();
                for (key, value) in &module.attributes {
                    values.insert(key.clone(), value.clone());
                }
                if module.child("UnitCreatePoint").is_none()
                    || module.child("NaturalRallyPoint").is_none()
                {
                    pending.push("production exit lacks create/rally point".into());
                }
                let create =
                    self.vector(module.child("UnitCreatePoint"), &mut pending, "create", 3);
                let rally =
                    self.vector(module.child("NaturalRallyPoint"), &mut pending, "rally", 3);
                let no_exit_path = values
                    .get("NoExitPath")
                    .ok_or_else(|| "schema default missing: NoExitPath".to_string())?
                    .to_lowercase()
                    == "true";
                let angle = values
                    .get("PlacementViewAngle")
                    .map(|value| self.angle(value, &mut pending, "exit angle"));
                let mut exit = Map::new();
                exit.insert(
                    "create".into(),
                    Value::Array(create.into_iter().map(Value::from).collect()),
                );
                exit.insert(
                    "rally".into(),
                    Value::Array(rally.into_iter().map(Value::from).collect()),
                );
                exit.insert("no_exit_path".into(), Value::Bool(no_exit_path));
                exit.insert(
                    "placement_view_angle_radians".into(),
                    angle.map_or(Value::Null, Value::from),
                );
                Some(Value::Object(exit))
            }
            None => None,
        };
        let restricted = nodes
            .iter()
            .copied()
            .filter(|module| module.local() == "RestrictedAreaShape")
            .collect::<Vec<_>>();
        if restricted.len() > 1 {
            pending.push("multiple restricted area shapes".into());
        }
        let restricted_area = match restricted.len() {
            1 => Some(self.shape(restricted[0], &inputs.shape, &mut pending)?),
            _ => None,
        };
        let mut assisted = Vec::new();
        for module in &nodes {
            if module.local() != "AssistedPlacementInfoBehavior" {
                continue;
            }
            let mut errors = Vec::new();
            let restriction = module.child("PlacementRestriction");
            let filters =
                restriction.and_then(|restriction| restriction.child("RestrictedObjectFilter"));
            let filter_value = |name: &str, fallback: &str| {
                filters
                    .and_then(|filters| filters.attr(name))
                    .unwrap_or(fallback)
                    .to_string()
            };
            let filter_radius = restriction
                .map(|restriction| restriction.attr("FilterRadius").unwrap_or("0").to_string())
                .unwrap_or_else(|| "0".to_string());
            let strings = |children: Vec<&Node>| {
                children
                    .into_iter()
                    .map(|node| node.text.trim().to_string())
                    .collect::<Vec<_>>()
            };
            let mut entry = Map::new();
            entry.insert(
                "suggested_offset".into(),
                Value::Array(
                    self.vector(
                        module.child("SuggestedOffset"),
                        &mut errors,
                        "suggested offset",
                        3,
                    )
                    .into_iter()
                    .map(Value::from)
                    .collect(),
                ),
            );
            entry.insert(
                "filter_radius".into(),
                Value::from(self.number(&filter_radius, &mut errors, "filter radius", &[])),
            );
            entry.insert(
                "filter_rule".into(),
                Value::String(filter_value("Rule", "NONE")),
            );
            entry.insert(
                "relationships".into(),
                Value::Array(
                    filter_value("Relationship", "")
                        .split_whitespace()
                        .map(|value| Value::String(value.to_string()))
                        .collect(),
                ),
            );
            entry.insert(
                "restricted_kind_of".into(),
                Value::Array(
                    filter_value("Include", "")
                        .split_whitespace()
                        .map(|value| Value::String(value.to_string()))
                        .collect(),
                ),
            );
            entry.insert(
                "excluded_kind_of".into(),
                Value::Array(
                    filter_value("Exclude", "")
                        .split_whitespace()
                        .map(|value| Value::String(value.to_string()))
                        .collect(),
                ),
            );
            entry.insert(
                "restricted_objects".into(),
                Value::Array(
                    filters
                        .map(|filters| strings(filters.children_named("IncludeThing")))
                        .unwrap_or_default()
                        .into_iter()
                        .map(Value::String)
                        .collect(),
                ),
            );
            entry.insert(
                "excluded_objects".into(),
                Value::Array(
                    filters
                        .map(|filters| strings(filters.children_named("ExcludeThing")))
                        .unwrap_or_default()
                        .into_iter()
                        .map(Value::String)
                        .collect(),
                ),
            );
            entry.insert(
                "hint_objects".into(),
                Value::Array(
                    module
                        .children_named("HintObject")
                        .into_iter()
                        .filter_map(|node| node.attr("HintDisplayObject"))
                        .map(|value| Value::String(value.to_string()))
                        .collect(),
                ),
            );
            entry.insert(
                "pending".into(),
                Value::Array(errors.into_iter().map(Value::String).collect()),
            );
            assisted.push(Value::Object(entry));
        }
        let scale_value = node
            .attr("Scale")
            .map(str::to_string)
            .or_else(|| inputs.object.get("Scale").cloned())
            .ok_or_else(|| "schema default missing: Scale".to_string())?;
        let scale = self.number(&scale_value, &mut pending, "Scale", &[]);
        if scale != 1.0 {
            pending.push(format!(
                "non-unit Scale requires explicit geometry semantics: {scale}"
            ));
        }
        for module in &nodes {
            if module.local() == "GeometryUpgrade" {
                pending.push("conditional GeometryUpgrade is not a static footprint".into());
            }
        }
        let view_angle = node
            .attr("PlacementViewAngle")
            .map(str::to_string)
            .or_else(|| inputs.object.get("PlacementViewAngle").cloned())
            .ok_or_else(|| "schema default missing: PlacementViewAngle".to_string())?;
        let placement_view_angle = self.angle(&view_angle, &mut pending, "placement view angle");
        let rotation_anchor = self.vector(anchor, &mut pending, "rotation anchor", 2);
        let locomotors = node
            .children_named("LocomotorSet")
            .into_iter()
            .filter(|module| module.attr("Locomotor").is_some())
            .map(|module| {
                let mut locomotor = Map::new();
                locomotor.insert(
                    "condition".into(),
                    Value::String(
                        module
                            .attr("Condition")
                            .unwrap_or("UNSPECIFIED")
                            .to_string(),
                    ),
                );
                locomotor.insert(
                    "locomotor".into(),
                    Value::String(module.attr("Locomotor").unwrap_or("").to_string()),
                );
                locomotor.insert(
                    "speed".into(),
                    module
                        .attr("Speed")
                        .map_or(Value::Null, |speed| Value::String(speed.to_string())),
                );
                Value::Object(locomotor)
            })
            .collect::<Vec<_>>();
        let mut object = Map::new();
        object.insert(
            "id".into(),
            node.attr("id")
                .map_or(Value::Null, |id| Value::String(id.to_string())),
        );
        object.insert("source".into(), Value::String(source));
        object.insert(
            "lineage".into(),
            Value::Array(lineage.into_iter().map(Value::String).collect()),
        );
        object.insert(
            "kind_of".into(),
            Value::Array(
                flags(&[], node.attr("KindOf").unwrap_or(""))
                    .into_iter()
                    .map(Value::String)
                    .collect(),
            ),
        );
        object.insert("shapes".into(), Value::Array(shapes));
        object.insert(
            "placement_shape".into(),
            match placement {
                Some(placement) => self.shape(placement, &inputs.shape, &mut pending)?,
                None => Value::Null,
            },
        );
        object.insert(
            "rotation_anchor".into(),
            Value::Array(rotation_anchor.into_iter().map(Value::from).collect()),
        );
        object.insert(
            "placement_view_angle_radians".into(),
            Value::from(placement_view_angle),
        );
        object.insert(
            "production_exit".into(),
            production_exit.unwrap_or(Value::Null),
        );
        object.insert(
            "restricted_area".into(),
            restricted_area.unwrap_or(Value::Null),
        );
        object.insert("locomotors".into(), Value::Array(locomotors));
        object.insert("assisted_placement".into(), Value::Array(assisted));
        object.insert(
            "preloaded".into(),
            Value::Bool(inputs.preloaded.contains(&name.to_lowercase())),
        );
        pending.sort();
        pending.dedup();
        object.insert(
            "pending".into(),
            Value::Array(pending.into_iter().map(Value::String).collect()),
        );
        Ok(Some(Value::Object(object)))
    }
}

struct Inputs {
    preloaded: BTreeSet<String>,
    shape: BTreeMap<String, String>,
    exit: BTreeMap<String, String>,
    object: BTreeMap<String, String>,
}

fn pointer_selection(pointer: &str) -> Option<String> {
    let rest = match pointer.strip_prefix("xmlns(") {
        Some(rest) => rest.split_once(")")?.1.trim_start(),
        None => pointer,
    };
    let rest = rest.strip_prefix("xpointer(/")?;
    let (prefix, rest) = rest.split_once(':')?;
    let (name, rest) = rest.split_once("/child::*)")?;
    if rest.is_empty()
        && !prefix.is_empty()
        && !name.is_empty()
        && prefix
            .chars()
            .all(|character| character.is_alphanumeric() || character == '_')
        && name
            .chars()
            .all(|character| character.is_alphanumeric() || character == '_')
    {
        Some(name.to_string())
    } else {
        None
    }
}

fn walk_files(directory: &Path, visit: &mut impl FnMut(&Path) -> Fallible<()>) -> Fallible<()> {
    let entries = match fs::read_dir(directory) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(format!("{}: {error}", directory.display())),
    };
    for entry in entries {
        let entry = entry.map_err(|error| format!("{}: {error}", directory.display()))?;
        let path = entry.path();
        let file_type = entry
            .file_type()
            .map_err(|error| format!("{}: {error}", path.display()))?;
        if file_type.is_dir() {
            walk_files(&path, visit)?;
        } else if file_type.is_file()
            || fs::metadata(&path).is_ok_and(|metadata| metadata.is_file())
        {
            visit(&path)?;
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Build
// ---------------------------------------------------------------------------
fn build(root: &Path) -> Fallible<Value> {
    let mut importer = Importer::new(root)?;
    let provenance: Value =
        serde_json::from_slice(&importer.read(&root.join("data/ea-modding/provenance.json"))?)
            .map_err(|error| format!("invalid provenance: {error}"))?;
    let revision = provenance["revision"]
        .as_str()
        .ok_or_else(|| "provenance without a revision".to_string())?;
    if revision != REVISION {
        return Err(
            "SDK revision changed; explicitly review and update the pinned extractor".into(),
        );
    }
    let checkout = Path::new(env!("CARGO_MANIFEST_DIR"));
    for source in [EXTRACTOR, SUPPORT] {
        let digest = sha256::hex(&fs::read(checkout.join(source)).stringify()?);
        importer.inputs.insert(source.to_string(), digest);
    }
    let registry: Value = serde_json::from_slice(
        &importer.read(&root.join("templates/resources/stock-object-sources.json"))?,
    )
    .map_err(|error| format!("invalid object registry: {error}"))?;
    let membership: Vec<String> = serde_json::from_slice(
        &importer.read(&root.join("templates/resources/stock-global-objects.json"))?,
    )
    .map_err(|error| format!("invalid global membership: {error}"))?;
    let preloaded = membership
        .iter()
        .map(|name| name.to_lowercase())
        .collect::<BTreeSet<_>>();
    let static_source = importer
        .locate("DATA:StaticGameObjects.xml", None)
        .ok_or_else(|| "StaticGameObjects.xml is missing".to_string())?;
    importer.load(&static_source)?;
    let static_scope = importer.closure(&static_source);
    let registry = match registry {
        Value::Object(registry) => registry,
        _ => return Err("object registry must be an object".into()),
    };
    let mut explicit = BTreeMap::new();
    for (name, uri) in &registry {
        let Some(uri) = uri.as_str() else {
            continue;
        };
        let path = importer
            .locate(uri, None)
            .ok_or_else(|| format!("verified object source not found: {name}: {uri}"))?;
        importer.load(&path)?;
        explicit.insert(name.to_lowercase(), path);
    }
    for uri in [
        "DATA:Terrain/Terrain.xml",
        "DATA:GlobalData/Locomotor.xml",
        "DATA:SkirmishAI/AIData.xml",
    ] {
        let path = importer
            .locate(uri, None)
            .ok_or_else(|| format!("SDK source is missing: {uri}"))?;
        importer.load(&path)?;
    }
    let shape = importer.defaults("Schemas/Includes/Geometry.xsd", "Shape")?;
    let exit = importer.defaults(
        "Schemas/Modules/QueueProductionExitUpdate.xsd",
        "QueueProductionExitUpdateModuleData",
    )?;
    let locomotor_defaults = importer.defaults(
        "Schemas/AssetTypeLocomotorTemplate.xsd",
        "LocomotorTemplate",
    )?;
    let ai_defaults = importer.defaults("Schemas/AssetTypeAIData.xsd", "AIData")?;
    importer.object_defaults =
        importer.defaults("Schemas/AssetTypeGameObject.xsd", "GameObject")?;
    for count in [2, 3] {
        let defaults = importer.defaults("Schemas/Includes/Base.xsd", &format!("Vector{count}"))?;
        importer.vector_defaults.insert(count, defaults);
    }
    // These schemas define the object field and bitfield semantics used here.
    for schema in [
        "Schemas/AssetTypeGameObject.xsd",
        "Schemas/Includes/KindOf.xsd",
        "Schemas/Includes/ObjectFilter.xsd",
        "Schemas/Modules/AssistedPlacementInfoBehavior.xsd",
        "Schemas/Modules/StructurePlacementBehavior.xsd",
    ] {
        let path = importer
            .locate(schema, None)
            .ok_or_else(|| format!("schema is missing: {schema}"))?;
        importer.read(&path)?;
    }
    let mut names = importer
        .definitions
        .iter()
        .filter(|definition| {
            definition.kind == "GameObject" && static_scope.contains(&definition.path)
        })
        .map(|definition| definition.name.clone())
        .collect::<BTreeSet<_>>();
    names.extend(registry.keys().map(|name| name.to_lowercase()));
    let inputs = Inputs {
        preloaded: preloaded.clone(),
        shape: shape.clone(),
        exit: exit.clone(),
        object: importer.object_defaults.clone(),
    };
    let mut objects = BTreeMap::new();
    for name in &names {
        if let Some(value) = importer.object(
            name,
            explicit.get(name).map(PathBuf::as_path),
            Some(&static_scope),
            &inputs,
        )? {
            objects.insert(name.clone(), value);
        }
    }
    for name in preloaded.iter().filter(|name| !objects.contains_key(*name)) {
        importer.diagnostics.push(format!(
            "WBStatic_12 membership has no unambiguous SDK definition: {name}"
        ));
    }
    let mut classification_scope = static_scope.clone();
    for path in explicit.values() {
        classification_scope.extend(importer.closure(path));
    }
    let mut classifications = BTreeMap::new();
    for name in names.iter().filter(|name| !objects.contains_key(*name)) {
        if let Some(fact) = importer.classification(name, &classification_scope) {
            classifications.insert(name.clone(), fact);
        }
    }
    let locomotor_source = importer
        .locate("DATA:GlobalData/Locomotor.xml", None)
        .ok_or_else(|| "Locomotor.xml is missing".to_string())?;
    let locomotor_names = importer
        .definitions
        .iter()
        .filter(|definition| definition.kind == "LocomotorTemplate")
        .map(|definition| definition.name.clone())
        .collect::<BTreeSet<_>>();
    let mut locomotors = BTreeMap::new();
    for name in &locomotor_names {
        let (node, lineage, pending) = importer.definition(
            "LocomotorTemplate",
            name,
            Some(&locomotor_source),
            None,
            &[],
        );
        let Some(node) = node else {
            importer.diagnostics.extend(pending);
            continue;
        };
        let mut attributes = locomotor_defaults.clone();
        for (key, value) in &node.attributes {
            if !key.starts_with('{') {
                attributes.insert(key.clone(), value.clone());
            }
        }
        let mut locomotor = Map::new();
        locomotor.insert(
            "id".into(),
            node.attr("id")
                .map_or(Value::Null, |id| Value::String(id.to_string())),
        );
        locomotor.insert(
            "source".into(),
            Value::String(
                lineage
                    .first()
                    .and_then(|lineage| lineage.rsplit_once('#'))
                    .map_or(String::new(), |(source, _)| source.to_string()),
            ),
        );
        locomotor.insert(
            "surfaces".into(),
            Value::Array(
                flags(&[], attributes.get("Surfaces").map_or("", String::as_str))
                    .into_iter()
                    .map(Value::String)
                    .collect(),
            ),
        );
        locomotor.insert(
            "attributes".into(),
            Value::Object(
                attributes
                    .into_iter()
                    .map(|(key, value)| (key, Value::String(value)))
                    .collect(),
            ),
        );
        let mut pending = pending;
        pending.sort();
        pending.dedup();
        locomotor.insert(
            "pending".into(),
            Value::Array(pending.into_iter().map(Value::String).collect()),
        );
        locomotors.insert(name.clone(), Value::Object(locomotor));
    }
    let terrain_source = importer
        .locate("DATA:Terrain/Terrain.xml", None)
        .ok_or_else(|| "Terrain.xml is missing".to_string())?;
    let terrain_document = importer.documents[&terrain_source]
        .as_ref()
        .ok_or_else(|| "Terrain.xml did not parse".to_string())?;
    let mut terrain_nodes = Vec::new();
    terrain_document.walk(&mut terrain_nodes);
    let mut terrain = BTreeMap::new();
    let terrain_label = importer.label(&terrain_source)?;
    for node in terrain_nodes
        .into_iter()
        .filter(|node| node.local() == "TerrainAsset")
    {
        let id = node
            .attr("id")
            .ok_or_else(|| "TerrainAsset without an id".to_string())?;
        let mut entry = Map::new();
        entry.insert("id".into(), Value::String(id.to_string()));
        entry.insert(
            "texture".into(),
            node.attr("Texture")
                .map_or(Value::Null, |value| Value::String(value.to_string())),
        );
        entry.insert(
            "bump_texture".into(),
            node.attr("BumpTexture")
                .map_or(Value::Null, |value| Value::String(value.to_string())),
        );
        entry.insert(
            "class".into(),
            node.attr("Class")
                .map_or(Value::Null, |value| Value::String(value.to_string())),
        );
        entry.insert(
            "wb_folder".into(),
            node.attr("WBFolder")
                .map_or(Value::Null, |value| Value::String(value.to_string())),
        );
        entry.insert("source".into(), Value::String(terrain_label.clone()));
        terrain.insert(id.to_lowercase(), Value::Object(entry));
    }
    let ai_source = importer
        .locate("DATA:SkirmishAI/AIData.xml", None)
        .ok_or_else(|| "AIData.xml is missing".to_string())?;
    let ai_document = importer.documents[&ai_source]
        .as_ref()
        .ok_or_else(|| "AIData.xml did not parse".to_string())?;
    let mut ai_nodes = Vec::new();
    ai_document.walk(&mut ai_nodes);
    let ai = ai_nodes
        .into_iter()
        .find(|node| node.local() == "AIData")
        .ok_or_else(|| "AIData.xml has no AIData element".to_string())?;
    let mut water_errors = Vec::new();
    let wade_value = ai
        .attr("WadeWaterDepth")
        .map(str::to_string)
        .or_else(|| ai_defaults.get("WadeWaterDepth").cloned())
        .ok_or_else(|| "schema default missing: WadeWaterDepth".to_string())?;
    let deep_value = ai
        .attr("DeepWaterDepth")
        .map(str::to_string)
        .or_else(|| ai_defaults.get("DeepWaterDepth").cloned())
        .ok_or_else(|| "schema default missing: DeepWaterDepth".to_string())?;
    let wade = importer.number(&wade_value, &mut water_errors, "WadeWaterDepth", &[]);
    let deep = importer.number(&deep_value, &mut water_errors, "DeepWaterDepth", &[]);
    if !water_errors.is_empty() {
        return Err(water_errors.join("; "));
    }
    let mut water = Map::new();
    water.insert("wade".into(), Value::from(wade));
    water.insert("deep".into(), Value::from(deep));
    water.insert("source".into(), Value::String(importer.label(&ai_source)?));
    let mut binding = Map::new();
    binding.insert("schema_version".into(), Value::from(1));
    binding.insert("extractor_version".into(), Value::String(VERSION.into()));
    binding.insert("revision".into(), Value::String(revision.to_string()));
    binding.insert(
        "inputs_sha256".into(),
        Value::Object(
            importer
                .inputs
                .iter()
                .map(|(path, digest)| (path.clone(), Value::String(digest.clone())))
                .collect(),
        ),
    );
    let fingerprint = sha256::hex(
        serde_json::to_string(&Value::Object(binding.clone()))
            .stringify()?
            .as_bytes(),
    );
    binding.insert("fingerprint".into(), Value::String(fingerprint));
    let dependencies = dependency_list(&importer.dependencies)?;
    let mut diagnostics = importer.diagnostics;
    diagnostics.sort();
    diagnostics.dedup();
    let mut catalog = Map::new();
    catalog.insert("binding".into(), Value::Object(binding));
    catalog.insert(
        "terrain".into(),
        Value::Object(terrain.into_iter().collect()),
    );
    catalog.insert(
        "objects".into(),
        Value::Object(objects.into_iter().collect()),
    );
    catalog.insert(
        "classifications".into(),
        Value::Object(classifications.into_iter().collect()),
    );
    catalog.insert(
        "locomotors".into(),
        Value::Object(locomotors.into_iter().collect()),
    );
    catalog.insert("water_depths".into(), Value::Object(water));
    catalog.insert(
        "global_membership".into(),
        Value::Array(membership.into_iter().map(Value::String).collect()),
    );
    catalog.insert("dependencies".into(), Value::Array(dependencies));
    catalog.insert(
        "diagnostics".into(),
        Value::Array(diagnostics.into_iter().map(Value::String).collect()),
    );
    Ok(Value::Object(catalog))
}

/// Python's compact, key-sorted dependency identity, including its ASCII escaping.
fn dependency_key(value: &Value) -> String {
    let field = |name: &str| {
        value[name]
            .as_str()
            .map_or_else(|| "null".to_string(), python_string)
    };
    format!(
        "{{\"kind\":{},\"resolved\":{},\"source\":{},\"target\":{}}}",
        field("kind"),
        field("resolved"),
        field("source"),
        field("target")
    )
}
fn python_string(text: &str) -> String {
    let mut out = String::from("\"");
    for character in text.chars() {
        match character {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\u{08}' => out.push_str("\\b"),
            '\u{0c}' => out.push_str("\\f"),
            character if (character as u32) < 0x20 || (character as u32) > 0x7e => {
                let code = character as u32;
                if code > 0xffff {
                    let offset = code - 0x10000;
                    let high = 0xd800 + (offset >> 10);
                    let low = 0xdc00 + (offset & 0x3ff);
                    out.push_str(&format!("\\u{high:04x}\\u{low:04x}"));
                } else {
                    out.push_str(&format!("\\u{code:04x}"));
                }
            }
            character => out.push(character),
        }
    }
    out.push('"');
    out
}
fn dependency_list(dependencies: &[Value]) -> Fallible<Vec<Value>> {
    let mut keyed = BTreeMap::new();
    for dependency in dependencies {
        keyed.insert(dependency_key(dependency), dependency.clone());
    }
    Ok(keyed.into_values().collect())
}

fn encode(catalog: &Value) -> Fallible<String> {
    Ok(format!(
        "{}\n",
        serde_json::to_string_pretty(catalog).stringify()?
    ))
}

fn main() {
    if let Err(error) = run() {
        eprintln!("EA catalog: {error}");
        std::process::exit(1);
    }
}

fn run() -> Fallible<()> {
    let mut root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let mut output = None;
    let mut check = false;
    let mut arguments = env::args().skip(1);
    while let Some(argument) = arguments.next() {
        match argument.as_str() {
            "--root" => {
                root = PathBuf::from(
                    arguments
                        .next()
                        .ok_or_else(|| "--root needs a value".to_string())?,
                );
            }
            "--output" => {
                output = Some(PathBuf::from(
                    arguments
                        .next()
                        .ok_or_else(|| "--output needs a value".to_string())?,
                ));
            }
            "--check" => check = true,
            "--test" => {
                return Err(
                    "importer unit tests run under cargo test (--example import-ea-catalog)".into(),
                );
            }
            "--help" | "-h" => {
                println!(
                    "Offline extraction of the pinned EA SDK resource metadata.\n\n  --root <dir>    project root (default: compile-time checkout)\n  --output <file> catalog path (default: <root>/templates/resources/ea-catalog.json)\n  --check         verify the checked-in index without writing"
                );
                return Ok(());
            }
            other => return Err(format!("unknown argument {other}")),
        }
    }
    let root = fs::canonicalize(&root).map_err(|error| format!("invalid project root: {error}"))?;
    let catalog = build(&root)?;
    let encoded = encode(&catalog)?;
    let output = output.unwrap_or_else(|| root.join("templates/resources/ea-catalog.json"));
    if check {
        if fs::read(&output).ok().as_deref() != Some(encoded.as_bytes()) {
            return Err(
                "EA catalog differs from its pinned offline inputs; regenerate and review".into(),
            );
        }
    } else {
        if let Some(parent) = output.parent() {
            fs::create_dir_all(parent).stringify()?;
        }
        let temporary = output.with_extension(format!("pending-{}", std::process::id()));
        fs::write(&temporary, encoded.as_bytes()).stringify()?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&temporary, fs::Permissions::from_mode(0o644)).stringify()?;
        }
        fs::rename(&temporary, &output).stringify()?;
    }
    println!(
        "EA catalog: {} materials, {} scoped objects, {} locomotors, {} input digests; {} diagnostics",
        catalog["terrain"].as_object().map_or(0, Map::len),
        catalog["objects"].as_object().map_or(0, Map::len),
        catalog["locomotors"].as_object().map_or(0, Map::len),
        catalog["binding"]["inputs_sha256"]
            .as_object()
            .map_or(0, Map::len),
        catalog["diagnostics"].as_array().map_or(0, Vec::len),
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kindof_deltas_preserve_inherited_membership() {
        assert_eq!(
            flags(
                &["STRUCTURE".into(), "IMMOBILE".into()],
                "+SELECTABLE -IMMOBILE"
            ),
            ["SELECTABLE", "STRUCTURE"]
        );
        assert_eq!(flags(&["STRUCTURE".into()], "INFANTRY"), ["INFANTRY"]);
    }
    fn node(xml: &str) -> Node {
        XmlParser::parse_root(xml).unwrap()
    }
    #[test]
    fn keyed_states_and_modules_merge_without_combining_mobility_states() {
        let base = node(
            "<GameObject KindOf=\"STRUCTURE\"><LocomotorSet Condition=\"NORMAL\" Locomotor=\"Ground\" Speed=\"100%\"/><LocomotorSet Condition=\"SECONDARY\" Locomotor=\"Water\"/></GameObject>",
        );
        let update = node(
            "<GameObject KindOf=\"+SELECTABLE -STRUCTURE\"><LocomotorSet Condition=\"NORMAL\" Speed=\"50%\"/></GameObject>",
        );
        let actual = merge(&base, &update);
        assert_eq!(actual.attr("KindOf"), Some("SELECTABLE"));
        let states = actual
            .children_named("LocomotorSet")
            .into_iter()
            .map(|state| {
                (
                    state.attr("Condition"),
                    state.attr("Locomotor"),
                    state.attr("Speed"),
                )
            })
            .collect::<Vec<_>>();
        assert_eq!(
            states,
            [
                (Some("NORMAL"), Some("Ground"), Some("50%")),
                (Some("SECONDARY"), Some("Water"), None)
            ]
        );
    }
    #[test]
    fn two_normal_surface_locomotors_do_not_overwrite_one_another() {
        let base = node("<GameObject/>");
        let update = node(
            "<GameObject><LocomotorSet Condition=\"NORMAL\" Locomotor=\"Land\"/><LocomotorSet Condition=\"NORMAL\" Locomotor=\"Water\"/></GameObject>",
        );
        let actual = merge(&base, &update);
        let locomotors = actual
            .children_named("LocomotorSet")
            .into_iter()
            .map(|state| state.attr("Locomotor"))
            .collect::<Vec<_>>();
        assert_eq!(locomotors, [Some("Land"), Some("Water")]);
    }
    fn stub_importer(defines: &[(&str, &str)]) -> Importer {
        Importer {
            root: PathBuf::new(),
            mirror: PathBuf::new(),
            files: BTreeMap::new(),
            inputs: BTreeMap::new(),
            manifest: BTreeMap::new(),
            documents: BTreeMap::new(),
            dependencies: Vec::new(),
            edges: BTreeMap::new(),
            definitions: Vec::new(),
            defines: defines
                .iter()
                .map(|(name, value)| (name.to_string(), value.to_string()))
                .collect(),
            define_conflicts: BTreeSet::new(),
            diagnostics: Vec::new(),
            resolved: BTreeMap::new(),
            vector_defaults: BTreeMap::new(),
            object_defaults: BTreeMap::new(),
        }
    }
    #[test]
    fn bitfield_macro_concatenation_and_inherited_deltas_are_resolved_first() {
        let importer = stub_importer(&[
            ("BASE", "STRUCTURE IMMOBILE"),
            ("EXTENDED", "=$BASE+' SELECTABLE'"),
        ]);
        let mut pending = Vec::new();
        let value = importer.bitfield("=$EXTENDED", &mut pending, "fixture.KindOf");
        assert_eq!(value.as_deref(), Some("STRUCTURE IMMOBILE SELECTABLE"));
        let mut base = node("<GameObject/>");
        base.attributes.insert("KindOf".into(), value.unwrap());
        let actual = merge(
            &base,
            &node("<GameObject KindOf=\"+CAN_ATTACK -IMMOBILE\"/>"),
        );
        assert_eq!(
            actual.attr("KindOf"),
            Some("CAN_ATTACK SELECTABLE STRUCTURE")
        );
        assert_eq!(pending, Vec::<String>::new());
        let cyclic = stub_importer(&[("A", "=$B"), ("B", "=$A")]);
        assert!(
            cyclic
                .bitfield("=$A", &mut pending, "cycle.KindOf")
                .is_none()
        );
        assert!(pending.last().unwrap().contains("cyclic macro"));
        assert!(
            importer
                .bitfield("=$MISSING", &mut pending, "missing.KindOf")
                .is_none()
        );
        assert!(
            pending
                .last()
                .unwrap()
                .contains("missing or ambiguous macro")
        );
    }
    struct Temp(PathBuf);
    impl Temp {
        fn new() -> Self {
            let path = env::temp_dir().join(format!(
                "ra3-ea-catalog-{}-{:?}",
                std::process::id(),
                std::thread::current().id()
            ));
            let _ = fs::remove_dir_all(&path);
            fs::create_dir_all(&path).unwrap();
            Self(path)
        }
    }
    impl Drop for Temp {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }
    #[test]
    fn explicit_include_scope_selects_the_parent_and_expands_geometry_fragment() {
        let temp = Temp::new();
        let root = &temp.0;
        let files = [
            (
                "Xml/Base/Parent.xml",
                "<AssetDeclaration><GameObject id=\"Parent\" KindOf=\"STRUCTURE\"><Geometry><Shape MajorRadius=\"4\"/></Geometry></GameObject></AssetDeclaration>",
            ),
            (
                "Xml/Other/Parent.xml",
                "<AssetDeclaration><GameObject id=\"Parent\"><Geometry><Shape MajorRadius=\"99\"/></Geometry></GameObject></AssetDeclaration>",
            ),
            (
                "Xml/Includes/Restriction.xml",
                "<StructurePlacementBehavior><RestrictedAreaShape MajorRadius=\"30\" MinorRadius=\"45\"><Offset x=\"90\"/></RestrictedAreaShape></StructurePlacementBehavior>",
            ),
            (
                "Xml/Unit.xml",
                "<AssetDeclaration xmlns:xi=\"http://www.w3.org/2001/XInclude\"><Includes><Include type=\"instance\" source=\"DATA:base/parent.xml\"/></Includes><GameObject id=\"Unit\" inheritFrom=\"Parent\" KindOf=\"+SELECTABLE\"><Behaviors><xi:include href=\"DATA:Includes/Restriction.xml\"/></Behaviors></GameObject></AssetDeclaration>",
            ),
        ];
        let mut rows = Vec::new();
        for (name, text) in files {
            let target = root.join("data/ea-modding/red-alert-3").join(name);
            fs::create_dir_all(target.parent().unwrap()).unwrap();
            fs::write(&target, text).unwrap();
            rows.push(format!(
                "{{\"path\":\"{name}\",\"bytes\":{},\"sha256\":\"{}\"}}",
                text.len(),
                sha256::hex(text.as_bytes())
            ));
        }
        fs::create_dir_all(root.join("data/ea-modding")).unwrap();
        fs::write(
            root.join("data/ea-modding/manifest.jsonl"),
            rows.join("\n") + "\n",
        )
        .unwrap();
        let mut importer = Importer::new(root).unwrap();
        let source = importer.locate("DATA:Unit.xml", None).unwrap();
        importer.load(&source).unwrap();
        let other = importer.locate("DATA:Other/Parent.xml", None).unwrap();
        importer.load(&other).unwrap();
        let (definition, lineage, pending) =
            importer.definition("GameObject", "Unit", Some(&source), None, &[]);
        let definition = definition.unwrap();
        assert_eq!(pending, Vec::<String>::new());
        assert_eq!(definition.attr("KindOf"), Some("SELECTABLE STRUCTURE"));
        assert!(lineage.contains(&"Xml/Base/Parent.xml#Parent".to_string()));
        let shape = definition
            .child("Geometry")
            .unwrap()
            .child("Shape")
            .unwrap();
        assert_eq!(shape.attr("MajorRadius"), Some("4"));
        let mut nodes = Vec::new();
        definition.walk(&mut nodes);
        let restricted = nodes
            .into_iter()
            .find(|node| node.local() == "RestrictedAreaShape")
            .unwrap();
        assert_eq!(restricted.child("Offset").unwrap().attr("x"), Some("90"));
        // Loaded duplicate IDs are unavailable without an unambiguous scope.
        assert!(
            importer
                .choose("GameObject", "Parent", None, None)
                .is_none()
        );
        let scope = importer.documents.keys().cloned().collect::<BTreeSet<_>>();
        assert!(importer.classification("Parent", &scope).is_none());
        // Agreeing classification is exportable without inventing a
        // unique source or copying either candidate's exact geometry.
        let index = importer
            .definitions
            .iter()
            .position(|definition| definition.kind == "GameObject" && definition.path == other)
            .unwrap();
        importer.definitions[index]
            .node
            .attributes
            .insert("KindOf".into(), "STRUCTURE".into());
        importer.resolved.clear();
        let fact = importer.classification("Parent", &scope).unwrap();
        assert_eq!(fact["kind_of"][0], "STRUCTURE");
        assert_eq!(fact["equivalent_sources"].as_array().unwrap().len(), 2);
        assert!(fact.get("source").is_none());
        assert!(fact.get("geometry").is_none());
        let restriction = importer
            .locate("DATA:Includes/Restriction.xml", None)
            .unwrap();
        fs::write(&restriction, "local changed bytes").unwrap();
        let error = importer.read(&restriction).unwrap_err();
        assert!(error.contains("pinned mirror manifest"));
    }
}
