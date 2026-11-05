use std::collections::BTreeSet;

use serde_json::{Map, Value};

use crate::Error;
use crate::openapi::Contract;

#[derive(Debug, Clone)]
pub(crate) struct ExampleVariant {
    pub label: String,
    pub note: String,
    pub value: Option<Value>,
    pub unavailable: Option<String>,
    pub provided: bool,
}

type ResolvedSchema<'a> = (&'a Value, String, Vec<String>);

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
                    provided: true,
                });
            } else if let Some(value) = resolved.value.get("value") {
                variants.push(ExampleVariant {
                    label: name.clone(),
                    note: summary.to_owned(),
                    value: Some(value.clone()),
                    unavailable: None,
                    provided: true,
                });
            }
        }
        if !variants.is_empty() {
            return Ok(variants);
        }
    }

    if let Some(value) = media.get("example") {
        return Ok(vec![ExampleVariant {
            label: "Example".to_owned(),
            note: "Contract-provided example".to_owned(),
            value: Some(value.clone()),
            unavailable: None,
            provided: true,
        }]);
    }

    if media_type.eq_ignore_ascii_case("application/xml") || media_type.ends_with("+xml") {
        return Ok(vec![unavailable_variant(
            "XML example unavailable",
            "No contract-provided XML example is available; this reader does not invent XML from a JSON schema.",
        )]);
    }

    let Some(schema) = media.get("schema") else {
        return Ok(vec![unavailable_variant(
            "No example",
            "This media type has no schema or contract-provided example.",
        )]);
    };
    let schema_pointer = format!("{media_pointer}/schema");
    let Some((resolved_schema, resolved_pointer, scope)) =
        resolve_schema(contract, schema, &schema_pointer, &[])?
    else {
        return Ok(vec![unavailable_variant(
            "Example unavailable",
            &format!("Recursive schema reference at {schema_pointer}; no sample was generated."),
        )]);
    };
    if let Some(values) = resolved_schema.get("examples").and_then(Value::as_array)
        && !values.is_empty()
    {
        return Ok(values
            .iter()
            .enumerate()
            .map(|(index, value)| ExampleVariant {
                label: format!("Schema example {}", index + 1),
                note: "Contract-provided schema example".to_owned(),
                value: Some(value.clone()),
                unavailable: None,
                provided: true,
            })
            .collect());
    }
    for keyword in ["example", "default", "const"] {
        if let Some(value) = resolved_schema.get(keyword) {
            return Ok(vec![ExampleVariant {
                label: format!("Schema {keyword}"),
                note: format!("Contract-provided schema {keyword}"),
                value: Some(value.clone()),
                unavailable: None,
                provided: true,
            }]);
        }
    }
    if let Some(values) = resolved_schema.get("enum").and_then(Value::as_array)
        && !values.is_empty()
    {
        return Ok(values
            .iter()
            .enumerate()
            .map(|(index, value)| ExampleVariant {
                label: format!("Enum value {}", index + 1),
                note: "Contract-provided enum value".to_owned(),
                value: Some(value.clone()),
                unavailable: None,
                provided: true,
            })
            .collect());
    }

    let mut active = BTreeSet::new();
    let mut notes = Vec::new();
    let value = derive_value(
        contract,
        resolved_schema,
        &resolved_pointer,
        &scope,
        request,
        0,
        &mut active,
        &mut notes,
    )?;
    Ok(match value {
        Some(value) => vec![ExampleVariant {
            label: "Schema-derived example".to_owned(),
            note: format!(
                "{}{}",
                if request {
                    "Derived from required writable fields. This is a reading aid, not a validated request."
                    .to_owned()
                } else {
                    "Derived from readable schema fields. This is a reading aid, not a validated response."
                    .to_owned()
                },
                if notes.is_empty() {
                    String::new()
                } else {
                    format!(" Source notes: {}", notes.join("; "))
                }
            ),
            value: Some(value),
            unavailable: None,
            provided: false,
        }],
        None => vec![unavailable_variant(
            "Example unavailable",
            &format!("No bounded source-faithful sample could be derived for {media_type}."),
        )],
    })
}

fn resolve_schema<'a>(
    contract: &'a Contract,
    mut schema: &'a Value,
    pointer: &str,
    inherited_scope: &[String],
) -> Result<Option<ResolvedSchema<'a>>, Error> {
    let mut pointer = pointer.to_owned();
    let mut scope = if inherited_scope.is_empty() {
        contract.schema_resources.lexical_scope(&pointer)
    } else {
        inherited_scope.to_vec()
    };
    let current = contract.schema_resources.resource_for_pointer(&pointer);
    if !scope.contains(&current) {
        scope.push(current);
    }
    let mut visited = BTreeSet::new();
    while let Some(object) = schema.as_object() {
        let dynamic = object
            .get("$dynamicRef")
            .and_then(Value::as_str)
            .map(|reference| (reference, true))
            .or_else(|| {
                object
                    .get("$ref")
                    .and_then(Value::as_str)
                    .map(|reference| (reference, false))
            });
        let Some((reference, is_dynamic)) = dynamic else {
            break;
        };
        if !visited.insert(pointer.clone()) {
            return Ok(None);
        }
        let target = contract.resolve_schema_reference(&pointer, reference, is_dynamic, &scope)?;
        schema = contract
            .schema_resources
            .schema_at(&contract.document, &target.pointer)
            .ok_or_else(|| {
                Error::Invalid(format!(
                    "resolved example schema {} is missing",
                    target.pointer
                ))
            })?;
        pointer = target.pointer;
        if !scope.contains(&target.uri) {
            scope.push(target.uri);
        }
    }
    Ok(Some((schema, pointer, scope)))
}

#[allow(clippy::too_many_arguments)]
fn derive_value(
    contract: &Contract,
    schema: &Value,
    pointer: &str,
    scope: &[String],
    request: bool,
    depth: usize,
    active: &mut BTreeSet<String>,
    notes: &mut Vec<String>,
) -> Result<Option<Value>, Error> {
    let active_key = format!("{pointer}|{}", scope.join("|"));
    if depth >= 8 || !active.insert(active_key.clone()) {
        return Ok(None);
    }
    let result = (|| {
        if let Some(values) = schema.get("examples").and_then(Value::as_array)
            && let Some(value) = values.first()
        {
            return Ok(Some(value.clone()));
        }
        for keyword in ["example", "default", "const"] {
            if let Some(value) = schema.get(keyword) {
                return Ok(Some(value.clone()));
            }
        }
        if let Some(values) = schema.get("enum").and_then(Value::as_array)
            && let Some(value) = values.first()
        {
            return Ok(Some(value.clone()));
        }
        if let Some(alternatives) = schema.get("oneOf").and_then(Value::as_array) {
            return derive_first_alternative(
                contract,
                alternatives,
                pointer,
                "oneOf",
                scope,
                request,
                depth,
                active,
                notes,
            );
        }
        if let Some(alternatives) = schema.get("anyOf").and_then(Value::as_array) {
            return derive_first_alternative(
                contract,
                alternatives,
                pointer,
                "anyOf",
                scope,
                request,
                depth,
                active,
                notes,
            );
        }
        if schema.get("allOf").is_some() {
            return Ok(None);
        }
        let Some((schema, resolved_pointer, resolved_scope)) =
            resolve_schema(contract, schema, pointer, scope)?
        else {
            return Ok(None);
        };
        if let Some(values) = schema.get("examples").and_then(Value::as_array)
            && let Some(value) = values.first()
        {
            return Ok(Some(value.clone()));
        }
        for keyword in ["example", "default", "const"] {
            if let Some(value) = schema.get(keyword) {
                return Ok(Some(value.clone()));
            }
        }
        if let Some(values) = schema.get("enum").and_then(Value::as_array)
            && let Some(value) = values.first()
        {
            return Ok(Some(value.clone()));
        }
        if let Some(properties) = schema.get("properties").and_then(Value::as_object) {
            if schema.get("minProperties").is_some() || schema.get("maxProperties").is_some() {
                return Ok(None);
            }
            let required: BTreeSet<&str> = schema
                .get("required")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
                .filter_map(Value::as_str)
                .collect();
            let mut result = Map::new();
            for (name, property) in properties {
                let child_pointer =
                    format!("{resolved_pointer}/properties/{}", pointer_escape(name));
                if request
                    && (!required.contains(name.as_str())
                        || property.get("readOnly") == Some(&Value::Bool(true)))
                {
                    notes.push(format!(
                        "{child_pointer} omitted: {}",
                        if required.contains(name.as_str()) {
                            "readOnly request property"
                        } else {
                            "optional request property"
                        }
                    ));
                    continue;
                }
                if !request && property.get("writeOnly") == Some(&Value::Bool(true)) {
                    notes.push(format!(
                        "{child_pointer} omitted: writeOnly response property"
                    ));
                    continue;
                }
                if let Some(value) = derive_value(
                    contract,
                    property,
                    &child_pointer,
                    &resolved_scope,
                    request,
                    depth + 1,
                    active,
                    notes,
                )? {
                    result.insert(name.clone(), value);
                } else if required.contains(name.as_str()) {
                    return Ok(None);
                } else if !request {
                    notes.push(format!(
                        "{child_pointer} omitted: no bounded source-faithful value was available"
                    ));
                }
            }
            if schema.get("additionalProperties").is_some() {
                if schema
                    .get("minProperties")
                    .and_then(Value::as_u64)
                    .unwrap_or(0)
                    > 0
                {
                    return Ok(None);
                }
                notes.push(format!("{resolved_pointer}/additionalProperties shown as an empty map; no map key was invented"));
            }
            if result.is_empty() && request {
                return Ok(None);
            }
            return Ok(Some(Value::Object(result)));
        }
        if let Some(items) = schema.get("items") {
            if ["minItems", "maxItems", "uniqueItems", "prefixItems"]
                .iter()
                .any(|key| schema.get(key).is_some())
            {
                return Ok(None);
            }
            let child_pointer = format!("{resolved_pointer}/items");
            return Ok(derive_value(
                contract,
                items,
                &child_pointer,
                &resolved_scope,
                request,
                depth + 1,
                active,
                notes,
            )?
            .map(|value| Value::Array(vec![value])));
        }
        match schema.get("type").and_then(Value::as_str) {
            Some("string")
                if ["pattern", "format", "minLength", "maxLength"]
                    .iter()
                    .all(|key| schema.get(key).is_none()) =>
            {
                Ok(Some(Value::String("string".to_owned())))
            }
            Some("integer" | "number")
                if [
                    "minimum",
                    "maximum",
                    "exclusiveMinimum",
                    "exclusiveMaximum",
                    "multipleOf",
                ]
                .iter()
                .all(|key| schema.get(key).is_none()) =>
            {
                Ok(Some(Value::Number(0.into())))
            }
            Some("boolean") => Ok(Some(Value::Bool(false))),
            Some("null") => Ok(Some(Value::Null)),
            _ => Ok(None),
        }
    })();
    active.remove(&active_key);
    result
}

// Keep the schema pointer, resolution scope, direction, recursion depth, and
// active-cycle set explicit at this recursive decision point.
#[allow(clippy::too_many_arguments)]
fn derive_first_alternative(
    contract: &Contract,
    alternatives: &[Value],
    pointer: &str,
    keyword: &str,
    scope: &[String],
    request: bool,
    depth: usize,
    active: &mut BTreeSet<String>,
    notes: &mut Vec<String>,
) -> Result<Option<Value>, Error> {
    for (index, schema) in alternatives.iter().enumerate() {
        if schema.get("not").is_some() {
            continue;
        }
        let mut branch_notes = Vec::new();
        if let Some(value) = derive_value(
            contract,
            schema,
            &format!("{pointer}/{keyword}/{index}"),
            scope,
            request,
            depth + 1,
            active,
            &mut branch_notes,
        )? {
            notes.push(format!(
                "{pointer}/{keyword}/{index} selected as a reading example"
            ));
            notes.extend(branch_notes);
            return Ok(Some(value));
        }
    }
    Ok(None)
}

fn unavailable_variant(label: &str, reason: &str) -> ExampleVariant {
    ExampleVariant {
        label: label.to_owned(),
        note: reason.to_owned(),
        value: None,
        unavailable: None,
        provided: false,
    }
}

fn pointer_escape(value: &str) -> String {
    value.replace('~', "~0").replace('/', "~1")
}
