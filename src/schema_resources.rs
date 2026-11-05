//! Local schema-resource and dynamic-anchor indexing for one OpenAPI input.
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use percent_encoding::percent_decode_str;
use serde_json::Value;
use url::Url;

use crate::Error;

const OAS_DIALECT: &str = "https://spec.openapis.org/oas/3.1/dialect/base";
const JSON_SCHEMA_2020_12: &str = "https://json-schema.org/draft/2020-12/schema";

#[derive(Debug, Clone)]
pub(crate) struct SchemaTarget {
    pub pointer: String,
    pub uri: String,
    pub dynamic: bool,
}

#[derive(Debug, Default)]
struct Resource {
    root: String,
    anchors: BTreeMap<String, String>,
    dynamic_anchors: BTreeMap<String, String>,
}

#[derive(Debug)]
pub(crate) struct SchemaResources {
    entry_uri: String,
    resources: BTreeMap<String, Resource>,
    nearest_resource: BTreeMap<String, String>,
    schema_pointers: BTreeSet<String>,
    declared_ids: BTreeMap<String, String>,
    version_31: bool,
}

impl SchemaResources {
    pub fn build(
        document: &Value,
        entry_uri: String,
        version: &str,
        label: &str,
    ) -> Result<Self, Error> {
        let mut index = Self {
            entry_uri: entry_uri.clone(),
            resources: BTreeMap::from([(
                entry_uri.clone(),
                Resource {
                    root: "#".to_owned(),
                    ..Resource::default()
                },
            )]),
            nearest_resource: BTreeMap::new(),
            schema_pointers: BTreeSet::new(),
            declared_ids: BTreeMap::new(),
            version_31: version.starts_with("3.1."),
        };
        let Some(root) = document.as_object() else {
            return Err(invalid(label, "OpenAPI root must be an object"));
        };
        if let Some(value) = root.get("jsonSchemaDialect") {
            let dialect = value
                .as_str()
                .ok_or_else(|| invalid(label, "jsonSchemaDialect must be a string"))?;
            validate_dialect(dialect)
                .map_err(|reason| invalid(label, &format!("jsonSchemaDialect: {reason}")))?;
        }
        if let Some(paths) = root.get("paths").and_then(Value::as_object) {
            for (path, item) in paths {
                index.path_item(
                    item,
                    &format!("#/paths/{}", escape(path)),
                    &entry_uri,
                    version,
                    label,
                )?;
            }
        }
        if let Some(webhooks) = root.get("webhooks").and_then(Value::as_object) {
            for (name, item) in webhooks {
                index.path_item(
                    item,
                    &format!("#/webhooks/{}", escape(name)),
                    &entry_uri,
                    version,
                    label,
                )?;
            }
        }
        if let Some(components) = root.get("components").and_then(Value::as_object) {
            if let Some(entries) = components.get("schemas").and_then(Value::as_object) {
                for (name, schema) in entries {
                    index.index_schema(
                        schema,
                        &format!("#/components/schemas/{}", escape(name)),
                        &entry_uri,
                        version,
                        label,
                    )?;
                }
            }
            for (kind, entries) in components {
                let Some(entries) = entries.as_object() else {
                    continue;
                };
                match kind.as_str() {
                    "parameters" | "headers" => {
                        for (name, item) in entries {
                            index.parameter(
                                item,
                                &format!("#/components/{kind}/{}", escape(name)),
                                &entry_uri,
                                version,
                                label,
                            )?;
                        }
                    }
                    "requestBodies" => {
                        for (name, item) in entries {
                            index.content(
                                item.get("content"),
                                &format!("#/components/requestBodies/{}/content", escape(name)),
                                &entry_uri,
                                version,
                                label,
                            )?;
                        }
                    }
                    "responses" => {
                        for (name, item) in entries {
                            index.response(
                                item,
                                &format!("#/components/responses/{}", escape(name)),
                                &entry_uri,
                                version,
                                label,
                            )?;
                        }
                    }
                    "callbacks" => {
                        for (name, item) in entries {
                            index.callbacks(
                                item,
                                &format!("#/components/callbacks/{}", escape(name)),
                                &entry_uri,
                                version,
                                label,
                            )?;
                        }
                    }
                    _ => {}
                }
            }
        }
        index.validate_schema_references(document, label)?;
        Ok(index)
    }

    pub fn schema_at<'a>(&self, document: &'a Value, pointer: &str) -> Option<&'a Value> {
        if !self.schema_pointers.contains(pointer) {
            return None;
        }
        document.pointer(pointer.strip_prefix('#')?)
    }

    pub fn lexical_scope(&self, pointer: &str) -> Vec<String> {
        let mut scope = vec![self.entry_uri.clone()];
        let mut candidate = String::from("#");
        for part in pointer.strip_prefix("#/").unwrap_or_default().split('/') {
            candidate.push('/');
            candidate.push_str(part);
            if let Some(uri) = self.nearest_resource.get(&candidate)
                && !scope.contains(uri)
            {
                scope.push(uri.clone());
            }
        }
        scope
    }

    pub fn resource_for_pointer(&self, pointer: &str) -> String {
        self.nearest_resource
            .get(pointer)
            .cloned()
            .unwrap_or_else(|| self.entry_uri.clone())
    }

    pub fn resolve(
        &self,
        document: &Value,
        origin: &str,
        reference: &str,
        dynamic: bool,
        inherited_scope: &[String],
    ) -> Result<SchemaTarget, String> {
        let base = self.nearest_resource.get(origin).unwrap_or(&self.entry_uri);
        let (target_uri, fragment) = resolve_reference(base, reference)?;
        let resource = self
            .resources
            .get(&target_uri)
            .ok_or_else(|| format!("resource {target_uri:?} is not registered in this input"))?;
        let decoded = decode_fragment(&fragment)?;
        let mut pointer = if decoded.is_empty() {
            resource.root.clone()
        } else if decoded.starts_with('/') {
            if resource.root == "#" {
                format!("#{decoded}")
            } else {
                format!("{}{decoded}", resource.root)
            }
        } else {
            resource
                .anchors
                .get(&decoded)
                .or_else(|| resource.dynamic_anchors.get(&decoded))
                .cloned()
                .ok_or_else(|| {
                    format!("anchor {decoded:?} is missing from resource {target_uri:?}")
                })?
        };
        let mut dynamically_bound = false;
        if dynamic
            && !decoded.is_empty()
            && !decoded.starts_with('/')
            && resource.dynamic_anchors.contains_key(&decoded)
        {
            let scope = if inherited_scope.is_empty() {
                self.lexical_scope(origin)
            } else {
                inherited_scope.to_vec()
            };
            for scope_uri in scope {
                if let Some(target) = self
                    .resources
                    .get(&scope_uri)
                    .and_then(|candidate| candidate.dynamic_anchors.get(&decoded))
                {
                    if *target != pointer {
                        pointer = target.clone();
                        dynamically_bound = true;
                    }
                    break;
                }
            }
            // A dynamicRef with no matching outer binding uses its static target.
            if !dynamically_bound {
                pointer = resource.dynamic_anchors[&decoded].clone();
            }
        }
        if !self.schema_pointers.contains(&pointer) || self.schema_at(document, &pointer).is_none()
        {
            return Err(format!(
                "reference {reference:?} does not target a registered schema"
            ));
        }
        let uri = if dynamically_bound {
            self.resource_for_pointer(&pointer)
        } else {
            target_uri
        };
        Ok(SchemaTarget {
            pointer,
            uri,
            dynamic: dynamically_bound,
        })
    }

    fn path_item(
        &mut self,
        item: &Value,
        pointer: &str,
        base: &str,
        version: &str,
        label: &str,
    ) -> Result<(), Error> {
        let Some(object) = item.as_object() else {
            return Ok(());
        };
        if let Some(parameters) = object.get("parameters").and_then(Value::as_array) {
            for (index, parameter) in parameters.iter().enumerate() {
                self.parameter(
                    parameter,
                    &format!("{pointer}/parameters/{index}"),
                    base,
                    version,
                    label,
                )?;
            }
        }
        for (method, operation) in object {
            if ![
                "get", "put", "post", "delete", "options", "head", "patch", "trace",
            ]
            .contains(&method.as_str())
            {
                continue;
            }
            let op = format!("{pointer}/{method}");
            if let Some(parameters) = operation.get("parameters").and_then(Value::as_array) {
                for (index, parameter) in parameters.iter().enumerate() {
                    self.parameter(
                        parameter,
                        &format!("{op}/parameters/{index}"),
                        base,
                        version,
                        label,
                    )?;
                }
            }
            if let Some(body) = operation.get("requestBody") {
                self.content(
                    body.get("content"),
                    &format!("{op}/requestBody/content"),
                    base,
                    version,
                    label,
                )?;
            }
            if let Some(responses) = operation.get("responses").and_then(Value::as_object) {
                for (status, response) in responses {
                    self.response(
                        response,
                        &format!("{op}/responses/{}", escape(status)),
                        base,
                        version,
                        label,
                    )?;
                }
            }
            if let Some(callbacks) = operation.get("callbacks") {
                self.callbacks(callbacks, &format!("{op}/callbacks"), base, version, label)?;
            }
        }
        Ok(())
    }

    fn parameter(
        &mut self,
        value: &Value,
        pointer: &str,
        base: &str,
        version: &str,
        label: &str,
    ) -> Result<(), Error> {
        if value.get("$ref").is_some() {
            return Ok(());
        }
        if let Some(schema) = value.get("schema") {
            self.index_schema(schema, &format!("{pointer}/schema"), base, version, label)?;
        }
        self.content(
            value.get("content"),
            &format!("{pointer}/content"),
            base,
            version,
            label,
        )
    }

    fn response(
        &mut self,
        value: &Value,
        pointer: &str,
        base: &str,
        version: &str,
        label: &str,
    ) -> Result<(), Error> {
        if value.get("$ref").is_some() {
            return Ok(());
        }
        if let Some(headers) = value.get("headers").and_then(Value::as_object) {
            for (name, header) in headers {
                self.parameter(
                    header,
                    &format!("{pointer}/headers/{}", escape(name)),
                    base,
                    version,
                    label,
                )?;
            }
        }
        self.content(
            value.get("content"),
            &format!("{pointer}/content"),
            base,
            version,
            label,
        )
    }

    fn content(
        &mut self,
        value: Option<&Value>,
        pointer: &str,
        base: &str,
        version: &str,
        label: &str,
    ) -> Result<(), Error> {
        if let Some(media) = value.and_then(Value::as_object) {
            for (media_type, item) in media {
                if let Some(schema) = item.get("schema") {
                    self.index_schema(
                        schema,
                        &format!("{pointer}/{}/schema", escape(media_type)),
                        base,
                        version,
                        label,
                    )?;
                }
            }
        }
        Ok(())
    }

    fn callbacks(
        &mut self,
        value: &Value,
        pointer: &str,
        base: &str,
        version: &str,
        label: &str,
    ) -> Result<(), Error> {
        if value.get("$ref").is_some() {
            return Ok(());
        }
        let Some(callbacks) = value.as_object() else {
            return Ok(());
        };
        for (name, callback) in callbacks {
            let Some(expressions) = callback.as_object() else {
                continue;
            };
            for (expression, item) in expressions.iter().filter(|(key, _)| !key.starts_with("x-")) {
                self.path_item(
                    item,
                    &format!("{pointer}/{}/{}", escape(name), escape(expression)),
                    base,
                    version,
                    label,
                )?;
            }
        }
        Ok(())
    }

    fn index_schema(
        &mut self,
        value: &Value,
        pointer: &str,
        parent_uri: &str,
        version: &str,
        label: &str,
    ) -> Result<(), Error> {
        let boolean_allowed =
            version.starts_with("3.1.") || pointer.ends_with("/additionalProperties");
        if !(value.is_object() || boolean_allowed && value.is_boolean()) {
            return Err(invalid(
                label,
                &format!(
                    "schema at {pointer} must be an object{}",
                    if version.starts_with("3.1.") {
                        " or boolean"
                    } else {
                        ""
                    }
                ),
            ));
        }
        if !value.is_object() {
            self.schema_pointers.insert(pointer.to_owned());
            self.nearest_resource
                .insert(pointer.to_owned(), parent_uri.to_owned());
            return Ok(());
        }
        let mut resource_uri = parent_uri.to_owned();
        if self.version_31
            && let Some(id) = value.get("$id")
        {
            let id = id
                .as_str()
                .ok_or_else(|| invalid(label, &format!("$id at {pointer} must be a string")))?;
            resource_uri = resolve_id(parent_uri, id)
                .map_err(|reason| invalid(label, &format!("$id at {pointer}: {reason}")))?;
            if let Some(previous) = self.declared_ids.get(&resource_uri)
                && previous != pointer
            {
                return Err(invalid(
                    label,
                    &format!(
                        "duplicate schema resource identifier {resource_uri:?} at {pointer}; first declared at {previous}"
                    ),
                ));
            }
            self.declared_ids
                .insert(resource_uri.clone(), pointer.to_owned());
            if let Some(existing) = self.resources.get(&resource_uri)
                && existing.root != pointer
            {
                return Err(invalid(
                    label,
                    &format!(
                        "duplicate schema resource identifier {resource_uri:?} at {pointer}; resource root is already {}",
                        existing.root
                    ),
                ));
            }
            if !self.resources.contains_key(&resource_uri) {
                self.resources.insert(
                    resource_uri.clone(),
                    Resource {
                        root: pointer.to_owned(),
                        ..Resource::default()
                    },
                );
            }
        }
        self.schema_pointers.insert(pointer.to_owned());
        self.nearest_resource
            .insert(pointer.to_owned(), resource_uri.clone());
        if !self.version_31 && value.get("$ref").is_some() {
            return Ok(());
        }
        for (keyword, dynamic) in [("$anchor", false), ("$dynamicAnchor", true)] {
            if !self.version_31 {
                break;
            }
            if let Some(anchor) = value.get(keyword) {
                let anchor = anchor.as_str().ok_or_else(|| {
                    invalid(label, &format!("{keyword} at {pointer} must be a string"))
                })?;
                if !valid_anchor(anchor) {
                    return Err(invalid(
                        label,
                        &format!("invalid {keyword} {anchor:?} at {pointer}"),
                    ));
                }
                let resource = self
                    .resources
                    .get_mut(&resource_uri)
                    .ok_or_else(|| invalid(label, "schema resource index is missing"))?;
                if (dynamic && resource.anchors.contains_key(anchor))
                    || (!dynamic && resource.dynamic_anchors.contains_key(anchor))
                {
                    return Err(invalid(
                        label,
                        &format!("duplicate anchor name {anchor:?} in resource {resource_uri:?}"),
                    ));
                }
                let anchors = if dynamic {
                    &mut resource.dynamic_anchors
                } else {
                    &mut resource.anchors
                };
                if anchors
                    .insert(anchor.to_owned(), pointer.to_owned())
                    .is_some()
                {
                    return Err(invalid(
                        label,
                        &format!("duplicate {keyword} {anchor:?} in resource {resource_uri:?}"),
                    ));
                }
            }
        }
        if self.version_31
            && let Some(dialect) = value.get("$schema")
        {
            let dialect = dialect
                .as_str()
                .ok_or_else(|| invalid(label, &format!("$schema at {pointer} must be a string")))?;
            validate_dialect(dialect)
                .map_err(|reason| invalid(label, &format!("$schema at {pointer}: {reason}")))?;
        }
        for key in [
            "properties",
            "patternProperties",
            "dependentSchemas",
            "$defs",
        ] {
            if !self.version_31 && key != "properties" {
                continue;
            }
            if value.get(key).is_some_and(|child| !child.is_object()) {
                return Err(invalid(
                    label,
                    &format!("schema {key} at {pointer}/{key} must be an object"),
                ));
            }
        }
        for key in ["allOf", "anyOf", "oneOf", "prefixItems"] {
            if key == "prefixItems" && !self.version_31 {
                continue;
            }
            if value.get(key).is_some_and(|child| !child.is_array()) {
                return Err(invalid(
                    label,
                    &format!("schema {key} at {pointer}/{key} must be an array"),
                ));
            }
        }
        for key in ["properties", "patternProperties", "dependentSchemas"] {
            if !self.version_31 && key != "properties" {
                continue;
            }
            if let Some(children) = value.get(key).and_then(Value::as_object) {
                for (name, child) in children {
                    self.index_schema(
                        child,
                        &format!("{pointer}/{key}/{}", escape(name)),
                        &resource_uri,
                        version,
                        label,
                    )?;
                }
            }
        }
        if self.version_31
            && let Some(definitions) = value.get("$defs").and_then(Value::as_object)
        {
            for (name, child) in definitions {
                self.index_schema(
                    child,
                    &format!("{pointer}/$defs/{}", escape(name)),
                    &resource_uri,
                    version,
                    label,
                )?;
            }
        }
        for key in [
            "items",
            "additionalProperties",
            "not",
            "contains",
            "propertyNames",
            "if",
            "then",
            "else",
        ] {
            if !self.version_31 && !matches!(key, "items" | "additionalProperties") {
                continue;
            }
            if let Some(child) = value.get(key) {
                self.index_schema(
                    child,
                    &format!("{pointer}/{key}"),
                    &resource_uri,
                    version,
                    label,
                )?;
            }
        }
        for key in ["allOf", "anyOf", "oneOf", "prefixItems"] {
            if key == "prefixItems" && !self.version_31 {
                continue;
            }
            if let Some(children) = value.get(key).and_then(Value::as_array) {
                for (index, child) in children.iter().enumerate() {
                    self.index_schema(
                        child,
                        &format!("{pointer}/{key}/{index}"),
                        &resource_uri,
                        version,
                        label,
                    )?;
                }
            }
        }
        Ok(())
    }

    fn validate_schema_references(&self, document: &Value, label: &str) -> Result<(), Error> {
        for pointer in &self.schema_pointers {
            let Some(schema) = self.schema_at(document, pointer) else {
                continue;
            };
            if let Some(object) = schema.as_object() {
                if !self.version_31 {
                    if let Some(reference_value) = object.get("$ref") {
                        let reference = reference_value.as_str().ok_or_else(|| {
                            invalid(label, &format!("$ref at {pointer} must be a string"))
                        })?;
                        if !reference.starts_with("#/") {
                            return Err(invalid(
                                label,
                                &format!(
                                    "external reference {reference:?} at {pointer} is unsupported"
                                ),
                            ));
                        }
                        let target = if reference == "#" {
                            Some(document)
                        } else if reference.starts_with("#/") {
                            document.pointer(&reference[1..])
                        } else {
                            None
                        }
                        .ok_or_else(|| {
                            invalid(
                                label,
                                &format!(
                                    "local reference {reference:?} at {pointer} has no target"
                                ),
                            )
                        })?;
                        if !target.is_object() {
                            return Err(invalid(
                                label,
                                &format!(
                                    "Reference Object {reference:?} at {pointer} must target an object"
                                ),
                            ));
                        }
                    }
                    continue;
                }
                for (key, dynamic) in [("$ref", false), ("$dynamicRef", true)] {
                    if let Some(reference) = object.get(key) {
                        let reference = reference.as_str().ok_or_else(|| {
                            invalid(label, &format!("{key} at {pointer} must be a string"))
                        })?;
                        self.resolve(document, pointer, reference, dynamic, &[])
                            .map_err(|reason| {
                                invalid(
                                    label,
                                    &format!("{key} {reference:?} at {pointer}: {reason}"),
                                )
                            })?;
                    }
                }
            }
        }
        Ok(())
    }
}

pub(crate) fn document_uri(path: &Path, explicit: Option<&str>) -> Result<String, Error> {
    if let Some(explicit) = explicit {
        return normalize_base_uri(explicit).map_err(Error::Invalid);
    }
    let canonical = path
        .canonicalize()
        .map_err(|error| Error::read(path, error))?;
    Url::from_file_path(&canonical)
        .map(|url| url.to_string())
        .map_err(|_| {
            Error::Invalid("cannot form an absolute file URI for contract input".to_owned())
        })
}

fn normalize_base_uri(value: &str) -> Result<String, String> {
    normalize_uri(value, false)
}

fn normalize_id_uri(value: &str) -> Result<String, String> {
    normalize_uri(value, true)
}

fn normalize_uri(value: &str, allow_empty_fragment: bool) -> Result<String, String> {
    validate_percent(value)?;
    if value
        .chars()
        .any(|c| c.is_whitespace() || c.is_control() || c == '\\')
    {
        return Err("URI contains whitespace, control characters, or backslash".to_owned());
    }
    let mut url = Url::parse(value).map_err(|error| format!("invalid absolute URI: {error}"))?;
    validate_url(&url)?;
    if url.query().is_some()
        || url
            .fragment()
            .is_some_and(|fragment| !allow_empty_fragment || !fragment.is_empty())
    {
        return Err(
            "document URI and $id may not contain a query or a disallowed fragment".to_owned(),
        );
    }
    url.set_fragment(None);
    Ok(url.to_string())
}

fn resolve_id(base: &str, id: &str) -> Result<String, String> {
    validate_percent(id)?;
    if id
        .chars()
        .any(|c| c.is_whitespace() || c.is_control() || c == '\\')
    {
        return Err("URI contains whitespace, control characters, or backslash".to_owned());
    }
    let base = Url::parse(base).map_err(|error| format!("invalid base URI: {error}"))?;
    let url = base
        .join(id)
        .map_err(|error| format!("cannot resolve URI: {error}"))?;
    normalize_id_uri(url.as_str())
}

fn resolve_reference(base: &str, reference: &str) -> Result<(String, String), String> {
    validate_percent(reference)?;
    if reference
        .chars()
        .any(|c| c.is_whitespace() || c.is_control() || c == '\\')
    {
        return Err("reference contains whitespace, control characters, or backslash".to_owned());
    }
    let base = Url::parse(base).map_err(|error| format!("invalid base URI: {error}"))?;
    let url = base
        .join(reference)
        .map_err(|error| format!("cannot resolve URI: {error}"))?;
    validate_url(&url)?;
    if url.query().is_some() {
        return Err("query-bearing schema references are unsupported".to_owned());
    }
    let target = {
        let mut url = url.clone();
        url.set_fragment(None);
        url.to_string()
    };
    Ok((target, url.fragment().unwrap_or_default().to_owned()))
}

fn validate_url(url: &Url) -> Result<(), String> {
    if !matches!(url.scheme(), "http" | "https" | "file") {
        return Err(format!("unsupported URI scheme {:?}", url.scheme()));
    }
    if !url.username().is_empty() || url.password().is_some() {
        return Err("URI userinfo is unsupported".to_owned());
    }
    Ok(())
}

fn decode_fragment(fragment: &str) -> Result<String, String> {
    validate_percent(fragment)?;
    percent_decode_str(fragment)
        .decode_utf8()
        .map(|value| value.into_owned())
        .map_err(|error| format!("fragment is not valid UTF-8: {error}"))
}

fn validate_percent(value: &str) -> Result<(), String> {
    let bytes = value.as_bytes();
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'%' {
            if index + 2 >= bytes.len()
                || !bytes[index + 1].is_ascii_hexdigit()
                || !bytes[index + 2].is_ascii_hexdigit()
            {
                return Err("malformed percent escape".to_owned());
            }
            index += 3;
        } else {
            index += 1;
        }
    }
    Ok(())
}

fn valid_anchor(value: &str) -> bool {
    let mut bytes = value.bytes();
    bytes.next().is_some_and(|byte| byte.is_ascii_alphabetic())
        && bytes.all(|byte| byte.is_ascii_alphanumeric() || b"-._:".contains(&byte))
}

fn validate_dialect(value: &str) -> Result<(), String> {
    if matches!(value, OAS_DIALECT | JSON_SCHEMA_2020_12) {
        Ok(())
    } else {
        Err(format!("unsupported JSON Schema dialect {value:?}"))
    }
}

fn escape(value: &str) -> String {
    value.replace('~', "~0").replace('/', "~1")
}
fn invalid(label: &str, message: &str) -> Error {
    Error::Invalid(format!("contract {label}: {message}"))
}

#[cfg(test)]
mod tests {
    use super::{SchemaResources, document_uri};
    use serde_json::json;
    use std::path::Path;

    fn dynamic_contract() -> (serde_json::Value, SchemaResources) {
        let document = json!({
            "openapi":"3.1.1",
            "paths":{},
            "components":{"schemas":{
                "Planet":{"type":"object","properties":{"id":{"type":"string"}}},
                "Outer":{
                    "$id":"https://example.test/schemas/outer",
                    "$defs":{"itemType":{"$dynamicAnchor":"itemType","$ref":"https://example.test/openapi.json#/components/schemas/Planet"}},
                    "$ref":"https://example.test/schemas/generic"
                },
                "Generic":{
                    "$id":"https://example.test/schemas/generic",
                    "$defs":{"itemType":{"$dynamicAnchor":"itemType","not":{}}},
                    "properties":{"data":{"type":"array","items":{"$dynamicRef":"#itemType"}}}
                }
            }}
        });
        let resources = SchemaResources::build(
            &document,
            "https://example.test/openapi.json".to_owned(),
            "3.1.1",
            "fixture",
        )
        .expect("valid same-input resources");
        (document, resources)
    }

    #[test]
    fn nested_resource_pointer_and_outer_dynamic_binding_are_distinct() {
        let (document, resources) = dynamic_contract();
        let generic_items = "#/components/schemas/Generic/properties/data/items";
        let inherited = vec![
            "https://example.test/openapi.json".to_owned(),
            "https://example.test/schemas/outer".to_owned(),
            "https://example.test/schemas/generic".to_owned(),
        ];
        let target = resources
            .resolve(&document, generic_items, "#itemType", true, &inherited)
            .expect("dynamic target resolves");
        assert_eq!(target.pointer, "#/components/schemas/Outer/$defs/itemType");
        assert!(target.dynamic);
        assert_eq!(target.uri, "https://example.test/schemas/outer");
        let standalone = resources
            .resolve(&document, generic_items, "#itemType", true, &[])
            .expect("generic default resolves");
        assert_eq!(
            standalone.pointer,
            "#/components/schemas/Generic/$defs/itemType"
        );
        assert!(!standalone.dynamic);
    }

    #[test]
    fn resource_pointers_anchors_and_opaque_examples_are_classified_separately() {
        let document = json!({
            "openapi":"3.1.1","paths":{},
            "components":{"schemas":{
                "Root":{"$id":"https://example.test/schema/root","$anchor":"named","properties":{"child":{"$id":"child","$anchor":"leaf","type":"string"}},"examples":[{"$ref":"https://example.test/literal"}]}
            }}
        });
        let resources = SchemaResources::build(
            &document,
            "https://example.test/openapi.json".to_owned(),
            "3.1.1",
            "fixture",
        )
        .expect("nested resource and opaque example");
        let anchor = resources
            .resolve(&document, "#/components/schemas/Root", "#named", false, &[])
            .expect("resource anchor");
        assert_eq!(anchor.pointer, "#/components/schemas/Root");
        let nested = resources
            .resolve(
                &document,
                "#/components/schemas/Root",
                "#/properties/child",
                false,
                &[],
            )
            .expect("resource-relative pointer");
        assert_eq!(nested.pointer, "#/components/schemas/Root/properties/child");
        assert!(
            resources
                .resolve(
                    &document,
                    "#/components/schemas/Root",
                    "https://example.test/literal",
                    false,
                    &[]
                )
                .is_err()
        );
    }

    #[test]
    fn duplicate_resource_ids_anchors_bad_fragments_and_external_resources_fail() {
        for document in [
            json!({"components":{"schemas":{"A":{"$id":"https://example.test/shared"},"B":{"$id":"https://example.test/shared"}}}}),
            json!({"components":{"schemas":{"A":{"$id":"https://example.test/r","$anchor":"same","properties":{"x":{"$anchor":"same"}}}}}}),
            json!({"components":{"schemas":{"A":{"$id":"https://example.test/openapi.json"}}}}),
        ] {
            assert!(
                SchemaResources::build(
                    &document,
                    "https://example.test/openapi.json".to_owned(),
                    "3.1.1",
                    "fixture"
                )
                .is_err()
            );
        }
        let document =
            json!({"components":{"schemas":{"A":{"$id":"https://example.test/r","$ref":"#/%GG"}}}});
        assert!(
            SchemaResources::build(
                &document,
                "https://example.test/openapi.json".to_owned(),
                "3.1.1",
                "fixture"
            )
            .unwrap_err()
            .to_string()
            .contains("malformed percent")
        );
        let document =
            json!({"components":{"schemas":{"A":{"$ref":"https://example.test/missing#/Schema"}}}});
        assert!(
            SchemaResources::build(
                &document,
                "https://example.test/openapi.json".to_owned(),
                "3.1.1",
                "fixture"
            )
            .unwrap_err()
            .to_string()
            .contains("not registered")
        );
    }

    #[test]
    fn version_30_keeps_boolean_additional_properties_and_ignores_31_keywords() {
        let document = json!({"components":{"schemas":{"Thing":{"$id":"https://example.test/ignored","$dynamicRef":"#unknown","additionalProperties":false}}}});
        assert!(
            SchemaResources::build(
                &document,
                "https://example.test/openapi.json".to_owned(),
                "3.0.3",
                "fixture"
            )
            .is_ok()
        );
    }

    #[test]
    fn malformed_supported_schema_containers_fail_instead_of_being_skipped() {
        for schema in [
            json!({"properties": []}),
            json!({"allOf": {}}),
            json!({"items": "string"}),
            json!({"additionalProperties": []}),
            json!({"$defs": []}),
        ] {
            let document = json!({"components":{"schemas":{"Thing":schema}}});
            assert!(
                SchemaResources::build(
                    &document,
                    "https://example.test/openapi.json".to_owned(),
                    "3.1.1",
                    "fixture"
                )
                .is_err(),
                "accepted malformed schema: {schema}"
            );
        }
        for schema in [
            json!({"properties": []}),
            json!({"allOf": {}}),
            json!({"items": "string"}),
        ] {
            let document = json!({"components":{"schemas":{"Thing":schema}}});
            assert!(
                SchemaResources::build(
                    &document,
                    "https://example.test/openapi.json".to_owned(),
                    "3.0.3",
                    "fixture"
                )
                .is_err(),
                "accepted malformed 3.0 schema: {schema}"
            );
        }
    }

    #[test]
    fn document_uri_rejects_even_empty_fragment_while_id_normalizes_it() {
        assert!(
            document_uri(
                Path::new("unused.yaml"),
                Some("https://example.test/openapi#")
            )
            .is_err()
        );
        let document = json!({"components":{"schemas":{"Thing":{"$id":"https://example.test/schema#","type":"object"}}}});
        assert!(
            SchemaResources::build(
                &document,
                "https://example.test/openapi.json".to_owned(),
                "3.1.1",
                "fixture"
            )
            .is_ok()
        );
    }
}
