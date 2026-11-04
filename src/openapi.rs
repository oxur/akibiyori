use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use serde_json::Value;

use crate::Error;
use crate::config::{ContractInput, validate_identifier};

const METHODS: &[&str] = &[
    "get", "put", "post", "delete", "options", "head", "patch", "trace",
];

#[derive(Debug)]
pub(crate) struct Contract {
    pub id: String,
    pub label: String,
    pub source: String,
    pub document: Value,
    pub operations: Vec<Operation>,
    pub schemas: BTreeMap<String, Value>,
}

#[derive(Debug)]
pub(crate) struct Operation {
    pub method: String,
    pub path: String,
    pub id: String,
    pub value: Value,
    pub path_item: Value,
}

impl Contract {
    pub fn load(input: &ContractInput, root: &Path) -> Result<Self, Error> {
        let path = root.join(&input.path);
        let source_label = format!("{} ({})", input.id, input.path.display());
        let bytes = std::fs::read(&path).map_err(|error| Error::read(&path, error))?;
        let document: Value = serde_json::from_slice(&bytes).map_err(|source| Error::Json {
            path: path.clone(),
            source,
        })?;
        let root_object = document
            .as_object()
            .ok_or_else(|| fail(&source_label, "OpenAPI root must be an object"))?;
        let version = root_object
            .get("openapi")
            .and_then(Value::as_str)
            .unwrap_or_default();
        if !version.starts_with("3.0.") {
            return Err(fail(
                &source_label,
                &format!("unsupported OpenAPI version {version:?}; expected 3.0.x"),
            ));
        }
        let paths = root_object
            .get("paths")
            .and_then(Value::as_object)
            .ok_or_else(|| fail(&source_label, "OpenAPI paths must be an object"))?;
        let schemas = root_object
            .get("components")
            .and_then(Value::as_object)
            .and_then(|components| components.get("schemas"))
            .and_then(Value::as_object)
            .ok_or_else(|| {
                fail(
                    &source_label,
                    "OpenAPI components.schemas must be an object",
                )
            })?;

        let mut operations = Vec::new();
        for (route, path_item) in paths {
            let object = path_item.as_object().ok_or_else(|| {
                fail(
                    &source_label,
                    &format!("path item {route:?} must be an object"),
                )
            })?;
            for method in METHODS {
                let Some(value) = object.get(*method) else {
                    continue;
                };
                let operation = value.as_object().ok_or_else(|| {
                    fail(
                        &source_label,
                        &format!("{method} {route} operation must be an object"),
                    )
                })?;
                let operation_id = operation
                    .get("operationId")
                    .and_then(Value::as_str)
                    .ok_or_else(|| {
                        fail(
                            &source_label,
                            &format!("{method} {route} is missing a string operationId"),
                        )
                    })?;
                validate_identifier(operation_id, "operationId")
                    .map_err(|error| fail(&source_label, &error.to_string()))?;
                operations.push(Operation {
                    method: method.to_uppercase(),
                    path: route.clone(),
                    id: operation_id.to_owned(),
                    value: value.clone(),
                    path_item: path_item.clone(),
                });
            }
        }

        let mut model_map = BTreeMap::new();
        for (name, schema) in schemas {
            validate_identifier(name, "schema name")
                .map_err(|error| fail(&source_label, &error.to_string()))?;
            validate_schema_shape(
                &source_label,
                schema,
                &format!("#/components/schemas/{name}"),
            )?;
            model_map.insert(name.clone(), schema.clone());
        }
        validate_references(&source_label, &document)?;

        Ok(Self {
            id: input.id.clone(),
            label: input.label.clone(),
            source: input.path.display().to_string(),
            document,
            operations,
            schemas: model_map,
        })
    }

    pub fn resolve_ref(&self, reference: &str) -> Option<&Value> {
        reference.strip_prefix('#').and_then(|pointer| {
            if pointer.is_empty() {
                Some(&self.document)
            } else {
                self.document.pointer(pointer)
            }
        })
    }
}

pub(crate) fn ensure_unique_operations(contracts: &[Contract]) -> Result<(), Error> {
    let mut ids = BTreeSet::new();
    for contract in contracts {
        for operation in &contract.operations {
            if !ids.insert(operation.id.as_str()) {
                return Err(fail(
                    &contract.id,
                    &format!(
                        "duplicate operationId {:?} across selected contracts",
                        operation.id
                    ),
                ));
            }
        }
    }
    Ok(())
}

fn validate_schema_shape(contract_id: &str, schema: &Value, pointer: &str) -> Result<(), Error> {
    let Some(object) = schema.as_object() else {
        return Err(fail(
            contract_id,
            &format!("schema at {pointer} must be an object"),
        ));
    };
    if let Some(properties) = object.get("properties") {
        let properties = properties.as_object().ok_or_else(|| {
            fail(
                contract_id,
                &format!("schema properties at {pointer}/properties must be an object"),
            )
        })?;
        for (name, property) in properties {
            let child = format!(
                "{pointer}/properties/{}",
                name.replace('~', "~0").replace('/', "~1")
            );
            validate_schema_shape(contract_id, property, &child)?;
        }
    }
    if let Some(items) = object.get("items") {
        validate_schema_shape(contract_id, items, &format!("{pointer}/items"))?;
    }
    if let Some(additional) = object.get("additionalProperties")
        && !additional.is_boolean()
    {
        validate_schema_shape(
            contract_id,
            additional,
            &format!("{pointer}/additionalProperties"),
        )?;
    }
    for keyword in ["allOf", "oneOf", "anyOf"] {
        if let Some(composition) = object.get(keyword) {
            let alternatives = composition.as_array().ok_or_else(|| {
                fail(
                    contract_id,
                    &format!("schema {keyword} at {pointer}/{keyword} must be an array"),
                )
            })?;
            for (index, alternative) in alternatives.iter().enumerate() {
                validate_schema_shape(
                    contract_id,
                    alternative,
                    &format!("{pointer}/{keyword}/{index}"),
                )?;
            }
        }
    }
    Ok(())
}

fn validate_references(contract_id: &str, document: &Value) -> Result<(), Error> {
    let mut references = Vec::new();
    collect_references(document, &mut references);
    for (pointer, reference) in references {
        if !reference.starts_with('#') {
            return Err(fail(
                contract_id,
                &format!("external reference {reference:?} at {pointer} is unsupported"),
            ));
        }
        if reference != "#" && !reference.starts_with("#/") {
            return Err(fail(
                contract_id,
                &format!("invalid local reference {reference:?} at {pointer}"),
            ));
        }
        let target = if reference == "#" {
            Some(document)
        } else {
            document.pointer(&reference[1..])
        };
        if target.is_none() {
            return Err(fail(
                contract_id,
                &format!("local reference {reference:?} at {pointer} has no target"),
            ));
        }
    }
    Ok(())
}

fn collect_references(value: &Value, references: &mut Vec<(String, String)>) {
    fn walk(value: &Value, pointer: &str, refs: &mut Vec<(String, String)>) {
        match value {
            Value::Object(object) => {
                if let Some(Value::String(reference)) = object.get("$ref") {
                    refs.push((pointer.to_owned(), reference.clone()));
                }
                for (key, child) in object {
                    walk(
                        child,
                        &format!("{pointer}/{}", key.replace('~', "~0").replace('/', "~1")),
                        refs,
                    );
                }
            }
            Value::Array(items) => {
                for (index, child) in items.iter().enumerate() {
                    walk(child, &format!("{pointer}/{index}"), refs);
                }
            }
            _ => {}
        }
    }
    walk(value, "#", references);
}

fn fail(contract_id: &str, message: &str) -> Error {
    Error::Invalid(format!("contract {contract_id}: {message}"))
}
