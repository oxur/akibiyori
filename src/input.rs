//! Bounded, source-order-preserving contract input readers.

use std::collections::HashSet;
use std::io::Read;
use std::path::Path;

use serde::de::{self, Deserialize, DeserializeSeed, MapAccess, SeqAccess, Visitor};
use serde_json::{Map, Value, value::RawValue};
use yaml_rust2::parser::{Event, Parser};
use yaml_rust2::scanner::{Marker, TScalarStyle};

use crate::Error;

const MAX_INPUT_BYTES: usize = 16 * 1024 * 1024;
const MAX_DEPTH: usize = 64;
const MAX_NODES: usize = 1_000_000;
const MAX_SCALAR_BYTES: usize = 4 * 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum InputFormat {
    Json,
    Yaml,
}

#[derive(Clone, Copy)]
struct Limits {
    bytes: usize,
    depth: usize,
    nodes: usize,
    scalar_bytes: usize,
}

const DEFAULT_LIMITS: Limits = Limits {
    bytes: MAX_INPUT_BYTES,
    depth: MAX_DEPTH,
    nodes: MAX_NODES,
    scalar_bytes: MAX_SCALAR_BYTES,
};

pub(crate) fn read_contract(path: &Path) -> Result<Value, Error> {
    let file = std::fs::File::open(path).map_err(|error| Error::read(path, error))?;
    let mut bytes = Vec::new();
    file.take((DEFAULT_LIMITS.bytes + 1) as u64)
        .read_to_end(&mut bytes)
        .map_err(|error| Error::read(path, error))?;
    if bytes.len() > DEFAULT_LIMITS.bytes {
        return Err(input_error(
            path,
            format!("input exceeds {} byte limit", DEFAULT_LIMITS.bytes),
        ));
    }
    match input_format(path)? {
        InputFormat::Json => parse_json(path, &bytes, DEFAULT_LIMITS),
        InputFormat::Yaml => parse_yaml(path, &bytes, DEFAULT_LIMITS),
    }
}

pub(crate) fn input_format(path: &Path) -> Result<InputFormat, Error> {
    let extension = path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or_default();
    if extension.eq_ignore_ascii_case("json") {
        Ok(InputFormat::Json)
    } else if extension.eq_ignore_ascii_case("yaml") || extension.eq_ignore_ascii_case("yml") {
        Ok(InputFormat::Yaml)
    } else {
        Err(input_error(
            path,
            format!("unsupported contract extension {extension:?}; expected .json, .yaml, or .yml"),
        ))
    }
}

fn parse_json(path: &Path, bytes: &[u8], limits: Limits) -> Result<Value, Error> {
    if bytes.len() > limits.bytes {
        return Err(input_error(
            path,
            format!("input exceeds {} byte limit", limits.bytes),
        ));
    }
    let mut deserializer = serde_json::Deserializer::from_slice(bytes);
    let raw = <&RawValue>::deserialize(&mut deserializer).map_err(|source| Error::Json {
        path: path.to_path_buf(),
        source,
    })?;
    deserializer.end().map_err(|source| Error::Json {
        path: path.to_path_buf(),
        source,
    })?;
    parse_json_value(path, raw.get(), 0, &mut JsonBudget { nodes: 0 }, limits)
        .map_err(|error| input_error(path, format!("invalid JSON: {error}")))
}

struct JsonBudget {
    nodes: usize,
}

impl JsonBudget {
    fn count(&mut self, count: usize, limits: Limits) -> Result<(), serde_json::Error> {
        self.nodes = self
            .nodes
            .checked_add(count)
            .ok_or_else(|| json_custom("JSON node count overflow"))?;
        if self.nodes > limits.nodes {
            return Err(json_custom(format!(
                "value exceeds {} node limit",
                limits.nodes
            )));
        }
        Ok(())
    }
}

fn parse_json_value(
    path: &Path,
    raw: &str,
    depth: usize,
    budget: &mut JsonBudget,
    limits: Limits,
) -> Result<Value, serde_json::Error> {
    if depth > limits.depth {
        return Err(json_custom(format!(
            "nesting exceeds {} level limit",
            limits.depth
        )));
    }
    match raw.trim_start().as_bytes().first() {
        Some(b'{') => {
            let mut deserializer = serde_json::Deserializer::from_str(raw);
            let object = JsonObjectSeed {
                path,
                depth,
                budget,
                limits,
            }
            .deserialize(&mut deserializer)?;
            deserializer.end()?;
            Ok(Value::Object(object))
        }
        Some(b'[') => {
            let mut deserializer = serde_json::Deserializer::from_str(raw);
            let array = JsonArraySeed {
                path,
                depth,
                budget,
                limits,
            }
            .deserialize(&mut deserializer)?;
            deserializer.end()?;
            Ok(Value::Array(array))
        }
        _ => {
            if raw.len() > limits.scalar_bytes && !raw.trim_start().starts_with('"') {
                return Err(json_custom(format!(
                    "numeric scalar exceeds {} byte limit",
                    limits.scalar_bytes
                )));
            }
            budget.count(1, limits)?;
            let value: Value = serde_json::from_str(raw)?;
            if value
                .as_str()
                .is_some_and(|value| value.len() > limits.scalar_bytes)
            {
                return Err(json_custom(format!(
                    "decoded scalar exceeds {} byte limit",
                    limits.scalar_bytes
                )));
            }
            Ok(value)
        }
    }
}

struct JsonObjectSeed<'a> {
    path: &'a Path,
    depth: usize,
    budget: &'a mut JsonBudget,
    limits: Limits,
}

impl<'de> DeserializeSeed<'de> for JsonObjectSeed<'_> {
    type Value = Map<String, Value>;
    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        if self.depth >= self.limits.depth {
            return Err(de::Error::custom(format!(
                "nesting exceeds {} level limit",
                self.limits.depth
            )));
        }
        self.budget
            .count(1, self.limits)
            .map_err(de::Error::custom)?;
        deserializer.deserialize_map(JsonObjectVisitor {
            path: self.path,
            depth: self.depth + 1,
            budget: self.budget,
            limits: self.limits,
        })
    }
}

struct JsonObjectVisitor<'a> {
    path: &'a Path,
    depth: usize,
    budget: &'a mut JsonBudget,
    limits: Limits,
}

impl<'de> Visitor<'de> for JsonObjectVisitor<'_> {
    type Value = Map<String, Value>;
    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("a JSON object with unique keys")
    }
    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
        let mut values = Map::new();
        let mut keys = HashSet::new();
        while let Some(key) = map.next_key::<String>()? {
            if key.len() > self.limits.scalar_bytes {
                return Err(de::Error::custom(format!(
                    "decoded key exceeds {} byte limit",
                    self.limits.scalar_bytes
                )));
            }
            if !keys.insert(key.clone()) {
                return Err(de::Error::custom(format!("duplicate object key {key:?}")));
            }
            self.budget
                .count(1, self.limits)
                .map_err(de::Error::custom)?;
            let raw = map.next_value::<&RawValue>()?;
            let value =
                parse_json_value(self.path, raw.get(), self.depth, self.budget, self.limits)
                    .map_err(de::Error::custom)?;
            values.insert(key, value);
        }
        Ok(values)
    }
}

struct JsonArraySeed<'a> {
    path: &'a Path,
    depth: usize,
    budget: &'a mut JsonBudget,
    limits: Limits,
}

impl<'de> DeserializeSeed<'de> for JsonArraySeed<'_> {
    type Value = Vec<Value>;
    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        if self.depth >= self.limits.depth {
            return Err(de::Error::custom(format!(
                "nesting exceeds {} level limit",
                self.limits.depth
            )));
        }
        self.budget
            .count(1, self.limits)
            .map_err(de::Error::custom)?;
        deserializer.deserialize_seq(JsonArrayVisitor {
            path: self.path,
            depth: self.depth + 1,
            budget: self.budget,
            limits: self.limits,
        })
    }
}

struct JsonArrayVisitor<'a> {
    path: &'a Path,
    depth: usize,
    budget: &'a mut JsonBudget,
    limits: Limits,
}
impl<'de> Visitor<'de> for JsonArrayVisitor<'_> {
    type Value = Vec<Value>;
    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("a JSON array")
    }
    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
        let mut values = Vec::new();
        while let Some(raw) = seq.next_element::<&RawValue>()? {
            values.push(
                parse_json_value(self.path, raw.get(), self.depth, self.budget, self.limits)
                    .map_err(de::Error::custom)?,
            );
        }
        Ok(values)
    }
}

fn json_custom(message: impl std::fmt::Display) -> serde_json::Error {
    <serde_json::Error as de::Error>::custom(message)
}

enum Frame {
    Sequence(Vec<Value>),
    Mapping {
        values: Map<String, Value>,
        key: Option<String>,
    },
}

fn parse_yaml(path: &Path, bytes: &[u8], limits: Limits) -> Result<Value, Error> {
    if bytes.len() > limits.bytes {
        return Err(input_error(
            path,
            format!("input exceeds {} byte limit", limits.bytes),
        ));
    }
    let source = std::str::from_utf8(bytes)
        .map_err(|error| input_error(path, format!("YAML must be UTF-8: {error}")))?;
    if source
        .lines()
        .any(|line| line.starts_with("%YAML") || line.starts_with("%TAG"))
    {
        return Err(input_error(path, "YAML directives are unsupported"));
    }
    let mut parser = Parser::new_from_str(source);
    let mut frames: Vec<Frame> = Vec::new();
    let mut root = None;
    let mut nodes = 0usize;
    let mut documents = 0usize;
    loop {
        let (event, marker) = parser
            .next_token()
            .map_err(|error| yaml_error(path, *error.marker(), error.info()))?;
        match event {
            Event::StreamStart | Event::Nothing => {}
            Event::DocumentStart => {
                documents += 1;
                if documents != 1 || root.is_some() {
                    return Err(yaml_error(
                        path,
                        marker,
                        "multiple YAML documents are unsupported",
                    ));
                }
            }
            Event::DocumentEnd => {
                if !frames.is_empty() || root.is_none() {
                    return Err(yaml_error(
                        path,
                        marker,
                        "YAML document must contain one complete root value",
                    ));
                }
            }
            Event::MappingStart(anchor, tag) => {
                reject_yaml_features(path, marker, anchor, tag.is_some())?;
                push_collection(
                    path,
                    marker,
                    &mut frames,
                    &mut nodes,
                    limits,
                    Frame::Mapping {
                        values: Map::new(),
                        key: None,
                    },
                )?;
            }
            Event::SequenceStart(anchor, tag) => {
                reject_yaml_features(path, marker, anchor, tag.is_some())?;
                push_collection(
                    path,
                    marker,
                    &mut frames,
                    &mut nodes,
                    limits,
                    Frame::Sequence(Vec::new()),
                )?;
            }
            Event::MappingEnd => {
                let Some(Frame::Mapping { values, key: None }) = frames.pop() else {
                    return Err(yaml_error(path, marker, "malformed YAML mapping"));
                };
                attach(path, marker, Value::Object(values), &mut frames, &mut root)?;
            }
            Event::SequenceEnd => {
                let Some(Frame::Sequence(values)) = frames.pop() else {
                    return Err(yaml_error(path, marker, "malformed YAML sequence"));
                };
                attach(path, marker, Value::Array(values), &mut frames, &mut root)?;
            }
            Event::Scalar(value, style, anchor, tag) => {
                reject_yaml_features(path, marker, anchor, tag.is_some())?;
                nodes += 1;
                if nodes > limits.nodes {
                    return Err(yaml_error(
                        path,
                        marker,
                        format!("value exceeds {} node limit", limits.nodes),
                    ));
                }
                if value.len() > limits.scalar_bytes {
                    return Err(yaml_error(
                        path,
                        marker,
                        format!("decoded scalar exceeds {} byte limit", limits.scalar_bytes),
                    ));
                }
                if let Some(Frame::Mapping { key: None, .. }) = frames.last() {
                    let key = yaml_key(path, marker, &value, style)?;
                    let Some(Frame::Mapping { key: slot, .. }) = frames.last_mut() else {
                        unreachable!()
                    };
                    *slot = Some(key);
                } else {
                    let scalar = yaml_scalar(path, marker, &value, style)?;
                    attach(path, marker, scalar, &mut frames, &mut root)?;
                }
            }
            Event::Alias(_) => {
                return Err(yaml_error(path, marker, "YAML aliases are unsupported"));
            }
            Event::StreamEnd => break,
        }
        if frames.len() > limits.depth {
            return Err(yaml_error(
                path,
                marker,
                format!("nesting exceeds {} level limit", limits.depth),
            ));
        }
    }
    if documents != 1 || !frames.is_empty() {
        return Err(input_error(
            path,
            "YAML must contain exactly one complete document",
        ));
    }
    root.ok_or_else(|| input_error(path, "YAML document is empty"))
}

fn push_collection(
    path: &Path,
    marker: Marker,
    frames: &mut Vec<Frame>,
    nodes: &mut usize,
    limits: Limits,
    frame: Frame,
) -> Result<(), Error> {
    *nodes += 1;
    if *nodes > limits.nodes {
        return Err(yaml_error(
            path,
            marker,
            format!("value exceeds {} node limit", limits.nodes),
        ));
    }
    frames.push(frame);
    Ok(())
}

fn attach(
    path: &Path,
    marker: Marker,
    value: Value,
    frames: &mut [Frame],
    root: &mut Option<Value>,
) -> Result<(), Error> {
    if let Some(frame) = frames.last_mut() {
        match frame {
            Frame::Sequence(values) => values.push(value),
            Frame::Mapping { values, key } => {
                let Some(key) = key.take() else {
                    return Err(yaml_error(path, marker, "mapping value has no string key"));
                };
                if values.contains_key(&key) {
                    return Err(yaml_error(
                        path,
                        marker,
                        format!("duplicate mapping key {key:?}"),
                    ));
                }
                values.insert(key, value);
            }
        }
    } else if root.replace(value).is_some() {
        return Err(yaml_error(
            path,
            marker,
            "multiple root values are unsupported",
        ));
    }
    Ok(())
}

fn reject_yaml_features(
    path: &Path,
    marker: Marker,
    anchor: usize,
    has_tag: bool,
) -> Result<(), Error> {
    if anchor != 0 {
        return Err(yaml_error(path, marker, "YAML anchors are unsupported"));
    }
    if has_tag {
        return Err(yaml_error(
            path,
            marker,
            "YAML explicit tags are unsupported",
        ));
    }
    Ok(())
}

fn yaml_key(
    path: &Path,
    marker: Marker,
    value: &str,
    style: TScalarStyle,
) -> Result<String, Error> {
    if value == "<<" {
        return Err(yaml_error(path, marker, "YAML merge keys are unsupported"));
    }
    if style == TScalarStyle::Plain
        && !matches!(yaml_scalar(path, marker, value, style)?, Value::String(_))
    {
        return Err(yaml_error(
            path,
            marker,
            "YAML mapping keys must be strings",
        ));
    }
    Ok(value.to_owned())
}

fn yaml_scalar(
    path: &Path,
    marker: Marker,
    value: &str,
    style: TScalarStyle,
) -> Result<Value, Error> {
    if style != TScalarStyle::Plain {
        return Ok(Value::String(value.to_owned()));
    }
    if value.is_empty() {
        return Ok(Value::Null);
    }
    match value {
        "null" | "Null" | "NULL" | "~" => return Ok(Value::Null),
        "true" | "True" | "TRUE" => return Ok(Value::Bool(true)),
        "false" | "False" | "FALSE" => return Ok(Value::Bool(false)),
        _ => {}
    }
    if looks_yaml_number(value) {
        let parsed: Value = serde_json::from_str(value).map_err(|error| {
            yaml_error(
                path,
                marker,
                format!("unsupported or invalid plain YAML number {value:?}: {error}"),
            )
        })?;
        if let Value::Number(number) = parsed {
            return Ok(Value::Number(number));
        }
        return Err(yaml_error(
            path,
            marker,
            format!("invalid plain YAML number {value:?}"),
        ));
    }
    Ok(Value::String(value.to_owned()))
}

fn looks_yaml_number(value: &str) -> bool {
    if let Some(digits) = value.strip_prefix("0x") {
        return !digits.is_empty() && digits.bytes().all(|byte| byte.is_ascii_hexdigit());
    }
    if let Some(digits) = value.strip_prefix("0o") {
        return !digits.is_empty() && digits.bytes().all(|byte| matches!(byte, b'0'..=b'7'));
    }
    let unsigned = value.strip_prefix(['+', '-']).unwrap_or(value);
    if unsigned.is_empty() {
        return false;
    }
    if matches!(unsigned, ".nan" | ".NaN" | ".NAN") {
        return value == unsigned;
    }
    if matches!(unsigned, ".inf" | ".Inf" | ".INF") {
        return true;
    }
    if unsigned.bytes().all(|byte| byte.is_ascii_digit()) {
        return true;
    }
    let mut parts = unsigned.split(['e', 'E']);
    let mantissa = parts.next().unwrap_or_default();
    let exponent = parts.next();
    if parts.next().is_some() {
        return false;
    }
    if let Some(exponent) = exponent {
        let exponent = exponent.strip_prefix(['+', '-']).unwrap_or(exponent);
        if exponent.is_empty() || !exponent.bytes().all(|byte| byte.is_ascii_digit()) {
            return false;
        }
    }
    let mut decimals = mantissa.split('.');
    let before = decimals.next().unwrap_or_default();
    let after = decimals.next();
    if decimals.next().is_some() {
        return false;
    }
    match after {
        None => {
            !before.is_empty()
                && before.bytes().all(|byte| byte.is_ascii_digit())
                && exponent.is_some()
        }
        Some(after) => {
            let before_ok = before.bytes().all(|byte| byte.is_ascii_digit());
            let after_ok = after.bytes().all(|byte| byte.is_ascii_digit());
            before_ok && after_ok && (!before.is_empty() || !after.is_empty())
        }
    }
}

fn input_error(path: &Path, message: impl std::fmt::Display) -> Error {
    Error::Invalid(format!("contract input {}: {message}", path.display()))
}

fn yaml_error(path: &Path, marker: Marker, message: impl std::fmt::Display) -> Error {
    Error::Invalid(format!(
        "contract input {}:{}:{}: {message}",
        path.display(),
        marker.line(),
        marker.col()
    ))
}

#[cfg(test)]
mod tests {
    use super::{Limits, parse_json, parse_yaml};
    use serde_json::Value;
    use std::path::Path;

    #[test]
    fn ordered_yaml_retains_strings_and_exact_large_numbers() {
        let value = parse_yaml(
            Path::new("x.yaml"),
            br#"first: 123456789012345678901234567890.123456789
timestamp: 2025-11-05T12:00:00Z
flag: yes
version: 3.1.1
folded: >-
  hello
  world
"#,
            Limits {
                bytes: 1024,
                depth: 8,
                nodes: 20,
                scalar_bytes: 100,
            },
        )
        .expect("valid YAML subset");
        let keys: Vec<_> = value
            .as_object()
            .unwrap()
            .keys()
            .map(String::as_str)
            .collect();
        assert_eq!(keys, ["first", "timestamp", "flag", "version", "folded"]);
        assert_eq!(
            value["first"].as_number().unwrap().to_string(),
            "123456789012345678901234567890.123456789"
        );
        assert_eq!(value["timestamp"], "2025-11-05T12:00:00Z");
        assert_eq!(value["flag"], "yes");
        assert_eq!(value["version"], "3.1.1");
        assert_eq!(value["folded"], "hello world");
    }

    #[test]
    fn ordered_json_rejects_duplicate_keys_and_preserves_number_value() {
        let duplicate = parse_json(
            Path::new("x.json"),
            br#"{"a":1,"\u0061":2}"#,
            Limits {
                bytes: 128,
                depth: 8,
                nodes: 20,
                scalar_bytes: 32,
            },
        )
        .unwrap_err();
        assert!(duplicate.to_string().contains("duplicate object key"));
        let value = parse_json(
            Path::new("x.json"),
            br#"{"z":-0,"a":1e+09}"#,
            Limits {
                bytes: 128,
                depth: 8,
                nodes: 20,
                scalar_bytes: 32,
            },
        )
        .expect("valid JSON");
        assert_eq!(
            value
                .as_object()
                .unwrap()
                .keys()
                .map(String::as_str)
                .collect::<Vec<_>>(),
            ["z", "a"]
        );
        assert_eq!(value["z"].as_number().unwrap().to_string(), "-0");
        assert_eq!(value["a"].as_number().unwrap().to_string(), "1e+09");
    }

    #[test]
    fn yaml_rejects_duplicate_nonstring_and_tagged_values() {
        let limits = Limits {
            bytes: 128,
            depth: 8,
            nodes: 20,
            scalar_bytes: 32,
        };
        for source in [
            "a: 1\na: 2\n",
            "1: value\n",
            "a: !custom value\n",
            "a: &x value\n",
            "a: *x\n",
            "---\na: 1\n---\nb: 2\n",
        ] {
            assert!(
                parse_yaml(Path::new("x.yaml"), source.as_bytes(), limits).is_err(),
                "{source}"
            );
        }
    }

    #[test]
    fn yaml_plain_scalars_follow_the_declared_core_numeric_subset() {
        let limits = Limits {
            bytes: 1024,
            depth: 8,
            nodes: 40,
            scalar_bytes: 128,
        };
        let strings = parse_yaml(Path::new("x.yaml"), b"empty:\nunderscore: 1_000\nbinary: 0b10\nbad_octal: 0o9\nupper_hex: 0XFF\nsigned_hex: -0xFF\nsigned_octal: +0o77\nexponent: 1e\nrange: 1-2\nversion: 3.1.1\ntimestamp: 2025-11-05T12:00:00Z\n", limits).expect("non-core number spellings remain strings");
        assert_eq!(strings["empty"], Value::Null);
        for key in [
            "underscore",
            "binary",
            "bad_octal",
            "upper_hex",
            "signed_hex",
            "signed_octal",
            "exponent",
            "range",
            "version",
            "timestamp",
        ] {
            assert!(strings[key].is_string(), "{key}");
        }
        for number in ["0xFF", "0o77", "+5", ".5", "5.", ".nan", "-.Inf", "01"] {
            let source = format!("value: {number}\n");
            assert!(
                parse_yaml(Path::new("x.yaml"), source.as_bytes(), limits).is_err(),
                "{number}"
            );
        }
    }

    #[test]
    fn yaml_enforces_each_injected_bound() {
        assert!(
            parse_yaml(
                Path::new("x.yaml"),
                b"a: [1, 2]\n",
                Limits {
                    bytes: 4,
                    depth: 8,
                    nodes: 20,
                    scalar_bytes: 8
                }
            )
            .is_err()
        );
        assert!(
            parse_yaml(
                Path::new("x.yaml"),
                b"a: [[1]]\n",
                Limits {
                    bytes: 128,
                    depth: 1,
                    nodes: 20,
                    scalar_bytes: 8
                }
            )
            .is_err()
        );
        assert!(
            parse_yaml(
                Path::new("x.yaml"),
                b"a: [1, 2]\n",
                Limits {
                    bytes: 128,
                    depth: 8,
                    nodes: 3,
                    scalar_bytes: 8
                }
            )
            .is_err()
        );
        assert!(
            parse_yaml(
                Path::new("x.yaml"),
                b"a: long\n",
                Limits {
                    bytes: 128,
                    depth: 8,
                    nodes: 20,
                    scalar_bytes: 3
                }
            )
            .is_err()
        );
    }

    #[test]
    fn ordered_json_preserves_literal_reserved_key_and_exact_number() {
        let source = br#"{"opaque":{"$serde_json::private::Number":"123"},"precise":123456789012345678901234567890.123456789}"#;
        let value = parse_json(
            Path::new("x.json"),
            source,
            Limits {
                bytes: 1024,
                depth: 8,
                nodes: 20,
                scalar_bytes: 128,
            },
        )
        .expect("valid JSON");
        assert_eq!(value["opaque"]["$serde_json::private::Number"], "123");
        assert_eq!(
            value["precise"].as_number().unwrap().to_string(),
            "123456789012345678901234567890.123456789"
        );
    }
}
