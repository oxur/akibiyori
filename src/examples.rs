use std::collections::{BTreeMap, BTreeSet};
use std::io;

use serde_json::{Map, Value};

use crate::Error;
use crate::openapi::Contract;
use crate::schema_reading::{SchemaReadStop, SchemaReading, read_schema};

#[derive(Debug, Clone)]
pub(crate) struct ExampleVariant {
    pub label: String,
    pub note: String,
    pub value: Option<Value>,
    /// External example URL, which is displayed but never fetched.
    pub unavailable: Option<String>,
    pub unavailable_pointer: Option<String>,
    pub provided: bool,
}

#[derive(Debug, Clone, Eq, PartialEq)]
struct SampleNote {
    source_pointer: String,
    reason: String,
}

#[derive(Debug, Clone)]
enum SampleOutcome {
    Value {
        value: Value,
        notes: Vec<SampleNote>,
    },
    Unavailable {
        reason: String,
        source_pointer: String,
    },
}

enum FieldOutcome {
    Included(SampleOutcome),
    Omitted(SampleNote),
}

struct ObjectProperty {
    schema: Value,
    source_pointer: String,
    scope: Vec<String>,
}

#[derive(Default)]
struct ObjectComposition {
    properties: Vec<(String, ObjectProperty)>,
    property_names: BTreeSet<String>,
    required: BTreeSet<String>,
    required_pointers: BTreeMap<String, String>,
    saw_object_shape: bool,
}

#[derive(Clone, Copy)]
enum SchemaUse {
    Request,
    Response,
}

/// Maximum number of entries synthesized for one derived array.
const MAX_DERIVED_ARRAY_ITEMS: u64 = 16;
/// Maximum compact UTF-8 JSON bytes emitted for one derived array.
const MAX_DERIVED_ARRAY_JSON_BYTES: usize = 65_536;

#[derive(Clone)]
struct ArrayBound {
    value: u64,
    pointer: String,
}

#[derive(Default)]
struct DerivedArrayBounds<'a> {
    minimum: Option<ArrayBound>,
    maximum: Option<ArrayBound>,
    unique: bool,
    items: Option<(&'a Value, String)>,
    item_count: usize,
}

/// Counts serde_json's compact output without retaining a serialized buffer.
struct BoundedJsonCounter {
    bytes: usize,
    limit: usize,
    exceeded: bool,
}

impl io::Write for BoundedJsonCounter {
    fn write(&mut self, buffer: &[u8]) -> io::Result<usize> {
        let Some(next) = self.bytes.checked_add(buffer.len()) else {
            self.exceeded = true;
            return Err(io::Error::other("JSON byte count overflow"));
        };
        if next > self.limit {
            self.exceeded = true;
            return Err(io::Error::other("derived array JSON byte limit exceeded"));
        }
        self.bytes = next;
        Ok(buffer.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

/// Returns whether a media label names a concrete JSON representation.
///
/// This is intentionally a narrow reader policy, not a complete MIME parser.
pub(crate) fn is_json_media_type(media_type: &str) -> bool {
    if media_type
        .bytes()
        .any(|byte| byte.is_ascii_control() && byte != b'\t')
    {
        return false;
    }

    let essence = media_type
        .split_once(';')
        .map_or(media_type, |(essence, _)| essence)
        .trim_matches([' ', '\t']);
    let Some((type_name, subtype)) = essence.split_once('/') else {
        return false;
    };

    if type_name.is_empty()
        || subtype.is_empty()
        || subtype.contains('/')
        || essence.contains('*')
        || essence.chars().any(char::is_whitespace)
    {
        return false;
    }

    (type_name.eq_ignore_ascii_case("application") && subtype.eq_ignore_ascii_case("json"))
        || (subtype.len() > 5
            && subtype
                .as_bytes()
                .get(subtype.len() - 5..)
                .is_some_and(|suffix| suffix.eq_ignore_ascii_case(b"+json")))
}

pub(crate) fn variants_for_media(
    contract: &Contract,
    media_type: &str,
    media: &Value,
    media_pointer: &str,
    request: bool,
) -> Result<Vec<ExampleVariant>, Error> {
    if let Some(examples) = media.get("examples").and_then(Value::as_object) {
        let mut variants = Vec::new();
        for (name, example) in examples {
            let pointer = format!("{media_pointer}/examples/{}", pointer_escape(name));
            let resolved = contract.resolve_component_object(example, &pointer, "examples")?;
            let summary = resolved
                .value
                .get("summary")
                .and_then(Value::as_str)
                .or_else(|| resolved.value.get("description").and_then(Value::as_str))
                .unwrap_or("Contract-provided example");
            if let Some(external) = resolved.value.get("externalValue").and_then(Value::as_str) {
                variants.push(ExampleVariant {
                    label: name.clone(),
                    note: format!("{summary}. External example; this reader does not fetch it."),
                    value: None,
                    unavailable: Some(external.to_owned()),
                    unavailable_pointer: None,
                    provided: true,
                });
            } else if let Some(value) = resolved.value.get("value") {
                variants.push(ExampleVariant {
                    label: name.clone(),
                    note: summary.to_owned(),
                    value: Some(value.clone()),
                    unavailable: None,
                    unavailable_pointer: None,
                    provided: true,
                });
            }
        }
        if !variants.is_empty() {
            return Ok(variants);
        }
    }

    // Whole media payload examples are author-provided and remain opaque.
    if let Some(value) = media.get("example") {
        return Ok(vec![provided_variant(
            "Example",
            "Contract-provided example",
            value.clone(),
        )]);
    }

    if media_type.eq_ignore_ascii_case("application/xml") || media_type.ends_with("+xml") {
        return Ok(vec![unavailable_variant(
            "XML example unavailable",
            "No contract-provided XML example is available; this reader does not invent XML from a JSON schema.",
            None,
        )]);
    }

    let Some(schema) = media.get("schema") else {
        return Ok(vec![unavailable_variant(
            "No example",
            &format!("Media type {media_type} has no schema or contract-provided example."),
            Some(media_pointer.to_owned()),
        )]);
    };
    let schema_pointer = format!("{media_pointer}/schema");
    if !is_json_media_type(media_type) {
        return Ok(vec![unavailable_variant(
            "Example unavailable",
            &format!(
                "No contract-provided example is available for {media_type}; this reader does not encode JSON schema values as that media type."
            ),
            Some(schema_pointer),
        )]);
    }
    let reading = read_schema(contract, schema, &schema_pointer, &[])?;
    if let Some((label, pointer)) = reading.stop.as_ref().map(stop_description) {
        return Ok(vec![unavailable_variant(
            "Example unavailable",
            label,
            Some(pointer.to_owned()),
        )]);
    }

    if let Some(variants) = schema_provided_variants(&reading) {
        return Ok(variants);
    }

    let schema_use = if request {
        SchemaUse::Request
    } else {
        SchemaUse::Response
    };
    let mut active = BTreeSet::new();
    let mut remaining = 512;
    let outcome = derive_reading(
        contract,
        &reading,
        schema_use,
        0,
        &mut active,
        &mut remaining,
    )?;

    Ok(match outcome {
        SampleOutcome::Value { value, notes } => {
            let mut note = if request {
                "Derived from writable required fields. This is a reading aid, not a validated request.".to_owned()
            } else {
                "Derived from readable schema fields. This is a reading aid, not a validated response.".to_owned()
            };
            if !notes.is_empty() {
                note.push_str(" Source notes: ");
                note.push_str(&format_notes(&notes));
            }
            vec![ExampleVariant {
                label: "Schema-derived example".to_owned(),
                note,
                value: Some(value),
                unavailable: None,
                unavailable_pointer: None,
                provided: false,
            }]
        }
        SampleOutcome::Unavailable {
            reason,
            source_pointer,
        } => vec![unavailable_variant(
            "Example unavailable",
            &reason,
            Some(source_pointer),
        )],
    })
}

fn schema_provided_variants(reading: &SchemaReading<'_>) -> Option<Vec<ExampleVariant>> {
    for node in &reading.nodes {
        let Some(facts) = node.facts() else {
            continue;
        };
        if let Some(values) = facts.get("examples").and_then(Value::as_array)
            && !values.is_empty()
        {
            return Some(
                values
                    .iter()
                    .enumerate()
                    .map(|(index, value)| {
                        provided_variant(
                            &format!("Schema example {}", index + 1),
                            "Contract-provided schema example; payload is preserved as supplied.",
                            value.clone(),
                        )
                    })
                    .collect(),
            );
        }
        for keyword in ["example", "default", "const"] {
            if let Some(value) = facts.get(keyword) {
                return Some(vec![provided_variant(
                    &format!("Schema {keyword}"),
                    &format!(
                        "Contract-provided schema {keyword}; payload is preserved as supplied."
                    ),
                    value.clone(),
                )]);
            }
        }
        if let Some(values) = facts.get("enum").and_then(Value::as_array)
            && let Some(value) = values.first()
        {
            return Some(vec![provided_variant(
                "Enum value 1",
                "Contract-provided enum value.",
                value.clone(),
            )]);
        }
    }
    None
}

#[allow(clippy::too_many_arguments)]
fn derive_value(
    contract: &Contract,
    schema: &Value,
    pointer: &str,
    scope: &[String],
    schema_use: SchemaUse,
    depth: usize,
    active: &mut BTreeSet<String>,
    remaining: &mut usize,
) -> Result<SampleOutcome, Error> {
    let reading = read_schema(contract, schema, pointer, scope)?;
    if schema_read_omits_field(&reading, schema_use) {
        let keyword = direction_keyword(schema_use);
        return Ok(SampleOutcome::Unavailable {
            reason: format!("{keyword} property is not included in this sample direction."),
            source_pointer: annotation_pointer(&reading, keyword)
                .unwrap_or_else(|| pointer.to_owned()),
        });
    }
    derive_reading(contract, &reading, schema_use, depth, active, remaining)
}

#[allow(clippy::too_many_arguments)]
fn derive_field(
    contract: &Contract,
    schema: &Value,
    pointer: &str,
    scope: &[String],
    schema_use: SchemaUse,
    depth: usize,
    active: &mut BTreeSet<String>,
    remaining: &mut usize,
) -> Result<FieldOutcome, Error> {
    let reading = read_schema(contract, schema, pointer, scope)?;
    if schema_read_omits_field(&reading, schema_use) {
        let keyword = direction_keyword(schema_use);
        return Ok(FieldOutcome::Omitted(SampleNote {
            source_pointer: annotation_pointer(&reading, keyword)
                .unwrap_or_else(|| pointer.to_owned()),
            reason: format!("{keyword} property omitted for this sample direction"),
        }));
    }
    Ok(FieldOutcome::Included(derive_reading(
        contract, &reading, schema_use, depth, active, remaining,
    )?))
}

fn schema_read_omits_field(reading: &SchemaReading<'_>, schema_use: SchemaUse) -> bool {
    match schema_use {
        SchemaUse::Request => reading.has_true("readOnly"),
        SchemaUse::Response => reading.has_true("writeOnly"),
    }
}

fn direction_keyword(schema_use: SchemaUse) -> &'static str {
    match schema_use {
        SchemaUse::Request => "readOnly",
        SchemaUse::Response => "writeOnly",
    }
}

fn annotation_pointer(reading: &SchemaReading<'_>, keyword: &str) -> Option<String> {
    reading
        .nodes
        .iter()
        .find(|node| node.has_true(keyword))
        .map(|node| node.pointer.clone())
}

fn reading_is_array(reading: &SchemaReading<'_>) -> bool {
    let mut has_array_type = false;
    let mut has_array_keyword = false;
    let mut has_malformed_type = false;
    let mut has_valid_non_array_type = false;
    let mut has_properties = false;

    for node in &reading.nodes {
        let Some(facts) = node.facts() else {
            continue;
        };
        match facts.get("type") {
            Some(Value::String(kind)) if kind == "array" => has_array_type = true,
            Some(Value::String(kind))
                if matches!(
                    kind.as_str(),
                    "object" | "string" | "integer" | "number" | "boolean" | "null"
                ) =>
            {
                has_valid_non_array_type = true;
            }
            Some(Value::Array(kinds))
                if kinds.iter().any(|kind| kind.as_str() == Some("array")) =>
            {
                has_array_type = true;
                has_malformed_type = true;
            }
            Some(_) => has_malformed_type = true,
            None => {}
        }
        has_properties |= facts.get("properties").is_some();
        has_array_keyword |= [
            "items",
            "minItems",
            "maxItems",
            "uniqueItems",
            "prefixItems",
            "contains",
            "minContains",
            "maxContains",
            "unevaluatedItems",
            "additionalItems",
        ]
        .iter()
        .any(|keyword| facts.get(keyword).is_some());
    }

    // Explicit array declarations always enter the array-specific guard. A
    // malformed type is also an array candidate when array keywords supply
    // that context; valid non-array types and ordinary object properties keep
    // their established readers.
    has_array_type
        || has_array_keyword && has_malformed_type
        || has_array_keyword && !has_valid_non_array_type && !has_properties
}

fn derive_array(
    contract: &Contract,
    reading: &SchemaReading<'_>,
    schema_use: SchemaUse,
    depth: usize,
    active: &mut BTreeSet<String>,
    remaining: &mut usize,
) -> Result<SampleOutcome, Error> {
    let fallback_pointer = reading
        .terminal()
        .or_else(|| reading.nodes.last())
        .map_or("#", |node| node.pointer.as_str());
    let mut bounds = DerivedArrayBounds::default();

    for node in &reading.nodes {
        let Some(facts) = node.facts() else {
            continue;
        };
        let pointer = node.pointer.as_str();

        if facts
            .get("type")
            .is_some_and(|kind| !matches!(kind, Value::String(name) if name == "array"))
        {
            return Ok(unavailable(
                "Array sampling requires each applicable type to be the string `array`.",
                &format!("{pointer}/type"),
            ));
        }
        for keyword in ["oneOf", "anyOf", "allOf", "not", "if", "then", "else"] {
            if facts.get(keyword).is_some() {
                return Ok(unavailable(
                    &format!("Array {keyword} composition is not synthesized."),
                    &format!("{pointer}/{keyword}"),
                ));
            }
        }
        for keyword in [
            "prefixItems",
            "contains",
            "minContains",
            "maxContains",
            "unevaluatedItems",
            "additionalItems",
        ] {
            if facts.get(keyword).is_some() {
                return Ok(unavailable(
                    &format!("Array constraint {keyword} is not synthesized."),
                    &format!("{pointer}/{keyword}"),
                ));
            }
        }

        if let Some(minimum) = facts.get("minItems") {
            let Some(value) = minimum.as_u64() else {
                return Ok(unavailable(
                    "Array minItems must be a non-negative integer representable as u64.",
                    &format!("{pointer}/minItems"),
                ));
            };
            if bounds.minimum.as_ref().is_none_or(|old| value > old.value) {
                bounds.minimum = Some(ArrayBound {
                    value,
                    pointer: format!("{pointer}/minItems"),
                });
            }
        }
        if let Some(maximum) = facts.get("maxItems") {
            let Some(value) = maximum.as_u64() else {
                return Ok(unavailable(
                    "Array maxItems must be a non-negative integer representable as u64.",
                    &format!("{pointer}/maxItems"),
                ));
            };
            if bounds.maximum.as_ref().is_none_or(|old| value < old.value) {
                bounds.maximum = Some(ArrayBound {
                    value,
                    pointer: format!("{pointer}/maxItems"),
                });
            }
        }
        if let Some(unique) = facts.get("uniqueItems") {
            let Some(unique) = unique.as_bool() else {
                return Ok(unavailable(
                    "Array uniqueItems must be a boolean.",
                    &format!("{pointer}/uniqueItems"),
                ));
            };
            bounds.unique |= unique;
        }
        if facts.get("items").is_some() {
            bounds.item_count += 1;
            if node.reference.is_some() {
                return Ok(unavailable(
                    "Reference-sibling items conjunction is not synthesized.",
                    &format!("{pointer}/items"),
                ));
            }
            let items = node.schema.get("items").ok_or_else(|| {
                Error::Invalid(format!("applicable items schema is missing at {pointer}"))
            })?;
            bounds.items = Some((items, format!("{pointer}/items")));
        }
    }

    if let (Some(minimum), Some(maximum)) = (&bounds.minimum, &bounds.maximum)
        && minimum.value > maximum.value
    {
        return Ok(unavailable(
            "Array minItems exceeds maxItems; no array can satisfy both bounds.",
            &minimum.pointer,
        ));
    }
    if bounds.item_count > 1 {
        return Ok(unavailable(
            "Conjoined items schemas are not synthesized.",
            bounds
                .items
                .as_ref()
                .map_or(fallback_pointer, |(_, pointer)| pointer),
        ));
    }

    let minimum = bounds.minimum.as_ref().map_or(0, |bound| bound.value);
    let maximum = bounds.maximum.as_ref().map(|bound| bound.value);
    let target = if maximum == Some(0) {
        0
    } else {
        minimum.max(1)
    };
    if target > MAX_DERIVED_ARRAY_ITEMS {
        let pointer = bounds
            .minimum
            .as_ref()
            .map_or(fallback_pointer, |bound| bound.pointer.as_str());
        return Ok(unavailable(
            &format!(
                "Derived array target {target} exceeds the {MAX_DERIVED_ARRAY_ITEMS}-item limit."
            ),
            pointer,
        ));
    }
    let Ok(target_count) = usize::try_from(target) else {
        let pointer = bounds
            .minimum
            .as_ref()
            .map_or(fallback_pointer, |bound| bound.pointer.as_str());
        return Ok(unavailable(
            "Derived array target cannot be represented by this platform.",
            pointer,
        ));
    };
    if bounds.unique && target > 1 {
        let pointer = reading
            .nodes
            .iter()
            .find(|node| node.has_true("uniqueItems"))
            .map_or(fallback_pointer, |node| node.pointer.as_str());
        return Ok(unavailable(
            "uniqueItems=true supports only zero or one derived item.",
            &format!("{pointer}/uniqueItems"),
        ));
    }

    let mut notes = Vec::new();
    if let Some(minimum) = bounds.minimum.as_ref() {
        notes.push(SampleNote {
            source_pointer: minimum.pointer.clone(),
            reason: format!("bounded derived array minimum is {}", minimum.value),
        });
    }
    if let Some(maximum) = bounds.maximum.as_ref() {
        notes.push(SampleNote {
            source_pointer: maximum.pointer.clone(),
            reason: format!("bounded derived array maximum is {}", maximum.value),
        });
    }
    if bounds.unique {
        notes.push(SampleNote {
            source_pointer: reading
                .nodes
                .iter()
                .find(|node| node.has_true("uniqueItems"))
                .map_or_else(|| fallback_pointer.to_owned(), |node| node.pointer.clone())
                + "/uniqueItems",
            reason: if target == 0 {
                "empty derived array satisfies uniqueItems=true".to_owned()
            } else {
                "single-item derived array satisfies uniqueItems=true".to_owned()
            },
        });
    }

    if target == 0 {
        notes.push(SampleNote {
            source_pointer: bounds.maximum.as_ref().map_or_else(
                || fallback_pointer.to_owned(),
                |bound| bound.pointer.clone(),
            ),
            reason: "derived as an empty array; item schema was not evaluated".to_owned(),
        });
        return Ok(SampleOutcome::Value {
            value: Value::Array(Vec::new()),
            notes,
        });
    }

    let Some((items, items_pointer)) = bounds.items else {
        return Ok(unavailable(
            "A non-empty array has no supported items schema to derive.",
            &format!("{fallback_pointer}/items"),
        ));
    };
    if !items.is_object() {
        return Ok(unavailable(
            "Boolean or non-object items schemas are not synthesized.",
            &items_pointer,
        ));
    }

    let mut values = Vec::with_capacity(target_count);
    let mut compact_bytes = 2usize; // `[]`
    for index in 0..target_count {
        let item = match derive_value(
            contract,
            items,
            &items_pointer,
            &reading.scope,
            schema_use,
            depth + 1,
            active,
            remaining,
        )? {
            SampleOutcome::Value {
                value,
                notes: item_notes,
            } => {
                notes.extend(item_notes);
                value
            }
            unavailable @ SampleOutcome::Unavailable { .. } => return Ok(unavailable),
        };
        if !consume_json_nodes(&item, remaining) {
            return Ok(unavailable(
                "Sample node budget exhausted while materializing a derived array item.",
                &items_pointer,
            ));
        }
        let delimiter_bytes = usize::from(index > 0);
        let Some(item_limit) = MAX_DERIVED_ARRAY_JSON_BYTES
            .checked_sub(compact_bytes)
            .and_then(|remaining_bytes| remaining_bytes.checked_sub(delimiter_bytes))
        else {
            return Ok(unavailable(
                "Derived array compact JSON exceeds the 65,536-byte limit.",
                &items_pointer,
            ));
        };
        let mut counter = BoundedJsonCounter {
            bytes: 0,
            limit: item_limit,
            exceeded: false,
        };
        if let Err(error) = serde_json::to_writer(&mut counter, &item) {
            if counter.exceeded {
                return Ok(unavailable(
                    "Derived array compact JSON exceeds the 65,536-byte limit.",
                    &items_pointer,
                ));
            }
            return Err(Error::Invalid(format!(
                "could not count compact JSON for derived array item at {items_pointer}: {error}"
            )));
        }
        compact_bytes = compact_bytes
            .checked_add(delimiter_bytes)
            .and_then(|bytes| bytes.checked_add(counter.bytes))
            .ok_or_else(|| Error::Invalid("derived array byte count overflowed".to_owned()))?;
        values.push(item);
    }
    notes.push(SampleNote {
        source_pointer: items_pointer,
        reason: format!("derived array contains {target} item(s)"),
    });
    Ok(SampleOutcome::Value {
        value: Value::Array(values),
        notes,
    })
}

fn consume_json_nodes(value: &Value, remaining: &mut usize) -> bool {
    if *remaining == 0 {
        return false;
    }
    *remaining -= 1;
    match value {
        Value::Array(values) => values
            .iter()
            .all(|value| consume_json_nodes(value, remaining)),
        Value::Object(values) => values
            .values()
            .all(|value| consume_json_nodes(value, remaining)),
        _ => true,
    }
}

fn derive_reading(
    contract: &Contract,
    reading: &SchemaReading<'_>,
    schema_use: SchemaUse,
    depth: usize,
    active: &mut BTreeSet<String>,
    remaining: &mut usize,
) -> Result<SampleOutcome, Error> {
    let source_pointer = reading
        .terminal()
        .map(|node| node.pointer.as_str())
        .or_else(|| reading.nodes.last().map(|node| node.pointer.as_str()))
        .unwrap_or("#")
        .to_owned();
    if let Some((reason, pointer)) = reading.stop.as_ref().map(stop_description) {
        return Ok(unavailable(reason, pointer));
    }
    if depth >= 8 {
        return Ok(unavailable(
            "Sample depth limit reached; no deeper value was derived.",
            &source_pointer,
        ));
    }
    if *remaining == 0 {
        return Ok(unavailable(
            "Sample node budget exhausted; no further value was derived.",
            &source_pointer,
        ));
    }
    let active_key = format!("{source_pointer}|{}", reading.scope.join("|"));
    if !active.insert(active_key.clone()) {
        return Ok(unavailable(
            "Recursive sample path reached an active schema location.",
            &source_pointer,
        ));
    }
    *remaining -= 1;

    let result = (|| {
        for node in &reading.nodes {
            let Some(facts) = node.facts() else {
                continue;
            };
            if let Some(values) = facts.get("examples").and_then(Value::as_array)
                && let Some(value) = values.first()
            {
                return Ok(SampleOutcome::Value {
                    value: value.clone(),
                    notes: Vec::new(),
                });
            }
            for keyword in ["example", "default", "const"] {
                if let Some(value) = facts.get(keyword) {
                    return Ok(SampleOutcome::Value {
                        value: value.clone(),
                        notes: Vec::new(),
                    });
                }
            }
            if let Some(values) = facts.get("enum").and_then(Value::as_array)
                && let Some(value) = values.first()
            {
                return Ok(SampleOutcome::Value {
                    value: value.clone(),
                    notes: Vec::new(),
                });
            }
        }

        if let Some(outcome) = derive_known_format(reading)? {
            return Ok(outcome);
        }

        if let Some((source_pointer, keyword, value)) = reading.nodes.iter().find_map(|node| {
            node.facts()
                .and_then(|facts| constrained_primitive(&facts))
                .map(|(keyword, value)| (node.pointer.clone(), keyword, value))
        }) {
            return Ok(unavailable(
                &format!(
                    "Cannot derive a sample satisfying {keyword}={value} without a source value."
                ),
                &format!("{source_pointer}/{keyword}"),
            ));
        }

        let Some(terminal) = reading.terminal() else {
            let (reason, pointer) = reading.stop.as_ref().map(stop_description).unwrap_or((
                "Schema reference did not reach a terminal node.",
                &source_pointer,
            ));
            return Ok(unavailable(reason, pointer));
        };
        let schema = terminal.schema;
        let pointer = terminal.pointer.as_str();

        if reading_is_array(reading) {
            return derive_array(contract, reading, schema_use, depth, active, remaining);
        }

        if reading.nodes.iter().any(|node| {
            node.facts()
                .is_some_and(|facts| facts.get("allOf").is_some())
        }) {
            return derive_object_composition(
                contract, reading, schema_use, depth, active, remaining,
            );
        }

        for keyword in ["oneOf", "anyOf"] {
            if let Some(alternatives) = schema.get(keyword).and_then(Value::as_array) {
                return derive_first_alternative(
                    contract,
                    alternatives,
                    pointer,
                    keyword,
                    &reading.scope,
                    schema_use,
                    depth,
                    active,
                    remaining,
                );
            }
        }
        if let Some(properties) = schema.get("properties").and_then(Value::as_object) {
            if schema.get("minProperties").is_some() || schema.get("maxProperties").is_some() {
                let bound = if schema.get("minProperties").is_some() {
                    "minProperties"
                } else {
                    "maxProperties"
                };
                return Ok(unavailable(
                    &format!("Constrained object {bound} cannot be faithfully sampled."),
                    &format!("{pointer}/{bound}"),
                ));
            }
            let required: BTreeSet<String> = reading
                .nodes
                .iter()
                .filter_map(|node| node.facts())
                .filter_map(|facts| facts.get("required").and_then(Value::as_array).cloned())
                .flatten()
                .filter_map(|name| name.as_str().map(str::to_owned))
                .collect();
            let mut result = Map::new();
            let mut notes = Vec::new();
            for (name, property) in properties {
                let child_pointer = format!("{pointer}/properties/{}", pointer_escape(name));
                if matches!(schema_use, SchemaUse::Request) && !required.contains(name) {
                    notes.push(SampleNote {
                        source_pointer: child_pointer,
                        reason: "optional request property omitted".to_owned(),
                    });
                    continue;
                }
                match derive_field(
                    contract,
                    property,
                    &child_pointer,
                    &reading.scope,
                    schema_use,
                    depth + 1,
                    active,
                    remaining,
                )? {
                    FieldOutcome::Omitted(note) => notes.push(note),
                    FieldOutcome::Included(SampleOutcome::Value {
                        value,
                        notes: child_notes,
                    }) => {
                        result.insert(name.clone(), value);
                        notes.extend(child_notes);
                    }
                    FieldOutcome::Included(SampleOutcome::Unavailable {
                        reason,
                        source_pointer,
                    }) if required.contains(name) => {
                        return Ok(unavailable(&reason, &source_pointer));
                    }
                    FieldOutcome::Included(SampleOutcome::Unavailable {
                        reason,
                        source_pointer,
                    }) => notes.push(SampleNote {
                        source_pointer,
                        reason: format!("optional property omitted: {reason}"),
                    }),
                }
            }
            if schema.get("additionalProperties").is_some() {
                if schema
                    .get("minProperties")
                    .and_then(Value::as_u64)
                    .is_some_and(|minimum| minimum > 0)
                {
                    return Ok(unavailable(
                        "A required map entry cannot be derived without inventing a property name.",
                        &format!("{pointer}/minProperties"),
                    ));
                }
                notes.push(SampleNote {
                    source_pointer: format!("{pointer}/additionalProperties"),
                    reason: "shown as an empty map; no map key was invented".to_owned(),
                });
            }
            return Ok(SampleOutcome::Value {
                value: Value::Object(result),
                notes,
            });
        }

        let value = match schema.get("type").and_then(Value::as_str) {
            Some("string") => Value::String("string".to_owned()),
            Some("integer" | "number") => Value::Number(0.into()),
            Some("boolean") => Value::Bool(false),
            Some("null") => Value::Null,
            Some("object") => Value::Object(Map::new()),
            _ => {
                return Ok(unavailable(
                    "Schema type is unbound or unsupported for a source-faithful sample.",
                    pointer,
                ));
            }
        };
        Ok(SampleOutcome::Value {
            value,
            notes: Vec::new(),
        })
    })();
    active.remove(&active_key);
    result
}

fn constrained_primitive(schema: &Value) -> Option<(&'static str, String)> {
    for key in [
        "minimum",
        "maximum",
        "exclusiveMinimum",
        "exclusiveMaximum",
        "multipleOf",
        "pattern",
        "format",
        "minLength",
        "maxLength",
    ] {
        if let Some(value) = schema.get(key) {
            return Some((key, value.to_string()));
        }
    }
    None
}

fn derive_known_format(reading: &SchemaReading<'_>) -> Result<Option<SampleOutcome>, Error> {
    let facts = reading
        .nodes
        .iter()
        .filter_map(|node| node.facts().map(|facts| (node.pointer.as_str(), facts)))
        .collect::<Vec<_>>();
    let Some((format_pointer, format_value)) = facts
        .iter()
        .find_map(|(pointer, facts)| facts.get("format").map(|format| (*pointer, format)))
    else {
        return Ok(None);
    };
    let Some(format) = format_value.as_str() else {
        return Ok(Some(unavailable(
            "Schema format is malformed; no reading placeholder was derived.",
            &format!("{format_pointer}/format"),
        )));
    };

    if !reading
        .terminal()
        .is_some_and(|node| node.schema.is_object())
    {
        return Ok(Some(unavailable(
            "Formatted schema does not resolve to an object schema; no reading placeholder was derived.",
            &format!("{format_pointer}/format"),
        )));
    }

    let expected_type = match format {
        "uuid" | "date-time" | "uri" => "string",
        "int32" | "int64" => "integer",
        _ => {
            return Ok(Some(unavailable(
                &format!("Format {format} has no configured reading placeholder."),
                &format!("{format_pointer}/format"),
            )));
        }
    };
    let placeholder = match format {
        "uuid" => Value::String("00000000-0000-4000-8000-000000000000".to_owned()),
        "date-time" => Value::String("2000-01-01T00:00:00Z".to_owned()),
        "uri" => Value::String("https://example.invalid/resource".to_owned()),
        "int32" | "int64" => Value::Number(0.into()),
        _ => return Ok(None),
    };

    let mut minimum_length = 0_u64;
    let mut maximum_length = u64::MAX;
    let mut minimum_pointer = format_pointer.to_owned();
    let mut maximum_pointer = format_pointer.to_owned();
    let mut saw_format = false;
    for (pointer, facts) in &facts {
        if let Some(value) = facts.get("type")
            && value.as_str() != Some(expected_type)
        {
            return Ok(Some(unavailable(
                &format!("Format {format} requires a {expected_type} schema type."),
                &format!("{pointer}/type"),
            )));
        }
        if let Some(value) = facts.get("format") {
            let Some(value) = value.as_str() else {
                return Ok(Some(unavailable(
                    "Schema format is malformed; no reading placeholder was derived.",
                    &format!("{pointer}/format"),
                )));
            };
            if value != format {
                return Ok(Some(unavailable(
                    "Conflicting schema formats cannot share one reading placeholder.",
                    &format!("{pointer}/format"),
                )));
            }
            saw_format = true;
        }
        for (key, value) in facts
            .as_object()
            .into_iter()
            .flat_map(|object| object.iter())
        {
            let benign = matches!(
                key.as_str(),
                "type"
                    | "format"
                    | "title"
                    | "description"
                    | "deprecated"
                    | "nullable"
                    | "readOnly"
                    | "writeOnly"
                    | "xml"
                    | "externalDocs"
                    | "$id"
                    | "$schema"
                    | "$anchor"
                    | "$dynamicAnchor"
                    | "$comment"
                    | "definitions"
                    | "$defs"
            ) || key.starts_with("x-");
            if !benign && !matches!(key.as_str(), "minLength" | "maxLength") {
                return Ok(Some(unavailable(
                    &format!(
                        "Format {format} cannot be combined with unsupported schema constraint {key}."
                    ),
                    &format!("{pointer}/{key}"),
                )));
            }
            if matches!(format, "int32" | "int64")
                && matches!(key.as_str(), "minLength" | "maxLength")
            {
                return Ok(Some(unavailable(
                    "Integer format placeholders do not support string length constraints.",
                    &format!("{pointer}/{key}"),
                )));
            }
            if matches!(format, "uuid" | "date-time" | "uri")
                && matches!(key.as_str(), "minLength" | "maxLength")
            {
                let Some(bound) = value.as_u64() else {
                    return Ok(Some(unavailable(
                        &format!("Malformed {key} bound for formatted string."),
                        &format!("{pointer}/{key}"),
                    )));
                };
                if key == "minLength" && bound > minimum_length {
                    minimum_length = bound;
                    minimum_pointer = (*pointer).to_owned();
                }
                if key == "maxLength" && bound < maximum_length {
                    maximum_length = bound;
                    maximum_pointer = (*pointer).to_owned();
                }
            }
        }
    }
    if !saw_format {
        return Ok(None);
    }
    if !facts.iter().any(|(_, facts)| facts.get("type").is_some()) {
        return Ok(Some(unavailable(
            &format!("Format {format} requires an explicit {expected_type} schema type."),
            &format!("{format_pointer}/format"),
        )));
    }
    if matches!(format, "uuid" | "date-time" | "uri") {
        if minimum_length > maximum_length {
            return Ok(Some(unavailable(
                "Formatted string length constraints have an empty intersection.",
                &format!("{maximum_pointer}/maxLength"),
            )));
        }
        let length = placeholder.as_str().map_or(0, str::len) as u64;
        if length < minimum_length || length > maximum_length {
            let pointer = if length < minimum_length {
                format!("{minimum_pointer}/minLength")
            } else {
                format!("{maximum_pointer}/maxLength")
            };
            return Ok(Some(unavailable(
                &format!(
                    "The fixed {format} reading placeholder does not satisfy the applicable string length bounds."
                ),
                &pointer,
            )));
        }
    }
    Ok(Some(SampleOutcome::Value {
        value: placeholder,
        notes: vec![SampleNote {
            source_pointer: format!("{format_pointer}/format"),
            reason: format!(
                "deterministic {format} reading placeholder; not validated as a live API value"
            ),
        }],
    }))
}

#[allow(clippy::too_many_arguments)]
fn derive_object_composition(
    contract: &Contract,
    reading: &SchemaReading<'_>,
    schema_use: SchemaUse,
    depth: usize,
    active: &mut BTreeSet<String>,
    remaining: &mut usize,
) -> Result<SampleOutcome, Error> {
    let mut composition = ObjectComposition::default();
    for node in &reading.nodes {
        if let Some(facts) = node.facts() {
            if node.reference.is_some() && !is_reference_metadata_only(&facts) {
                let key = facts
                    .as_object()
                    .and_then(|object| object.keys().find(|key| !is_reference_metadata_key(key)))
                    .map(String::as_str)
                    .unwrap_or("$ref");
                return Ok(unavailable(
                    "Applicable reference-sibling constraints are not combined into an object sample.",
                    &format!("{}/{key}", node.pointer),
                ));
            }
            if let Some(outcome) = collect_object_facts(
                contract,
                &facts,
                &node.pointer,
                &reading.scope,
                schema_use,
                0,
                active,
                remaining,
                &mut composition,
            )? {
                return Ok(outcome);
            }
        }
    }

    for required in &composition.required {
        if !composition.property_names.contains(required) {
            let pointer = composition
                .required_pointers
                .get(required)
                .map(String::as_str)
                .unwrap_or("#");
            return Ok(unavailable(
                &format!("Required object property {required} has no collected property schema."),
                pointer,
            ));
        }
    }
    if !composition.saw_object_shape {
        let pointer = reading
            .nodes
            .iter()
            .find(|node| {
                node.facts()
                    .is_some_and(|facts| facts.get("allOf").is_some())
            })
            .map(|node| format!("{}/allOf", node.pointer))
            .unwrap_or_else(|| "#".to_owned());
        return Ok(unavailable(
            "Object allOf has no branch that establishes an object type or property shape.",
            &pointer,
        ));
    }

    let mut result = Map::new();
    let mut notes = Vec::new();
    for (name, property) in composition.properties {
        if matches!(schema_use, SchemaUse::Request) && !composition.required.contains(&name) {
            notes.push(SampleNote {
                source_pointer: property.source_pointer,
                reason: "optional request property omitted".to_owned(),
            });
            continue;
        }
        match derive_field(
            contract,
            &property.schema,
            &property.source_pointer,
            &property.scope,
            schema_use,
            depth + 1,
            active,
            remaining,
        )? {
            FieldOutcome::Omitted(note) => notes.push(note),
            FieldOutcome::Included(SampleOutcome::Value {
                value,
                notes: child_notes,
            }) => {
                result.insert(name, value);
                notes.extend(child_notes);
            }
            FieldOutcome::Included(SampleOutcome::Unavailable {
                reason,
                source_pointer,
            }) if composition.required.contains(&name) => {
                return Ok(unavailable(&reason, &source_pointer));
            }
            FieldOutcome::Included(SampleOutcome::Unavailable {
                reason,
                source_pointer,
            }) => notes.push(SampleNote {
                source_pointer,
                reason: format!("optional property omitted: {reason}"),
            }),
        }
    }
    Ok(SampleOutcome::Value {
        value: Value::Object(result),
        notes,
    })
}

#[allow(clippy::too_many_arguments)]
fn collect_object_part(
    contract: &Contract,
    schema: &Value,
    pointer: &str,
    inherited_scope: &[String],
    schema_use: SchemaUse,
    composition_depth: usize,
    active: &mut BTreeSet<String>,
    remaining: &mut usize,
    composition: &mut ObjectComposition,
) -> Result<Option<SampleOutcome>, Error> {
    if composition_depth >= 8 {
        return Ok(Some(unavailable(
            "Object allOf composition depth limit reached; no deeper value was collected.",
            pointer,
        )));
    }
    if *remaining == 0 {
        return Ok(Some(unavailable(
            "Sample node budget exhausted while collecting object allOf facts.",
            pointer,
        )));
    }
    let reading = read_schema(contract, schema, pointer, inherited_scope)?;
    if let Some((reason, pointer)) = reading.stop.as_ref().map(stop_description) {
        return Ok(Some(unavailable(reason, pointer)));
    }
    if !reading
        .terminal()
        .is_some_and(|node| node.schema.is_object())
    {
        return Ok(Some(unavailable(
            "Object allOf branch does not resolve to an object schema.",
            pointer,
        )));
    }
    let resolved_pointer = reading
        .terminal()
        .map(|node| node.pointer.as_str())
        .or_else(|| reading.nodes.last().map(|node| node.pointer.as_str()))
        .unwrap_or(pointer);
    let active_key = format!("{resolved_pointer}|{}", reading.scope.join("|"));
    if !active.insert(active_key.clone()) {
        return Ok(Some(unavailable(
            "Recursive object allOf reached an active schema location.",
            resolved_pointer,
        )));
    }
    *remaining -= 1;
    let result = (|| {
        let mut saw_schema_object = false;
        for node in &reading.nodes {
            if let Some(facts) = node.facts() {
                saw_schema_object = true;
                if node.reference.is_some() && !is_reference_metadata_only(&facts) {
                    let key = facts
                        .as_object()
                        .and_then(|object| {
                            object.keys().find(|key| !is_reference_metadata_key(key))
                        })
                        .map(String::as_str)
                        .unwrap_or("$ref");
                    return Ok(Some(unavailable(
                        "Applicable reference-sibling constraints are not combined into an object sample.",
                        &format!("{}/{key}", node.pointer),
                    )));
                }
                if let Some(outcome) = collect_object_facts(
                    contract,
                    &facts,
                    &node.pointer,
                    &reading.scope,
                    schema_use,
                    composition_depth,
                    active,
                    remaining,
                    composition,
                )? {
                    return Ok(Some(outcome));
                }
            }
        }
        if !saw_schema_object {
            return Ok(Some(unavailable(
                "Boolean or unbound object allOf branches are not synthesized.",
                pointer,
            )));
        }
        Ok(None)
    })();
    active.remove(&active_key);
    result
}

#[allow(clippy::too_many_arguments)]
fn collect_object_facts(
    contract: &Contract,
    facts: &Value,
    pointer: &str,
    scope: &[String],
    schema_use: SchemaUse,
    composition_depth: usize,
    active: &mut BTreeSet<String>,
    remaining: &mut usize,
    composition: &mut ObjectComposition,
) -> Result<Option<SampleOutcome>, Error> {
    let Some(object) = facts.as_object() else {
        return Ok(Some(unavailable(
            "Object allOf branch is not an object schema.",
            pointer,
        )));
    };
    for (key, value) in object {
        let metadata = matches!(
            key.as_str(),
            "title"
                | "description"
                | "deprecated"
                | "readOnly"
                | "writeOnly"
                | "nullable"
                | "xml"
                | "externalDocs"
                | "$id"
                | "$schema"
                | "$anchor"
                | "$dynamicAnchor"
                | "$comment"
                | "definitions"
                | "$defs"
        ) || key.starts_with("x-");
        if metadata || matches!(key.as_str(), "type" | "properties" | "required" | "allOf") {
            continue;
        }
        if key == "additionalProperties" && value.as_bool() == Some(true) {
            continue;
        }
        return Ok(Some(unavailable(
            &format!("Object allOf cannot faithfully combine schema keyword {key}."),
            &format!("{pointer}/{key}"),
        )));
    }
    if (matches!(schema_use, SchemaUse::Request)
        && object.get("readOnly").and_then(Value::as_bool) == Some(true))
        || (matches!(schema_use, SchemaUse::Response)
            && object.get("writeOnly").and_then(Value::as_bool) == Some(true))
    {
        let keyword = if matches!(schema_use, SchemaUse::Request) {
            "readOnly"
        } else {
            "writeOnly"
        };
        return Ok(Some(unavailable(
            "A direction-excluded object allOf branch cannot be represented without omitting its whole schema.",
            &format!("{pointer}/{keyword}"),
        )));
    }
    if let Some(schema_type) = object.get("type") {
        if schema_type.as_str() != Some("object") {
            return Ok(Some(unavailable(
                "Object allOf requires each explicit type to be object.",
                &format!("{pointer}/type"),
            )));
        }
        composition.saw_object_shape = true;
    }
    if let Some(properties) = object.get("properties") {
        let Some(properties) = properties.as_object() else {
            return Ok(Some(unavailable(
                "Object allOf properties must be an object.",
                &format!("{pointer}/properties"),
            )));
        };
        composition.saw_object_shape = true;
        for (name, schema) in properties {
            if *remaining == 0 {
                return Ok(Some(unavailable(
                    "Sample node budget exhausted while collecting object properties.",
                    &format!("{pointer}/properties/{}", pointer_escape(name)),
                )));
            }
            *remaining -= 1;
            let property_pointer = format!("{pointer}/properties/{}", pointer_escape(name));
            if composition.property_names.contains(name) {
                return Ok(Some(unavailable(
                    &format!("Object allOf property {name} is defined more than once."),
                    &property_pointer,
                )));
            }
            composition.property_names.insert(name.clone());
            composition.properties.push((
                name.clone(),
                ObjectProperty {
                    schema: schema.clone(),
                    source_pointer: property_pointer,
                    scope: scope.to_vec(),
                },
            ));
        }
    }
    if let Some(required) = object.get("required") {
        let Some(required) = required.as_array() else {
            return Ok(Some(unavailable(
                "Object allOf required must be an array of property names.",
                &format!("{pointer}/required"),
            )));
        };
        for (index, name) in required.iter().enumerate() {
            if *remaining == 0 {
                return Ok(Some(unavailable(
                    "Sample node budget exhausted while collecting required names.",
                    &format!("{pointer}/required/{index}"),
                )));
            }
            *remaining -= 1;
            let Some(name) = name.as_str() else {
                return Ok(Some(unavailable(
                    "Object allOf required entries must be property names.",
                    &format!("{pointer}/required/{index}"),
                )));
            };
            composition.required.insert(name.to_owned());
            composition
                .required_pointers
                .entry(name.to_owned())
                .or_insert_with(|| format!("{pointer}/required/{index}"));
        }
    }
    if let Some(all_of) = object.get("allOf") {
        let Some(all_of) = all_of.as_array() else {
            return Ok(Some(unavailable(
                "Object allOf must be an array of schemas.",
                &format!("{pointer}/allOf"),
            )));
        };
        if all_of.is_empty() {
            return Ok(Some(unavailable(
                "Empty object allOf composition has no declared sample shape.",
                &format!("{pointer}/allOf"),
            )));
        }
        for (index, branch) in all_of.iter().enumerate() {
            let branch_pointer = format!("{pointer}/allOf/{index}");
            if let Some(outcome) = collect_object_part(
                contract,
                branch,
                &branch_pointer,
                scope,
                schema_use,
                composition_depth + 1,
                active,
                remaining,
                composition,
            )? {
                return Ok(Some(outcome));
            }
        }
    }
    Ok(None)
}

fn is_reference_metadata_only(facts: &Value) -> bool {
    facts
        .as_object()
        .is_some_and(|object| object.keys().all(|key| is_reference_metadata_key(key)))
}

fn is_reference_metadata_key(key: &str) -> bool {
    matches!(
        key,
        "title"
            | "description"
            | "deprecated"
            | "readOnly"
            | "writeOnly"
            | "nullable"
            | "xml"
            | "externalDocs"
            | "$id"
            | "$schema"
            | "$anchor"
            | "$dynamicAnchor"
            | "$comment"
            | "definitions"
            | "$defs"
    ) || key.starts_with("x-")
}

#[allow(clippy::too_many_arguments)]
fn derive_first_alternative(
    contract: &Contract,
    alternatives: &[Value],
    pointer: &str,
    keyword: &str,
    scope: &[String],
    schema_use: SchemaUse,
    depth: usize,
    active: &mut BTreeSet<String>,
    remaining: &mut usize,
) -> Result<SampleOutcome, Error> {
    let mut first_unavailable = None;
    for (index, schema) in alternatives.iter().enumerate() {
        if schema.get("not").is_some() {
            continue;
        }
        let branch_pointer = format!("{pointer}/{keyword}/{index}");
        match derive_value(
            contract,
            schema,
            &branch_pointer,
            scope,
            schema_use,
            depth + 1,
            active,
            remaining,
        )? {
            SampleOutcome::Value { value, mut notes } => {
                notes.insert(
                    0,
                    SampleNote {
                        source_pointer: branch_pointer,
                        reason: format!(
                            "selected {keyword} alternative {} as a reading example",
                            index + 1
                        ),
                    },
                );
                return Ok(SampleOutcome::Value { value, notes });
            }
            unavailable_outcome @ SampleOutcome::Unavailable { .. } => {
                first_unavailable.get_or_insert(unavailable_outcome);
            }
        }
    }
    Ok(first_unavailable.unwrap_or_else(|| {
        unavailable(
            &format!("No {keyword} branch could be represented as a source-faithful sample."),
            pointer,
        )
    }))
}

fn stop_description(stop: &SchemaReadStop) -> (&'static str, &str) {
    match stop {
        SchemaReadStop::Cycle { pointer } => (
            "Recursive schema reference; no sample was derived.",
            pointer,
        ),
        SchemaReadStop::DepthLimit { pointer } => (
            "Schema reference depth limit reached; no sample was derived.",
            pointer,
        ),
    }
}

fn unavailable(reason: &str, source_pointer: &str) -> SampleOutcome {
    SampleOutcome::Unavailable {
        reason: reason.to_owned(),
        source_pointer: source_pointer.to_owned(),
    }
}

fn format_notes(notes: &[SampleNote]) -> String {
    notes
        .iter()
        .map(|note| format!("{}: {}", note.source_pointer, note.reason))
        .collect::<Vec<_>>()
        .join("; ")
}

fn provided_variant(label: &str, note: &str, value: Value) -> ExampleVariant {
    ExampleVariant {
        label: label.to_owned(),
        note: note.to_owned(),
        value: Some(value),
        unavailable: None,
        unavailable_pointer: None,
        provided: true,
    }
}

fn unavailable_variant(
    label: &str,
    reason: &str,
    source_pointer: Option<String>,
) -> ExampleVariant {
    let note = source_pointer
        .as_ref()
        .map(|pointer| format!("{reason} Source: {pointer}."))
        .unwrap_or_else(|| reason.to_owned());
    ExampleVariant {
        label: label.to_owned(),
        note,
        value: None,
        unavailable: None,
        unavailable_pointer: source_pointer,
        provided: false,
    }
}

fn pointer_escape(value: &str) -> String {
    value.replace('~', "~0").replace('/', "~1")
}

#[cfg(test)]
mod media_type_tests {
    use super::is_json_media_type;

    #[test]
    fn concrete_json_media_types_allow_case_parameters_and_outer_ows() {
        for media_type in [
            "application/json",
            "Application/JSON",
            "application/json; charset=utf-8",
            "application/json;stream=watch",
            "application/merge-patch+json",
            "model/gltf+json",
            "text/example+JSON",
            "\t application/json \t; charset=utf-8\t",
        ] {
            assert!(is_json_media_type(media_type), "{media_type:?}");
        }
    }

    #[test]
    fn non_json_and_non_concrete_media_types_are_not_json() {
        for media_type in [
            "application/cbor",
            "application/yaml",
            "application/vnd.example.protobuf",
            "application/json-seq",
            "application/jsonx",
            "*/*",
            "application/*+json",
            "application/+json",
            "json",
            "application/",
            "/json",
            "application /json",
            "application/ json",
            "application/\u{00a0}json",
            "application/json\n",
            "application/json; x=\r1",
            "application/json; x=\u{7f}",
            "application\u{000b}/json",
        ] {
            assert!(!is_json_media_type(media_type), "{media_type:?}");
        }
    }
}
