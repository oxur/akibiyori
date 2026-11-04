use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

use serde_json::Value;

use crate::config::{NavigationLink, validate_route};
use crate::openapi::{Contract, Operation};
use crate::{Diagnostic, Error};

pub(crate) struct RenderContext<'a> {
    pub base_path: &'a str,
}

impl RenderContext<'_> {
    pub fn href(&self, route: &str) -> String {
        format!("{}{}", self.base_path, route)
    }
}

pub(crate) fn render_contract(
    contract: &Contract,
    context: &RenderContext<'_>,
    output: &mut Vec<super::PendingFile>,
    diagnostics: &mut Vec<Diagnostic>,
) -> Result<(), Error> {
    for operation in &contract.operations {
        inspect_unrendered_operation(contract, operation, diagnostics);
        let route = format!("/reference/{}/", operation.id);
        output.push(page_file(
            &route,
            &format!("{} {}", operation.method, operation.path),
            &render_operation(contract, operation, context),
        )?);
    }
    for (name, schema) in &contract.schemas {
        let route = format!("/reference/models/{}/{name}/", contract.id);
        output.push(page_file(
            &route,
            name,
            &render_model(contract, name, schema, context, diagnostics),
        )?);
    }
    Ok(())
}

fn inspect_unrendered_operation(
    contract: &Contract,
    operation: &Operation,
    diagnostics: &mut Vec<Diagnostic>,
) {
    inspect_unrendered_object(
        contract,
        &operation.path_item,
        &format!(
            "#/paths/{}",
            operation.path.replace('~', "~0").replace('/', "~1")
        ),
        &[
            "$ref",
            "parameters",
            "get",
            "put",
            "post",
            "delete",
            "options",
            "head",
            "patch",
            "trace",
        ],
        "path-item feature",
        diagnostics,
    );
    let operation_pointer = format!(
        "#/paths/{}/{}",
        operation.path.replace('~', "~0").replace('/', "~1"),
        operation.method.to_ascii_lowercase()
    );
    inspect_unrendered_object(
        contract,
        &operation.value,
        &operation_pointer,
        &[
            "operationId",
            "summary",
            "description",
            "tags",
            "parameters",
            "requestBody",
            "responses",
            "security",
        ],
        "operation feature",
        diagnostics,
    );
    for parameter in operation
        .path_item
        .get("parameters")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .chain(
            operation
                .value
                .get("parameters")
                .and_then(Value::as_array)
                .into_iter()
                .flatten(),
        )
    {
        let parameter = resolve_component_ref(parameter, contract, "parameters");
        let name = parameter
            .get("name")
            .and_then(Value::as_str)
            .unwrap_or("parameter");
        inspect_unrendered_object(
            contract,
            &parameter,
            &format!(
                "{operation_pointer}/parameters/{}",
                json_pointer_escape(name)
            ),
            &["$ref", "name", "in", "description", "required", "schema"],
            "parameter feature",
            diagnostics,
        );
    }
    if let Some(request_body) = operation.value.get("requestBody") {
        let request_body = resolve_component_ref(request_body, contract, "requestBodies");
        inspect_unrendered_object(
            contract,
            &request_body,
            &format!("{operation_pointer}/requestBody"),
            &["$ref", "description", "required", "content"],
            "request-body feature",
            diagnostics,
        );
        inspect_media_features(
            contract,
            request_body.get("content"),
            &format!("{operation_pointer}/requestBody/content"),
            diagnostics,
        );
    }
    if let Some(responses) = operation.value.get("responses").and_then(Value::as_object) {
        for (status, response) in responses {
            let response = resolve_component_ref(response, contract, "responses");
            let response_pointer = format!("{operation_pointer}/responses/{status}");
            inspect_unrendered_object(
                contract,
                &response,
                &response_pointer,
                &["$ref", "description", "content"],
                "response feature",
                diagnostics,
            );
            inspect_media_features(
                contract,
                response.get("content"),
                &format!("{response_pointer}/content"),
                diagnostics,
            );
        }
    }
}

fn inspect_media_features(
    contract: &Contract,
    content: Option<&Value>,
    pointer: &str,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let Some(media) = content.and_then(Value::as_object) else {
        return;
    };
    for (media_type, value) in media {
        inspect_unrendered_object(
            contract,
            value,
            &format!(
                "{pointer}/{}",
                media_type.replace('~', "~0").replace('/', "~1")
            ),
            &["schema", "example", "examples"],
            "media-type feature",
            diagnostics,
        );
    }
}

fn inspect_unrendered_object(
    contract: &Contract,
    value: &Value,
    pointer: &str,
    rendered: &[&str],
    feature_kind: &str,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let Some(object) = value.as_object() else {
        return;
    };
    for key in object
        .keys()
        .filter(|key| !rendered.contains(&key.as_str()))
    {
        diagnostics.push(Diagnostic {
            contract: contract.id.clone(),
            pointer: format!("{pointer}/{}", json_pointer_escape(key)),
            message: format!("{feature_kind} {key:?} is retained in source JSON but not interpreted in the concise view"),
        });
    }
}

pub(crate) fn render_navigation(links: &[NavigationLink], base_path: &str) -> String {
    let mut html = String::from("<nav aria-label=\"Primary\"><ul>");
    for link in links {
        let href = if link.href.starts_with('/') {
            format!("{base_path}{}", link.href)
        } else {
            link.href.clone()
        };
        html.push_str(&format!(
            "<li><a href=\"{}\">{}</a></li>",
            escape_attr(&href),
            escape_text(&link.label)
        ));
    }
    html.push_str("</ul></nav>");
    html
}

pub(crate) fn render_page(title: &str, route: &str, body: &str) -> String {
    format!(
        "---\ntitle: {}\npermalink: {}\nlayout: default.liquid\ntemplated: false\n---\n{}",
        super::yaml_quote(title),
        super::yaml_quote(route),
        body
    )
}

fn page_file(route: &str, title: &str, body: &str) -> Result<super::PendingFile, Error> {
    validate_route(route)?;
    Ok(super::PendingFile {
        path: page_route_file(route),
        contents: render_page(title, route, body).into_bytes(),
    })
}

fn render_model(
    contract: &Contract,
    name: &str,
    schema: &Value,
    context: &RenderContext<'_>,
    diagnostics: &mut Vec<Diagnostic>,
) -> String {
    let mut html = format!(
        "<p class=\"eyebrow\">{} contract · model</p><h1>{}</h1>",
        escape_text(&contract.label),
        escape_text(name)
    );
    html.push_str(&format!(
        "<p>Source: <code>{}</code></p>",
        escape_text(&format!("{}#/components/schemas/{name}", contract.source))
    ));
    html.push_str(&format!(
        "<section aria-labelledby=\"shape\"><h2 id=\"shape\">Schema shape</h2>{}</section>",
        schema_summary(schema)
    ));
    let mut seen = BTreeSet::new();
    render_schema_body(
        contract,
        schema,
        "",
        context,
        &mut html,
        &mut seen,
        diagnostics,
        &format!("#/components/schemas/{name}"),
    );
    html.push_str("<section><h2>Original schema JSON</h2><pre><code>");
    html.push_str(&escape_text(&pretty_json(schema)));
    html.push_str("</code></pre></section>");
    html
}

fn schema_summary(schema: &Value) -> String {
    let mut parts = Vec::new();
    for keyword in ["type", "format", "title", "description"] {
        if let Some(value) = schema.get(keyword) {
            let value = value
                .as_str()
                .map(str::to_owned)
                .unwrap_or_else(|| value.to_string());
            parts.push(format!(
                "<p><strong>{}</strong>: {}</p>",
                escape_text(keyword),
                escape_text(&value)
            ));
        }
    }
    if parts.is_empty() {
        "<p>No summary fields are supplied in this schema.</p>".to_owned()
    } else {
        parts.join("")
    }
}

#[allow(clippy::too_many_arguments)]
fn render_schema_body(
    contract: &Contract,
    schema: &Value,
    prefix: &str,
    context: &RenderContext<'_>,
    html: &mut String,
    seen: &mut BTreeSet<String>,
    diagnostics: &mut Vec<Diagnostic>,
    pointer: &str,
) {
    if let Some(reference) = schema.get("$ref").and_then(Value::as_str) {
        render_ref_link(contract, reference, context, html, pointer);
    }
    if !seen.insert(pointer.to_owned()) {
        return;
    }
    inspect_unknown_schema_keys(contract, schema, pointer, diagnostics);

    if let Some(properties) = schema.get("properties").and_then(Value::as_object) {
        let required: BTreeSet<&str> = schema
            .get("required")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
            .collect();
        html.push_str("<section class=\"fields\"><h2>Fields</h2><dl>");
        for (field, property) in properties {
            let full_path = if prefix.is_empty() {
                field.clone()
            } else {
                format!("{prefix}.{field}")
            };
            let field_pointer = format!("{pointer}/properties/{}", json_pointer_escape(field));
            let anchor = stable_anchor(&field_pointer);
            let label = format!(
                "{}{}",
                escape_text(&full_path),
                if required.contains(field.as_str()) {
                    " <span class=\"required\">required</span>"
                } else {
                    ""
                }
            );
            html.push_str(&format!(
                "<dt id=\"{anchor}\">{label}</dt><dd>{}",
                property_summary(property)
            ));
            if let Some(description) = property.get("description").and_then(Value::as_str) {
                html.push_str(&format!("<p>{}</p>", escape_text(description)));
            }
            render_constraints(property, html);
            if let Some(reference) = property.get("$ref").and_then(Value::as_str) {
                render_ref_link(contract, reference, context, html, &field_pointer);
            }
            if property.get("properties").is_some()
                || property.get("allOf").is_some()
                || property.get("oneOf").is_some()
                || property.get("anyOf").is_some()
            {
                render_schema_body(
                    contract,
                    property,
                    &full_path,
                    context,
                    html,
                    seen,
                    diagnostics,
                    &field_pointer,
                );
            }
            if let Some(items) = property.get("items") {
                html.push_str("<p>Array items:</p>");
                render_schema_body(
                    contract,
                    items,
                    &full_path,
                    context,
                    html,
                    seen,
                    diagnostics,
                    &format!("{field_pointer}/items"),
                );
            }
            if let Some(additional) = property
                .get("additionalProperties")
                .filter(|value| value.is_object())
            {
                html.push_str("<p>Additional properties:</p>");
                render_schema_body(
                    contract,
                    additional,
                    &full_path,
                    context,
                    html,
                    seen,
                    diagnostics,
                    &format!("{field_pointer}/additionalProperties"),
                );
            }
            html.push_str("</dd>");
        }
        html.push_str("</dl></section>");
    }
    if let Some(items) = schema.get("items") {
        html.push_str("<section><h2>Array items</h2>");
        render_schema_body(
            contract,
            items,
            prefix,
            context,
            html,
            seen,
            diagnostics,
            &format!("{pointer}/items"),
        );
        html.push_str("</section>");
    }
    if let Some(additional) = schema
        .get("additionalProperties")
        .filter(|value| value.is_object())
    {
        html.push_str("<section><h2>Additional properties</h2>");
        render_schema_body(
            contract,
            additional,
            prefix,
            context,
            html,
            seen,
            diagnostics,
            &format!("{pointer}/additionalProperties"),
        );
        html.push_str("</section>");
    }
    for (keyword, heading, explanation) in [
        ("allOf", "allOf", "All schemas must apply"),
        ("oneOf", "oneOf", "Exactly one schema must apply"),
        ("anyOf", "anyOf", "One or more schemas must apply"),
    ] {
        if let Some(alternatives) = schema.get(keyword).and_then(Value::as_array) {
            html.push_str(&format!("<section><h2>{heading}: {explanation}</h2><ul>"));
            for (index, alternative) in alternatives.iter().enumerate() {
                html.push_str(&format!("<li><strong>Alternative {}</strong>", index + 1));
                let alt_pointer = format!("{pointer}/{keyword}/{index}");
                if let Some(reference) = alternative.get("$ref").and_then(Value::as_str) {
                    render_ref_link(contract, reference, context, html, &alt_pointer);
                }
                if alternative.get("properties").is_some()
                    || alternative.get("allOf").is_some()
                    || alternative.get("oneOf").is_some()
                    || alternative.get("anyOf").is_some()
                {
                    render_schema_body(
                        contract,
                        alternative,
                        prefix,
                        context,
                        html,
                        seen,
                        diagnostics,
                        &alt_pointer,
                    );
                }
                html.push_str("</li>");
            }
            html.push_str("</ul></section>");
        }
    }
}

fn inspect_unknown_schema_keys(
    contract: &Contract,
    schema: &Value,
    pointer: &str,
    diagnostics: &mut Vec<Diagnostic>,
) {
    const KNOWN: &[&str] = &[
        "$ref",
        "$schema",
        "$id",
        "type",
        "format",
        "title",
        "description",
        "properties",
        "required",
        "items",
        "additionalProperties",
        "allOf",
        "oneOf",
        "anyOf",
        "not",
        "enum",
        "const",
        "nullable",
        "default",
        "example",
        "examples",
        "readOnly",
        "writeOnly",
        "deprecated",
        "xml",
        "externalDocs",
        "discriminator",
        "minLength",
        "maxLength",
        "pattern",
        "minimum",
        "maximum",
        "exclusiveMinimum",
        "exclusiveMaximum",
        "multipleOf",
        "minItems",
        "maxItems",
        "uniqueItems",
        "minProperties",
        "maxProperties",
        "contentEncoding",
        "contentMediaType",
    ];
    let Some(object) = schema.as_object() else {
        return;
    };
    for key in object.keys().filter(|key| !KNOWN.contains(&key.as_str())) {
        diagnostics.push(Diagnostic {
            contract: contract.id.clone(),
            pointer: format!("{pointer}/{}", json_pointer_escape(key)),
            message: format!("schema keyword {key:?} is retained in the original JSON but not interpreted in the field view"),
        });
    }
}

fn render_constraints(schema: &Value, html: &mut String) {
    const CONSTRAINTS: &[&str] = &[
        "type",
        "format",
        "nullable",
        "enum",
        "minimum",
        "maximum",
        "exclusiveMinimum",
        "exclusiveMaximum",
        "multipleOf",
        "minLength",
        "maxLength",
        "minItems",
        "maxItems",
        "pattern",
        "readOnly",
        "writeOnly",
        "default",
        "example",
    ];
    let rows: Vec<String> = CONSTRAINTS
        .iter()
        .filter_map(|key| {
            schema.get(*key).map(|value| {
                format!(
                    "<li><code>{}</code>: <code>{}</code></li>",
                    escape_text(key),
                    escape_text(&value.to_string())
                )
            })
        })
        .collect();
    if !rows.is_empty() {
        html.push_str(&format!("<ul class=\"constraints\">{}</ul>", rows.join("")));
    }
}

fn property_summary(schema: &Value) -> String {
    let mut values = Vec::new();
    if let Some(reference) = schema.get("$ref").and_then(Value::as_str) {
        values.push(format!("reference <code>{}</code>", escape_text(reference)));
    }
    if let Some(type_) = schema.get("type").and_then(Value::as_str) {
        values.push(format!("type <code>{}</code>", escape_text(type_)));
    }
    if let Some(format) = schema.get("format").and_then(Value::as_str) {
        values.push(format!("format <code>{}</code>", escape_text(format)));
    }
    if let Some(title) = schema.get("title").and_then(Value::as_str) {
        values.push(escape_text(title));
    }
    if values.is_empty() {
        "<p>See the original schema JSON for the complete property definition.</p>".to_owned()
    } else {
        format!("<p>{}</p>", values.join(" · "))
    }
}

fn render_ref_link(
    contract: &Contract,
    reference: &str,
    context: &RenderContext<'_>,
    html: &mut String,
    pointer: &str,
) {
    let model = reference
        .strip_prefix("#/components/schemas/")
        .unwrap_or_default();
    if contract.schemas.contains_key(model) {
        let route = context.href(&format!("/reference/models/{}/{model}/", contract.id));
        html.push_str(&format!(
            "<p>Schema reference: <a href=\"{}\">{}</a> <code>{}</code></p>",
            escape_attr(&route),
            escape_text(model),
            escape_text(pointer)
        ));
    } else {
        html.push_str(&format!(
            "<p>Local reference: <code>{}</code></p>",
            escape_text(reference)
        ));
    }
}

fn render_operation(
    contract: &Contract,
    operation: &Operation,
    context: &RenderContext<'_>,
) -> String {
    let mut html = format!(
        "<p class=\"eyebrow\">{} contract · operation</p><h1>{}</h1>",
        escape_text(&contract.label),
        escape_text(operation.id.as_str())
    );
    html.push_str(&format!(
        "<p><code>{} {}</code></p>",
        escape_text(&operation.method),
        escape_text(&operation.path)
    ));
    if let Some(summary) = operation.value.get("summary").and_then(Value::as_str) {
        html.push_str(&format!("<p>{}</p>", escape_text(summary)));
    }
    if let Some(description) = operation.value.get("description").and_then(Value::as_str) {
        html.push_str(&format!("<p>{}</p>", escape_text(description)));
    }
    if let Some(tags) = operation.value.get("tags").and_then(Value::as_array) {
        html.push_str("<p>Tags: ");
        html.push_str(
            &tags
                .iter()
                .filter_map(Value::as_str)
                .map(escape_text)
                .collect::<Vec<_>>()
                .join(", "),
        );
        html.push_str("</p>");
    }
    let parameters = effective_parameters(&operation.path_item, &operation.value, contract);
    if !parameters.is_empty() {
        html.push_str("<section><h2>Parameters</h2><dl>");
        for parameter in parameters {
            let name = parameter
                .get("name")
                .and_then(Value::as_str)
                .unwrap_or("(unnamed)");
            let location = parameter
                .get("in")
                .and_then(Value::as_str)
                .unwrap_or("unknown");
            html.push_str(&format!(
                "<dt>{} <span class=\"muted\">in {}</span>{}</dt><dd>",
                escape_text(name),
                escape_text(location),
                if parameter
                    .get("required")
                    .and_then(Value::as_bool)
                    .unwrap_or(false)
                {
                    " <span class=\"required\">required</span>"
                } else {
                    ""
                }
            ));
            if let Some(description) = parameter.get("description").and_then(Value::as_str) {
                html.push_str(&format!("<p>{}</p>", escape_text(description)));
            }
            if let Some(schema) = parameter.get("schema") {
                html.push_str(&format!(
                    "<p>Schema: {}</p>",
                    summarize_refs(schema, contract, context)
                ));
                html.push_str(&format!(
                    "<pre><code>{}</code></pre>",
                    escape_text(&pretty_json(schema))
                ));
            }
            html.push_str("</dd>");
        }
        html.push_str("</dl></section>");
    }
    if let Some(request_body) = operation.value.get("requestBody") {
        let request_body = resolve_component_ref(request_body, contract, "requestBodies");
        html.push_str("<section><h2>Request body</h2>");
        if let Some(description) = request_body.get("description").and_then(Value::as_str) {
            html.push_str(&format!("<p>{}</p>", escape_text(description)));
        }
        render_content(request_body.get("content"), contract, context, &mut html);
        html.push_str("</section>");
    }
    if let Some(responses) = operation.value.get("responses").and_then(Value::as_object) {
        html.push_str("<section><h2>Responses</h2>");
        for (status, response) in responses {
            let response = resolve_component_ref(response, contract, "responses");
            html.push_str(&format!("<h3>Status {}</h3>", escape_text(status)));
            if let Some(description) = response.get("description").and_then(Value::as_str) {
                html.push_str(&format!("<p>{}</p>", escape_text(description)));
            }
            render_content(response.get("content"), contract, context, &mut html);
        }
        html.push_str("</section>");
    }
    let document = contract.document.as_object();
    let operation_security = operation.value.get("security");
    let effective_security =
        operation_security.or_else(|| document.and_then(|root| root.get("security")));
    if let Some(security) = effective_security {
        html.push_str("<section><h2>Authentication and security</h2><p>Security alternatives are ORed; multiple schemes in one requirement are ANDed. An explicit empty array overrides inherited requirements.</p>");
        html.push_str(&format!(
            "<pre><code>{}</code></pre>",
            escape_text(&pretty_json(security))
        ));
        if let Some(schemes) = document
            .and_then(|root| root.get("components"))
            .and_then(|value| value.get("securitySchemes"))
            .and_then(Value::as_object)
        {
            for (name, scheme) in schemes {
                html.push_str(&format!(
                    "<h3>{}</h3><pre><code>{}</code></pre>",
                    escape_text(name),
                    escape_text(&pretty_json(scheme))
                ));
            }
        }
        html.push_str("</section>");
    }
    html.push_str("<section><h2>Source details</h2><p>Contract source and effective path metadata:</p><pre><code>");
    html.push_str(&escape_text(&pretty_json(&serde_json::json!({"contract":contract.id,"source":contract.source,"pathItem":operation.path_item,"operation":operation.value}))));
    html.push_str("</code></pre></section>");
    html
}

fn effective_parameters(path_item: &Value, operation: &Value, contract: &Contract) -> Vec<Value> {
    let mut merged: BTreeMap<(String, String), Value> = BTreeMap::new();
    for parameter in path_item
        .get("parameters")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .chain(
            operation
                .get("parameters")
                .and_then(Value::as_array)
                .into_iter()
                .flatten(),
        )
    {
        let resolved = resolve_component_ref(parameter, contract, "parameters");
        let name = resolved
            .get("name")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_owned();
        let location = resolved
            .get("in")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_owned();
        merged.insert((name, location), resolved);
    }
    merged.into_values().collect()
}

fn resolve_component_ref(value: &Value, contract: &Contract, component: &str) -> Value {
    let Some(reference) = value.get("$ref").and_then(Value::as_str) else {
        return value.clone();
    };
    let prefix = format!("#/components/{component}/");
    let Some(name) = reference.strip_prefix(&prefix) else {
        return value.clone();
    };
    contract.resolve_ref(reference).cloned().unwrap_or_else(|| {
        let pointer = format!("/components/{component}/{}", json_pointer_escape(name));
        contract
            .document
            .pointer(&pointer)
            .cloned()
            .unwrap_or_else(|| value.clone())
    })
}

fn render_content(
    content: Option<&Value>,
    contract: &Contract,
    context: &RenderContext<'_>,
    html: &mut String,
) {
    let Some(media) = content.and_then(Value::as_object) else {
        return;
    };
    for (media_type, media_object) in media {
        html.push_str(&format!(
            "<h4>Media type <code>{}</code></h4>",
            escape_text(media_type)
        ));
        if let Some(schema) = media_object.get("schema") {
            html.push_str(&format!(
                "<p>Schema: {}</p>",
                summarize_refs(schema, contract, context)
            ));
            html.push_str(&format!(
                "<pre><code>{}</code></pre>",
                escape_text(&pretty_json(schema))
            ));
        }
        for example_key in ["example", "examples"] {
            if let Some(examples) = media_object.get(example_key) {
                html.push_str(&format!("<p>Contract-provided {example_key}; validity is not independently asserted:</p><pre><code>{}</code></pre>", escape_text(&pretty_json(examples))));
            }
        }
    }
}

fn summarize_refs(schema: &Value, contract: &Contract, context: &RenderContext<'_>) -> String {
    let mut refs = Vec::new();
    collect_schema_refs(schema, &mut refs);
    if refs.is_empty() {
        return "inline schema (full definition follows)".to_owned();
    }
    refs.into_iter()
        .map(|reference| {
            let name = reference
                .strip_prefix("#/components/schemas/")
                .unwrap_or(&reference);
            if contract.schemas.contains_key(name) {
                let href = context.href(&format!("/reference/models/{}/{name}/", contract.id));
                format!(
                    "<a href=\"{}\">{}</a>",
                    escape_attr(&href),
                    escape_text(name)
                )
            } else {
                escape_text(name)
            }
        })
        .collect::<Vec<_>>()
        .join(", ")
}

fn collect_schema_refs(value: &Value, refs: &mut Vec<String>) {
    match value {
        Value::Object(object) => {
            if let Some(reference) = object.get("$ref").and_then(Value::as_str) {
                refs.push(reference.to_owned());
            }
            for child in object.values() {
                collect_schema_refs(child, refs);
            }
        }
        Value::Array(items) => {
            for child in items {
                collect_schema_refs(child, refs);
            }
        }
        _ => {}
    }
}

fn stable_anchor(pointer: &str) -> String {
    let mut anchor = String::from("field-");
    for byte in pointer.bytes() {
        anchor.push_str(&format!("{byte:02x}"));
    }
    anchor
}

fn json_pointer_escape(value: &str) -> String {
    value.replace('~', "~0").replace('/', "~1")
}

fn pretty_json(value: &Value) -> String {
    serde_json::to_string_pretty(value).unwrap_or_else(|_| value.to_string())
}

fn escape_text(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

fn escape_attr(value: &str) -> String {
    escape_text(value)
}

pub(crate) fn page_route_file(route: &str) -> PathBuf {
    let relative = route.trim_start_matches('/');
    if route == "/" {
        PathBuf::from("index.md")
    } else {
        PathBuf::from(relative).join("index.md")
    }
}

pub(crate) fn render_indexes(
    contracts: &[Contract],
    context: &RenderContext<'_>,
) -> Result<Vec<super::PendingFile>, Error> {
    let mut operations = String::from("<p>Operations in the selected contracts.</p><ul>");
    for contract in contracts {
        for operation in &contract.operations {
            let route = context.href(&format!("/reference/{}/", operation.id));
            operations.push_str(&format!(
                "<li><a href=\"{}\"><code>{} {}</code> · {}</a></li>",
                escape_attr(&route),
                escape_text(&operation.method),
                escape_text(&operation.path),
                escape_text(&operation.id)
            ));
        }
    }
    operations.push_str("</ul>");
    let mut models = String::from("<p>Named models are grouped by contract identifier.</p>");
    for contract in contracts {
        models.push_str(&format!("<h2>{}</h2><ul>", escape_text(&contract.label)));
        for name in contract.schemas.keys() {
            let route = context.href(&format!("/reference/models/{}/{name}/", contract.id));
            models.push_str(&format!(
                "<li><a href=\"{}\">{}</a></li>",
                escape_attr(&route),
                escape_text(name)
            ));
        }
        models.push_str("</ul>");
    }
    Ok(vec![
        page_file("/reference/operations/", "Operations", &operations)?,
        page_file("/reference/models/", "Models", &models)?,
    ])
}
