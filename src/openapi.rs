use std::collections::BTreeSet;
use std::path::Path;

use serde_json::Value;

use crate::Error;
use crate::config::{ContractInput, validate_identifier};
use crate::input::{InputFormat, input_format, read_contract};
use crate::schema_resources::SchemaTarget;
use crate::schema_resources::{SchemaResources, document_uri};

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
    pub schemas: serde_json::Map<String, Value>,
    pub schema_resources: SchemaResources,
    pub is_openapi_31: bool,
    pub input_format: InputFormat,
}

#[derive(Debug)]
pub(crate) struct Operation {
    pub method: String,
    pub path: String,
    pub id: String,
    pub kind: OperationKind,
    pub pointer: String,
    pub path_item_pointer: String,
    pub value: Value,
    pub path_item: Value,
}

#[derive(Debug, Clone)]
pub(crate) enum OperationKind {
    Path,
    Webhook { name: String },
    Callback { name: String, expression: String },
}

pub(crate) struct ResolvedObject<'a> {
    pub value: &'a Value,
    pub pointer: String,
    pub chain: Vec<String>,
}

impl Contract {
    pub fn load(input: &ContractInput, root: &Path) -> Result<Self, Error> {
        let path = root.join(&input.path);
        let source_label = format!("{} ({})", input.id, input.path.display());
        let input_format = input_format(&path)?;
        let document = read_contract(&path)?;
        let root_object = document
            .as_object()
            .ok_or_else(|| fail(&source_label, "OpenAPI root must be an object"))?;
        let version = root_object
            .get("openapi")
            .and_then(Value::as_str)
            .unwrap_or_default();
        if !(version.starts_with("3.0.") || matches!(version, "3.1.0" | "3.1.1")) {
            return Err(fail(
                &source_label,
                &format!(
                    "unsupported OpenAPI version {version:?}; expected 3.0.x, 3.1.0, or 3.1.1"
                ),
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
            collect_operations(
                &source_label,
                route,
                path_item,
                OperationKind::Path,
                "paths",
                route,
                &mut operations,
            )?;
        }
        if let Some(webhooks) = root_object.get("webhooks").and_then(Value::as_object) {
            for (name, path_item) in webhooks {
                collect_operations(
                    &source_label,
                    name,
                    path_item,
                    OperationKind::Webhook { name: name.clone() },
                    "webhooks",
                    name,
                    &mut operations,
                )?;
            }
        }

        let mut model_map = serde_json::Map::new();
        for (name, schema) in schemas {
            validate_identifier(name, "schema name")
                .map_err(|error| fail(&source_label, &error.to_string()))?;
            model_map.insert(name.clone(), schema.clone());
        }
        let entry_uri = document_uri(&path, input.document_uri.as_deref())?;
        let schema_resources =
            SchemaResources::build(&document, entry_uri, version, &source_label)?;
        validate_references(&source_label, &document)?;
        let is_openapi_31 = version.starts_with("3.1.");

        Ok(Self {
            id: input.id.clone(),
            label: input.label.clone(),
            source: input.path.display().to_string(),
            document,
            operations,
            schemas: model_map,
            schema_resources,
            is_openapi_31,
            input_format,
        })
    }

    pub fn resolve_schema_reference(
        &self,
        origin: &str,
        reference: &str,
        dynamic: bool,
        dynamic_scope: &[String],
    ) -> Result<SchemaTarget, Error> {
        self.schema_resources
            .resolve(&self.document, origin, reference, dynamic, dynamic_scope)
            .map_err(|reason| {
                fail(
                    &self.id,
                    &format!("schema reference {reference:?} at {origin}: {reason}"),
                )
            })
    }

    pub fn resolve_component_object<'a>(
        &'a self,
        value: &'a Value,
        origin: &str,
        component: &str,
    ) -> Result<ResolvedObject<'a>, Error> {
        let mut current = value;
        let mut pointer = origin.to_owned();
        let mut chain = vec![origin.to_owned()];
        let mut visited = BTreeSet::new();
        loop {
            let Some(object) = current.as_object() else {
                return Err(fail(
                    &self.id,
                    &format!("{component} object at {pointer} must be an object"),
                ));
            };
            let Some(reference_value) = object.get("$ref") else {
                match component {
                    "parameters"
                        if current.get("name").and_then(Value::as_str).is_none()
                            || current.get("in").and_then(Value::as_str).is_none() =>
                    {
                        return Err(fail(
                            &self.id,
                            &format!(
                                "parameter object at {pointer} needs string name and in fields"
                            ),
                        ));
                    }
                    "requestBodies" if !current.get("content").is_some_and(Value::is_object) => {
                        return Err(fail(
                            &self.id,
                            &format!(
                                "request-body object at {pointer} needs an object content field"
                            ),
                        ));
                    }
                    "responses" if current.get("description").and_then(Value::as_str).is_none() => {
                        return Err(fail(
                            &self.id,
                            &format!(
                                "response object at {pointer} needs a string description field"
                            ),
                        ));
                    }
                    _ => {}
                }
                return Ok(ResolvedObject {
                    value: current,
                    pointer,
                    chain,
                });
            };
            let Some(reference) = reference_value.as_str() else {
                return Err(fail(
                    &self.id,
                    &format!("Reference Object $ref at {pointer} must be a string"),
                ));
            };
            let prefix = format!("#/components/{component}/");
            if !reference.starts_with(&prefix) {
                return Err(fail(
                    &self.id,
                    &format!(
                        "{component} reference {reference:?} at {pointer} targets an unsupported object"
                    ),
                ));
            }
            let target_pointer = reference[1..].to_owned();
            if !visited.insert(target_pointer.clone()) {
                return Err(fail(
                    &self.id,
                    &format!("cyclic {component} reference chain at {pointer}: {reference:?}"),
                ));
            }
            current = self.document.pointer(&target_pointer).ok_or_else(|| {
                fail(
                    &self.id,
                    &format!("local reference {reference:?} at {pointer} has no target"),
                )
            })?;
            pointer = format!("#{target_pointer}");
            chain.push(pointer.clone());
        }
    }
}

fn collect_operations(
    source_label: &str,
    route: &str,
    path_item: &Value,
    kind: OperationKind,
    collection: &str,
    key: &str,
    operations: &mut Vec<Operation>,
) -> Result<(), Error> {
    let object = path_item.as_object().ok_or_else(|| {
        fail(
            source_label,
            &format!("path item {route:?} must be an object"),
        )
    })?;
    let path_item_pointer = format!("#/{collection}/{}", pointer_escape(key));
    if object.contains_key("$ref") {
        return Err(fail(
            source_label,
            &format!("path-item $ref is unsupported at {path_item_pointer}"),
        ));
    }
    for (method, value) in object {
        if !METHODS.contains(&method.as_str()) {
            continue;
        }
        let operation = value.as_object().ok_or_else(|| {
            fail(
                source_label,
                &format!("{method} {route} operation must be an object"),
            )
        })?;
        let operation_id = match (&kind, operation.get("operationId").and_then(Value::as_str)) {
            (OperationKind::Webhook { name }, None) => format!("webhook-{name}-{}", method),
            (_, Some(id)) => id.to_owned(),
            (_, None) => {
                return Err(fail(
                    source_label,
                    &format!("{method} {route} is missing a string operationId"),
                ));
            }
        };
        validate_identifier(&operation_id, "operationId")
            .map_err(|error| fail(source_label, &error.to_string()))?;
        operations.push(Operation {
            method: method.to_uppercase(),
            path: route.to_owned(),
            id: operation_id,
            kind: kind.clone(),
            pointer: format!("{path_item_pointer}/{method}"),
            path_item_pointer: path_item_pointer.clone(),
            value: value.clone(),
            path_item: path_item.clone(),
        });
    }
    Ok(())
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

fn validate_references(contract_id: &str, document: &Value) -> Result<(), Error> {
    let mut walker = ReferenceWalker {
        contract_id,
        document,
    };
    walker.document()
}

struct ReferenceWalker<'a> {
    contract_id: &'a str,
    document: &'a Value,
}

impl ReferenceWalker<'_> {
    fn document(&mut self) -> Result<(), Error> {
        if let Some(paths) = self.document.get("paths").and_then(Value::as_object) {
            for (path, item) in paths {
                self.path_item(item, &format!("#/paths/{}", pointer_escape(path)))?;
            }
        }
        if let Some(components) = self.document.get("components").and_then(Value::as_object) {
            for (kind, entries) in components {
                let Some(entries) = entries.as_object() else {
                    continue;
                };
                for (name, value) in entries {
                    let pointer = format!(
                        "#/components/{}/{}",
                        pointer_escape(kind),
                        pointer_escape(name)
                    );
                    match kind.as_str() {
                        "schemas" => self.schema(value, &pointer)?,
                        "parameters" => self.parameter(value, &pointer)?,
                        "headers" => self.header(value, &pointer)?,
                        "requestBodies" => self.request_body(value, &pointer)?,
                        "responses" => self.response(value, &pointer)?,
                        "examples" => self.example(value, &pointer)?,
                        "callbacks" => self.callback(value, &pointer)?,
                        "links" => self.link(value, &pointer)?,
                        _ => {}
                    }
                }
            }
        }
        Ok(())
    }

    fn reference(&self, value: &Value, pointer: &str) -> Result<bool, Error> {
        let Some(object) = value.as_object() else {
            return Ok(false);
        };
        let Some(value) = object.get("$ref") else {
            return Ok(false);
        };
        let Some(reference) = value.as_str() else {
            return Err(fail(
                self.contract_id,
                &format!("Reference Object $ref at {pointer} must be a string"),
            ));
        };
        if !reference.starts_with('#') {
            return Err(fail(
                self.contract_id,
                &format!("external reference {reference:?} at {pointer} is unsupported"),
            ));
        }
        if reference != "#" && !reference.starts_with("#/") {
            return Err(fail(
                self.contract_id,
                &format!("invalid local reference {reference:?} at {pointer}"),
            ));
        }
        let target = if reference == "#" {
            Some(self.document)
        } else {
            self.document.pointer(&reference[1..])
        };
        let Some(target) = target else {
            return Err(fail(
                self.contract_id,
                &format!("local reference {reference:?} at {pointer} has no target"),
            ));
        };
        if !target.is_object() {
            return Err(fail(
                self.contract_id,
                &format!("Reference Object {reference:?} at {pointer} must target an object"),
            ));
        }
        Ok(true)
    }

    fn path_item(&mut self, value: &Value, pointer: &str) -> Result<(), Error> {
        if !value.is_object() {
            return Ok(());
        }
        self.reference(value, pointer)?;
        if let Some(parameters) = value.get("parameters").and_then(Value::as_array) {
            for (index, parameter) in parameters.iter().enumerate() {
                self.parameter(parameter, &format!("{pointer}/parameters/{index}"))?;
            }
        }
        for method in METHODS {
            if let Some(operation) = value.get(*method) {
                self.operation(operation, &format!("{pointer}/{method}"))?;
            }
        }
        Ok(())
    }

    fn operation(&mut self, value: &Value, pointer: &str) -> Result<(), Error> {
        if let Some(parameters) = value.get("parameters").and_then(Value::as_array) {
            for (index, parameter) in parameters.iter().enumerate() {
                self.parameter(parameter, &format!("{pointer}/parameters/{index}"))?;
            }
        }
        if let Some(body) = value.get("requestBody") {
            self.request_body(body, &format!("{pointer}/requestBody"))?;
        }
        if let Some(responses) = value.get("responses").and_then(Value::as_object) {
            for (status, response) in responses {
                self.response(
                    response,
                    &format!("{pointer}/responses/{}", pointer_escape(status)),
                )?;
            }
        }
        if let Some(callbacks) = value.get("callbacks").and_then(Value::as_object) {
            for (name, callback) in callbacks {
                self.callback(
                    callback,
                    &format!("{pointer}/callbacks/{}", pointer_escape(name)),
                )?;
            }
        }
        Ok(())
    }

    fn parameter(&mut self, value: &Value, pointer: &str) -> Result<(), Error> {
        if self.reference(value, pointer)? {
            return Ok(());
        }
        if let Some(schema) = value.get("schema") {
            self.schema(schema, &format!("{pointer}/schema"))?;
        }
        if let Some(content) = value.get("content").and_then(Value::as_object) {
            self.content(content, &format!("{pointer}/content"))?;
        }
        if let Some(examples) = value.get("examples").and_then(Value::as_object) {
            for (name, example) in examples {
                self.example(
                    example,
                    &format!("{pointer}/examples/{}", pointer_escape(name)),
                )?;
            }
        }
        Ok(())
    }

    fn header(&mut self, value: &Value, pointer: &str) -> Result<(), Error> {
        self.parameter(value, pointer)
    }

    fn request_body(&mut self, value: &Value, pointer: &str) -> Result<(), Error> {
        if self.reference(value, pointer)? {
            return Ok(());
        }
        if let Some(content) = value.get("content").and_then(Value::as_object) {
            self.content(content, &format!("{pointer}/content"))?;
        }
        Ok(())
    }

    fn response(&mut self, value: &Value, pointer: &str) -> Result<(), Error> {
        if self.reference(value, pointer)? {
            return Ok(());
        }
        if let Some(headers) = value.get("headers").and_then(Value::as_object) {
            for (name, header) in headers {
                self.header(
                    header,
                    &format!("{pointer}/headers/{}", pointer_escape(name)),
                )?;
            }
        }
        if let Some(content) = value.get("content").and_then(Value::as_object) {
            self.content(content, &format!("{pointer}/content"))?;
        }
        if let Some(links) = value.get("links").and_then(Value::as_object) {
            for (name, link) in links {
                self.link(link, &format!("{pointer}/links/{}", pointer_escape(name)))?;
            }
        }
        Ok(())
    }

    fn content(
        &mut self,
        value: &serde_json::Map<String, Value>,
        pointer: &str,
    ) -> Result<(), Error> {
        for (media_type, media) in value {
            let media_pointer = format!("{pointer}/{}", pointer_escape(media_type));
            if let Some(schema) = media.get("schema") {
                self.schema(schema, &format!("{media_pointer}/schema"))?;
            }
            if let Some(examples) = media.get("examples").and_then(Value::as_object) {
                for (name, example) in examples {
                    self.example(
                        example,
                        &format!("{media_pointer}/examples/{}", pointer_escape(name)),
                    )?;
                }
            }
        }
        Ok(())
    }

    fn example(&self, value: &Value, pointer: &str) -> Result<(), Error> {
        self.reference(value, pointer).map(|_| ())
    }

    fn callback(&mut self, value: &Value, pointer: &str) -> Result<(), Error> {
        if self.reference(value, pointer)? {
            return Ok(());
        }
        let Some(object) = value.as_object() else {
            return Ok(());
        };
        for (expression, item) in object.iter().filter(|(key, _)| !key.starts_with("x-")) {
            self.path_item(item, &format!("{pointer}/{}", pointer_escape(expression)))?;
        }
        Ok(())
    }

    fn link(&self, value: &Value, pointer: &str) -> Result<(), Error> {
        self.reference(value, pointer).map(|_| ())
    }

    fn schema(&mut self, value: &Value, pointer: &str) -> Result<(), Error> {
        // Schema references are validated by SchemaResources, which tracks
        // resource roots, anchors and lexical dynamic scope. This walker is
        // retained for the non-schema Reference Object categories only.
        let _ = (value, pointer);
        Ok(())
    }
}

fn pointer_escape(value: &str) -> String {
    value.replace('~', "~0").replace('/', "~1")
}

fn fail(contract_id: &str, message: &str) -> Error {
    Error::Invalid(format!("contract {contract_id}: {message}"))
}
