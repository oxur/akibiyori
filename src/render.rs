use std::collections::{BTreeSet, HashMap};
use std::path::PathBuf;

use serde::Serialize;
use serde_json::Value;

use crate::config::{
    DescriptionMode, DocumentHeader, NavigationIcon, NavigationLink, ReferenceLayout,
    ReferenceOptions, SidebarGroup, validate_route,
};
use crate::markdown::DescriptionRenderer;
use crate::openapi::{Contract, Operation, OperationKind, ResolvedObject};
use crate::{Diagnostic, Error};

pub(crate) struct RenderContext<'a> {
    pub base_path: &'a str,
    pub has_home: bool,
    pub reference: ReferenceOptions,
    pub markdown: Option<&'a DescriptionRenderer>,
}

impl RenderContext<'_> {
    pub fn href(&self, route: &str) -> String {
        format!("{}{}", self.base_path, route)
    }

    fn description(&self, value: &str) -> Result<String, Error> {
        match (self.reference.descriptions, self.markdown) {
            (DescriptionMode::Markdown, Some(renderer)) => {
                renderer.render(value, self.base_path, self.reference.images)
            }
            _ => Ok(format!("<p>{}</p>", escape_text(value))),
        }
    }
}

pub(crate) fn render_contract(
    contract: &Contract,
    context: &RenderContext<'_>,
    output: &mut Vec<super::PendingFile>,
    diagnostics: &mut Vec<Diagnostic>,
) -> Result<(), Error> {
    for operation in &contract.operations {
        inspect_unrendered_operation(contract, operation, diagnostics)?;
        let route = format!("/reference/{}/", operation.id);
        if context.reference.layout == ReferenceLayout::Columns {
            let (body, title) =
                render_operation_columns(contract, operation, context, diagnostics)?;
            output.push(operation_columns_page_file(&route, &title, &body)?);
        } else {
            output.push(page_file(
                &route,
                &operation_title(operation),
                &render_operation(contract, operation, context)?,
                context,
            )?);
        }
    }
    for (name, schema) in &contract.schemas {
        let route = format!("/reference/models/{}/{name}/", contract.id);
        output.push(page_file(
            &route,
            name,
            &render_model(contract, name, schema, context, diagnostics)?,
            context,
        )?);
    }
    Ok(())
}

fn operation_columns_page_file(
    route: &str,
    title: &str,
    body: &str,
) -> Result<super::PendingFile, Error> {
    validate_route(route)?;
    Ok(super::PendingFile {
        path: generated_source_file(route),
        contents: format!(
            "---\ntitle: {}\npermalink: {}\nlayout: default.liquid\ntemplated: false\ndata:\n  reference_layout: columns\n---\n<div class=\"operation-document\"><h1 class=\"operation-title\">{}</h1><div class=\"operation-body prose flex-1 text-fd-foreground/90\">{}</div></div>",
            super::yaml_quote(title),
            super::yaml_quote(&format!("/{}", route_target(route))),
            escape_text(title),
            body,
        )
        .into_bytes(),
    })
}

fn operation_title(operation: &Operation) -> String {
    match &operation.kind {
        OperationKind::Path => format!("{} {}", operation.method, operation.path),
        OperationKind::Webhook { name } => format!(
            "Webhook {name} · {} · caller-defined destination",
            operation.method
        ),
    }
}

fn render_operation_columns(
    contract: &Contract,
    operation: &Operation,
    context: &RenderContext<'_>,
    diagnostics: &mut Vec<Diagnostic>,
) -> Result<(String, String), Error> {
    let title = operation
        .value
        .get("summary")
        .and_then(Value::as_str)
        .filter(|summary| !summary.trim().is_empty())
        .unwrap_or(&operation.id)
        .to_owned();
    let mut main = String::from("<div class=\"operation-main\"><div class=\"operation-flow\">");
    main.push_str(&format!(
        "<p class=\"operation-endpoint\"><span class=\"method {}\">{}</span><code>{}</code></p>",
        method_color_class(&operation.method),
        escape_text(&operation.method),
        escape_text(&operation.path)
    ));
    if let Some(description) = operation.value.get("description").and_then(Value::as_str) {
        main.push_str(&context.description(description)?);
    }

    render_security_summary(contract, operation, &mut main);
    let mut request_examples = Vec::new();
    if let Some(request_body) = operation.value.get("requestBody") {
        let resolved = contract.resolve_component_object(
            request_body,
            &format!("{}/requestBody", operation.pointer),
            "requestBodies",
        )?;
        main.push_str("<section class=\"operation-section\"><h2>Request body</h2>");
        if let Some(description) = resolved.value.get("description").and_then(Value::as_str) {
            main.push_str(&context.description(description)?);
        }
        if resolved.value.get("required").and_then(Value::as_bool) == Some(true) {
            main.push_str("<p class=\"required\">Required</p>");
        }
        if let Some(content) = resolved.value.get("content").and_then(Value::as_object) {
            main.push_str("<div class=\"operation-media-group\" data-media-group><div class=\"operation-media-controls\"><label><span class=\"visually-hidden\">Request media type</span><select data-media-select disabled aria-label=\"Request media type\"></select></label></div>");
            for (media_type, media) in content {
                let pointer = format!(
                    "{}/content/{}",
                    resolved.pointer,
                    json_pointer_escape(media_type)
                );
                main.push_str(&format!(
                    "<section class=\"operation-media-panel\" data-media-panel data-media-label=\"{}\"><h3>{}</h3>",
                    escape_attr(media_type), escape_text(media_type)
                ));
                if let Some(schema) = media.get("schema") {
                    main.push_str(&format!(
                        "<p class=\"schema-reference\">{}</p>",
                        summarize_refs(schema, contract, context)
                    ));
                    let schema_pointer = format!("{pointer}/schema");
                    render_operation_schema(
                        contract,
                        context,
                        schema,
                        &schema_pointer,
                        diagnostics,
                        &mut main,
                    )?;
                    if contract.is_openapi_31 {
                        main.push_str("<details class=\"resolved-schema\"><summary>Resolved schema reading</summary>");
                        let mut remaining = 128;
                        render_schema_resource_details(
                            contract,
                            context,
                            schema,
                            &schema_pointer,
                            &[],
                            &mut BTreeSet::new(),
                            0,
                            &mut remaining,
                            &mut main,
                        )?;
                        main.push_str("</details>");
                    }
                }
                let variants = crate::examples::variants_for_media(
                    contract, media_type, media, &pointer, true,
                )?;
                request_examples.push((media_type.clone(), variants));
                main.push_str("</section>");
            }
            main.push_str("</div>");
        }
        main.push_str("</section>");
    }

    if let Some(responses) = operation.value.get("responses").and_then(Value::as_object) {
        main.push_str("<section class=\"operation-section\"><h2>Responses</h2>");
        for (status, response) in responses {
            let resolved = contract.resolve_component_object(
                response,
                &format!(
                    "{}/responses/{}",
                    operation.pointer,
                    json_pointer_escape(status)
                ),
                "responses",
            )?;
            main.push_str(&format!("<section class=\"operation-response\"><h3><span class=\"status-code\">{}</span></h3>", escape_text(status)));
            if let Some(description) = resolved.value.get("description").and_then(Value::as_str) {
                main.push_str(&context.description(description)?);
            }
            if let Some(content) = resolved.value.get("content").and_then(Value::as_object) {
                main.push_str("<div class=\"operation-media-group\" data-media-group><div class=\"operation-media-controls\"><label><span class=\"visually-hidden\">Response media type</span><select data-media-select disabled aria-label=\"Response media type\"></select></label></div>");
                for (media_type, media) in content {
                    let pointer = format!(
                        "{}/content/{}",
                        resolved.pointer,
                        json_pointer_escape(media_type)
                    );
                    main.push_str(&format!("<section class=\"operation-media-panel\" data-media-panel data-media-label=\"{}\"><h4>{}</h4>", escape_attr(media_type), escape_text(media_type)));
                    if let Some(schema) = media.get("schema") {
                        main.push_str(&format!(
                            "<p class=\"schema-reference\">{}</p>",
                            summarize_refs(schema, contract, context)
                        ));
                        let schema_pointer = format!("{pointer}/schema");
                        render_operation_schema(
                            contract,
                            context,
                            schema,
                            &schema_pointer,
                            diagnostics,
                            &mut main,
                        )?;
                        if contract.is_openapi_31 {
                            main.push_str("<details class=\"resolved-schema\"><summary>Resolved schema reading</summary>");
                            let mut remaining = 128;
                            render_schema_resource_details(
                                contract,
                                context,
                                schema,
                                &schema_pointer,
                                &[],
                                &mut BTreeSet::new(),
                                0,
                                &mut remaining,
                                &mut main,
                            )?;
                            main.push_str("</details>");
                        }
                    }
                    let variants = crate::examples::variants_for_media(
                        contract, media_type, media, &pointer, false,
                    )?;
                    request_examples.push((format!("response:{status}:{media_type}"), variants));
                    main.push_str("</section>");
                }
                main.push_str("</div>");
            }
            main.push_str("</section>");
        }
        main.push_str("</section>");
    }
    main.push_str(&format!("<details class=\"operation-source\"><summary>Contract source</summary><p>{} ({}) · operation <code>{}</code></p><pre><code>", escape_text(&contract.label), escape_text(&contract.id), escape_text(&operation.id)));
    main.push_str(&escape_text(&format!(
        "{}{}",
        contract.source, operation.pointer
    )));
    main.push_str("</code></pre><p>Effective path item and operation source:</p><pre><code>");
    main.push_str(&escape_text(&pretty_json(&serde_json::json!({
        "contract": contract.id,
        "source": contract.source,
        "pathItem": operation.path_item,
        "operation": operation.value,
    }))));
    main.push_str("</code></pre></details></div></div>");

    let mut rail = String::from(
        "<aside class=\"operation-example-rail\" aria-label=\"Examples\"><section class=\"example-card example-group\"><header class=\"example-card-header\"><span>Example Request</span><label><span class=\"visually-hidden\">Request example variant</span><select data-example-select disabled aria-label=\"Request example variant\"></select></label></header>",
    );
    if request_examples.is_empty() {
        rail.push_str("<p>No request body is declared for this operation.</p>");
    }
    for (media_type, variants) in &request_examples {
        if media_type.starts_with("response:") {
            continue;
        }
        render_example_variants(contract, operation, media_type, variants, true, &mut rail);
    }
    rail.push_str("</section><section class=\"example-card example-group response-examples\"><header class=\"example-card-header\"><span>Example Response</span><label><span class=\"visually-hidden\">Response example variant</span><select data-example-select disabled aria-label=\"Response example variant\"></select></label></header>");
    for (media_type, variants) in &request_examples {
        if let Some(rest) = media_type.strip_prefix("response:") {
            render_example_variants(contract, operation, rest, variants, false, &mut rail);
        }
    }
    rail.push_str("</section></aside>");
    Ok((
        format!("<div class=\"operation-columns text-sm @container\">{main}{rail}</div>"),
        title,
    ))
}

fn method_color_class(method: &str) -> &'static str {
    match method.to_ascii_lowercase().as_str() {
        "post" => "text-blue-400",
        "put" => "text-yellow-400",
        "patch" => "text-orange-400",
        "delete" => "text-red-400",
        _ => "text-green-400",
    }
}

fn render_operation_schema(
    contract: &Contract,
    context: &RenderContext<'_>,
    schema: &Value,
    source_pointer: &str,
    diagnostics: &mut Vec<Diagnostic>,
    html: &mut String,
) -> Result<(), Error> {
    let mut current = schema;
    let mut pointer = source_pointer.to_owned();
    let mut scope = contract.schema_resources.lexical_scope(source_pointer);
    let mut visited = BTreeSet::new();
    for _ in 0..32 {
        let Some(object) = current.as_object() else {
            break;
        };
        let target = object
            .get("$dynamicRef")
            .and_then(Value::as_str)
            .map(|reference| (reference, true))
            .or_else(|| {
                object
                    .get("$ref")
                    .and_then(Value::as_str)
                    .map(|reference| (reference, false))
            });
        let Some((reference, dynamic)) = target else {
            break;
        };
        if !visited.insert(pointer.clone()) {
            html.push_str(
                "<p>Schema reference expansion stopped at a recursive source location.</p>",
            );
            return Ok(());
        }
        let target = contract.resolve_schema_reference(&pointer, reference, dynamic, &scope)?;
        current = contract
            .schema_resources
            .schema_at(&contract.document, &target.pointer)
            .ok_or_else(|| {
                Error::Invalid(format!(
                    "resolved operation schema {} is missing",
                    target.pointer
                ))
            })?;
        pointer = target.pointer;
        if !scope.contains(&target.uri) {
            scope.push(target.uri);
        }
    }
    let resource_uri = contract.schema_resources.resource_for_pointer(&pointer);
    if resource_uri.starts_with("http://") || resource_uri.starts_with("https://") {
        html.push_str(&format!(
            "<p class=\"schema-resource-source\">Schema resource: <code>{}{}</code></p>",
            escape_text(&resource_uri),
            escape_text(&pointer)
        ));
    }
    let mut active = BTreeSet::new();
    render_compact_schema_card(
        contract,
        context,
        current,
        &pointer,
        diagnostics,
        &mut active,
        0,
        html,
    )
}

// These parameters are the bounded recursive renderer's explicit traversal state.
#[allow(clippy::too_many_arguments)]
fn render_compact_schema_card(
    contract: &Contract,
    context: &RenderContext<'_>,
    schema: &Value,
    pointer: &str,
    diagnostics: &mut Vec<Diagnostic>,
    active: &mut BTreeSet<String>,
    depth: usize,
    html: &mut String,
) -> Result<(), Error> {
    if depth >= 8 || !active.insert(pointer.to_owned()) {
        html.push_str("<p class=\"schema-expansion-note\">Further nested fields are available from the linked schema.</p>");
        return Ok(());
    }
    let result = (|| {
        inspect_unknown_schema_keys(contract, schema, pointer, diagnostics);
        let Some(object) = schema.as_object() else {
            html.push_str(&format!("<div class=\"schema-card\"><div class=\"schema-card-header\">Boolean schema: {}</div></div>", if schema.as_bool() == Some(true) { "accepts all instances" } else { "accepts no instances" }));
            return Ok(());
        };
        let title = object
            .get("title")
            .and_then(Value::as_str)
            .unwrap_or_else(|| pointer.rsplit('/').next().unwrap_or("Schema"));
        let type_text = schema_type_label(schema);
        html.push_str(&format!("<section class=\"schema-card\" data-schema-card><header class=\"schema-card-header\"><span>{}</span><code>{}</code></header>", escape_text(title), escape_text(&type_text)));
        if let Some(description) = object.get("description").and_then(Value::as_str) {
            html.push_str(&format!(
                "<div class=\"schema-card-description\">{}</div>",
                context.description(description)?
            ));
        }
        if let Some(properties) = object.get("properties").and_then(Value::as_object) {
            let required: BTreeSet<&str> = object
                .get("required")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
                .filter_map(Value::as_str)
                .collect();
            html.push_str("<div class=\"schema-fields-header\"><h3>Properties</h3><label><span class=\"visually-hidden\">Filter Properties</span><input type=\"search\" class=\"property-filter\" data-property-filter disabled autocomplete=\"off\" aria-label=\"Filter Properties\" placeholder=\"Filter Properties\"></label></div><div class=\"fields\"><p class=\"property-filter-empty\" data-property-filter-empty hidden>No properties match this filter.</p>");
            for (name, property) in properties {
                let property_pointer =
                    format!("{pointer}/properties/{}", json_pointer_escape(name));
                render_compact_property(
                    contract,
                    context,
                    name,
                    property,
                    &property_pointer,
                    required.contains(name.as_str()),
                    diagnostics,
                    active,
                    depth + 1,
                    html,
                )?;
            }
            html.push_str("</div>");
        }
        if object.get("items").is_some_and(Value::is_object) {
            html.push_str("<details class=\"schema-nested\"><summary>Array item schema</summary>");
            render_compact_schema_card(
                contract,
                context,
                &object["items"],
                &format!("{pointer}/items"),
                diagnostics,
                active,
                depth + 1,
                html,
            )?;
            html.push_str("</details>");
        }
        if let Some(reference) = object
            .get("$ref")
            .or_else(|| object.get("$dynamicRef"))
            .and_then(Value::as_str)
        {
            html.push_str("<p class=\"schema-reference\">Schema reference: ");
            render_ref_link(contract, reference, context, html, pointer);
            html.push_str("</p>");
        }
        html.push_str("<details class=\"schema-original\"><summary>Source schema values</summary><p>JSON pointer: <code>");
        html.push_str(&escape_text(pointer));
        html.push_str("</code></p><pre><code>");
        html.push_str(&escape_text(&pretty_json(schema)));
        html.push_str("</code></pre></details></section>");
        Ok(())
    })();
    active.remove(pointer);
    result
}

// Property rendering shares the same explicit traversal state as nested schemas.
#[allow(clippy::too_many_arguments)]
fn render_compact_property(
    contract: &Contract,
    context: &RenderContext<'_>,
    name: &str,
    schema: &Value,
    pointer: &str,
    required: bool,
    diagnostics: &mut Vec<Diagnostic>,
    active: &mut BTreeSet<String>,
    depth: usize,
    html: &mut String,
) -> Result<(), Error> {
    inspect_unknown_schema_keys(contract, schema, pointer, diagnostics);
    html.push_str(&format!("<section class=\"property-card\" data-property-row data-property-name=\"{}\"><div class=\"property-summary\"><code class=\"property-name\">{}</code><span class=\"property-requirement {}\">{}</span><span class=\"property-type\">{}</span></div><div class=\"property-card-body\">", escape_attr(name), escape_text(name), if required { "required" } else { "optional" }, if required { "*" } else { "?" }, escape_text(&schema_type_label(schema))));
    if let Some(description) = schema.get("description").and_then(Value::as_str) {
        html.push_str(&format!(
            "<div class=\"property-description\">{}</div>",
            context.description(description)?
        ));
    }
    let mut chips = Vec::new();
    if let Some(format) = schema.get("format").and_then(Value::as_str) {
        chips.push(format!("format {format}"));
    }
    if let (Some(minimum), Some(maximum)) = (schema.get("minimum"), schema.get("maximum")) {
        chips.push(format!("{} ≤ value ≤ {}", minimum, maximum));
    } else {
        for key in [
            "minimum",
            "maximum",
            "exclusiveMinimum",
            "exclusiveMaximum",
            "minLength",
            "maxLength",
            "minItems",
            "maxItems",
        ] {
            if let Some(value) = schema.get(key) {
                chips.push(format!("{key} {value}"));
            }
        }
    }
    if let Some(values) = schema.get("enum").and_then(Value::as_array) {
        chips.push(format!("{} values", values.len()));
    }
    for flag in ["readOnly", "writeOnly", "deprecated"] {
        if schema.get(flag).and_then(Value::as_bool) == Some(true) {
            chips.push(flag.to_owned());
        }
    }
    if !chips.is_empty() {
        html.push_str("<div class=\"schema-property-chips\">");
        for chip in chips {
            html.push_str(&format!("<span>{}</span>", escape_text(&chip)));
        }
        html.push_str("</div>");
    }
    if let Some(values) = schema
        .get("examples")
        .and_then(Value::as_array)
        .filter(|values| !values.is_empty())
    {
        html.push_str(
            "<details class=\"property-examples\"><summary>Examples</summary><pre><code>",
        );
        html.push_str(&escape_text(&pretty_json(&Value::Array(values.clone()))));
        html.push_str("</code></pre></details>");
    } else if let Some(value) = schema.get("example") {
        html.push_str(&format!("<details class=\"property-examples\"><summary>Example</summary><pre><code>{}</code></pre></details>", escape_text(&pretty_json(value))));
    }
    if let Some(reference) = schema
        .get("$ref")
        .or_else(|| schema.get("$dynamicRef"))
        .and_then(Value::as_str)
    {
        html.push_str("<p class=\"schema-reference\">Schema: ");
        render_ref_link(contract, reference, context, html, pointer);
        html.push_str("</p>");
    }
    if schema.get("properties").is_some() {
        html.push_str("<details class=\"schema-nested\"><summary>Nested properties</summary>");
        render_compact_schema_card(
            contract,
            context,
            schema,
            pointer,
            diagnostics,
            active,
            depth + 1,
            html,
        )?;
        html.push_str("</details>");
    } else if let Some(items) = schema
        .get("items")
        .filter(|items| items.get("properties").is_some())
    {
        html.push_str("<details class=\"schema-nested\"><summary>Item properties</summary>");
        render_compact_schema_card(
            contract,
            context,
            items,
            &format!("{pointer}/items"),
            diagnostics,
            active,
            depth + 1,
            html,
        )?;
        html.push_str("</details>");
    }
    html.push_str("</div></section>");
    Ok(())
}

fn schema_type_label(schema: &Value) -> String {
    match schema.get("type") {
        Some(Value::String(value)) => value.clone(),
        Some(Value::Array(values)) => values
            .iter()
            .filter_map(Value::as_str)
            .collect::<Vec<_>>()
            .join(" | "),
        _ if schema.get("properties").is_some() => "object".to_owned(),
        _ if schema.get("items").is_some() => "array".to_owned(),
        _ => "schema".to_owned(),
    }
}

fn render_security_summary(contract: &Contract, operation: &Operation, html: &mut String) {
    let root = contract.document.as_object();
    let requirements = operation
        .value
        .get("security")
        .or_else(|| root.and_then(|object| object.get("security")));
    let Some(requirements) = requirements.and_then(Value::as_array) else {
        return;
    };
    html.push_str("<section class=\"operation-section auth-summary\"><h2>Authentication</h2><p>Choose one alternative. Schemes listed together are used in combination.</p><ul>");
    if requirements.is_empty() {
        html.push_str("<li>No authentication is required for this operation.</li>");
    }
    for (index, requirement) in requirements.iter().enumerate() {
        html.push_str(&format!("<li><strong>Alternative {}</strong>", index + 1));
        if let Some(schemes) = requirement.as_object() {
            if schemes.is_empty() {
                html.push_str("<span> No authentication.</span>");
            } else {
                html.push_str("<ul>");
                for (name, scopes) in schemes {
                    html.push_str(&format!("<li>{}", escape_text(name)));
                    if let Some(values) = scopes.as_array().filter(|values| !values.is_empty()) {
                        html.push_str(&format!(
                            " — scopes: {}",
                            values
                                .iter()
                                .filter_map(Value::as_str)
                                .map(escape_text)
                                .collect::<Vec<_>>()
                                .join(", ")
                        ));
                    }
                    html.push_str("</li>");
                }
                html.push_str("</ul>");
            }
        }
        html.push_str("</li>");
    }
    html.push_str("</ul></section>");
}

fn render_example_variants(
    contract: &Contract,
    operation: &Operation,
    media_type: &str,
    variants: &[crate::examples::ExampleVariant],
    request: bool,
    html: &mut String,
) {
    for variant in variants {
        html.push_str(&format!("<section class=\"example-variant\" data-example-panel data-example-label=\"{}\" data-media-type=\"{}\" data-example-provided=\"{}\"{}>", escape_attr(&variant.label), escape_attr(media_type), variant.provided, if request { "" } else { " data-response-example" }));
        html.push_str(&format!(
            "<div class=\"example-variant-heading\"><strong>{}</strong><span>{}</span></div>",
            escape_text(&variant.label),
            if variant.provided {
                "Contract-provided"
            } else {
                "Schema-derived"
            }
        ));
        if let Some(value) = &variant.value {
            let formatted = pretty_json(value);
            html.push_str(&format!(
                "<pre><code class=\"language-json\">{}</code></pre>",
                escape_text(&formatted)
            ));
            if request {
                html.push_str(&format!(
                    "<pre class=\"example-curl\"><code class=\"language-bash\">{}</code></pre>",
                    escape_text(&curl_example(contract, operation, media_type, &formatted))
                ));
            }
            html.push_str(&format!("<details class=\"example-provenance\"><summary>Example source and limitations</summary><p>{}</p><p>{}</p></details>", escape_text(&variant.note), if variant.provided { "Contract-provided values; validity is not independently asserted." } else { "Schema-derived reading aid; values are not validated or sent." }));
        } else if let Some(external) = &variant.unavailable {
            html.push_str(&format!(
                "<details class=\"example-provenance\"><summary>Example source and limitations</summary><p>{}</p><p>External example reference <code>{}</code> was not fetched.</p></details>",
                escape_text(&variant.note), escape_text(external)
            ));
        } else {
            html.push_str(&format!("<details class=\"example-provenance\"><summary>Example source and limitations</summary><p>{}</p></details>", escape_text(&variant.note)));
        }
        html.push_str("</section>");
    }
}

fn curl_example(
    contract: &Contract,
    operation: &Operation,
    media_type: &str,
    body: &str,
) -> String {
    let body = body.trim();
    let endpoint = operation_server(operation, contract).unwrap_or("https://api.example.invalid");
    let mut command = format!(
        "curl --request {} {}",
        operation.method,
        shell_quote(&format!(
            "{}{}",
            endpoint.trim_end_matches('/'),
            operation.path
        ))
    );
    if operation_uses_bearer(operation, contract) {
        command.push_str(" \\\n  --header 'Authorization: Bearer <token>'");
    }
    command.push_str(&format!(
        " \\\n  --header {} \\\n  --data {}",
        shell_quote(&format!("Content-Type: {media_type}")),
        shell_quote(body)
    ));
    command
}

fn operation_server<'a>(operation: &'a Operation, contract: &'a Contract) -> Option<&'a str> {
    operation
        .value
        .get("servers")
        .or_else(|| operation.path_item.get("servers"))
        .or_else(|| contract.document.get("servers"))
        .and_then(Value::as_array)
        .and_then(|servers| servers.first())
        .and_then(|server| server.get("url"))
        .and_then(Value::as_str)
}

fn operation_uses_bearer(operation: &Operation, contract: &Contract) -> bool {
    let requirements = operation
        .value
        .get("security")
        .or_else(|| contract.document.get("security"));
    let Some(requirements) = requirements.and_then(Value::as_array) else {
        return false;
    };
    let schemes = contract.document.pointer("/components/securitySchemes");
    requirements
        .first()
        .and_then(Value::as_object)
        .is_some_and(|requirement| {
            requirement.keys().any(|name| {
                schemes.and_then(|all| all.get(name)).is_some_and(|scheme| {
                    scheme.get("type").and_then(Value::as_str) == Some("http")
                        && scheme
                            .get("scheme")
                            .and_then(Value::as_str)
                            .is_some_and(|scheme| scheme.eq_ignore_ascii_case("bearer"))
                })
            })
        })
}

fn shell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}

fn inspect_unrendered_operation(
    contract: &Contract,
    operation: &Operation,
    diagnostics: &mut Vec<Diagnostic>,
) -> Result<(), Error> {
    inspect_unrendered_object(
        contract,
        &operation.path_item,
        &operation.path_item_pointer,
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
    inspect_unrendered_object(
        contract,
        &operation.value,
        &operation.pointer,
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

    for (source_pointer, parameter) in operation_parameters(operation) {
        let resolved =
            contract.resolve_component_object(parameter, &source_pointer, "parameters")?;
        inspect_unrendered_object(
            contract,
            resolved.value,
            &resolved.pointer,
            &[
                "$ref",
                "name",
                "in",
                "description",
                "required",
                "schema",
                "example",
                "examples",
                "content",
            ],
            "parameter feature",
            diagnostics,
        );
        if let Some(schema) = resolved.value.get("schema") {
            inspect_schema_features(
                contract,
                schema,
                &format!("{}/schema", resolved.pointer),
                diagnostics,
            );
        }
        inspect_media_features(
            contract,
            resolved.value.get("content"),
            &format!("{}/content", resolved.pointer),
            diagnostics,
        );
    }
    if let Some(request_body) = operation.value.get("requestBody") {
        let resolved = contract.resolve_component_object(
            request_body,
            &format!("{}/requestBody", operation.pointer),
            "requestBodies",
        )?;
        inspect_unrendered_object(
            contract,
            resolved.value,
            &resolved.pointer,
            &["$ref", "description", "required", "content"],
            "request-body feature",
            diagnostics,
        );
        inspect_media_features(
            contract,
            resolved.value.get("content"),
            &format!("{}/content", resolved.pointer),
            diagnostics,
        );
    }
    if let Some(responses) = operation.value.get("responses").and_then(Value::as_object) {
        for (status, response) in responses {
            let source_pointer = format!(
                "{}/responses/{}",
                operation.pointer,
                json_pointer_escape(status)
            );
            let resolved =
                contract.resolve_component_object(response, &source_pointer, "responses")?;
            inspect_unrendered_object(
                contract,
                resolved.value,
                &resolved.pointer,
                &["$ref", "description", "content"],
                "response feature",
                diagnostics,
            );
            inspect_media_features(
                contract,
                resolved.value.get("content"),
                &format!("{}/content", resolved.pointer),
                diagnostics,
            );
        }
    }
    Ok(())
}

fn operation_parameters(operation: &Operation) -> Vec<(String, &Value)> {
    let mut values = Vec::new();
    for (pointer, value) in [
        (operation.path_item_pointer.as_str(), &operation.path_item),
        (operation.pointer.as_str(), &operation.value),
    ] {
        if let Some(parameters) = value.get("parameters").and_then(Value::as_array) {
            for (index, parameter) in parameters.iter().enumerate() {
                values.push((format!("{pointer}/parameters/{index}"), parameter));
            }
        }
    }
    values
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
        let media_pointer = format!("{pointer}/{}", json_pointer_escape(media_type));
        inspect_unrendered_object(
            contract,
            value,
            &media_pointer,
            &["schema", "example", "examples"],
            "media-type feature",
            diagnostics,
        );
        if let Some(schema) = value.get("schema") {
            inspect_schema_features(
                contract,
                schema,
                &format!("{media_pointer}/schema"),
                diagnostics,
            );
        }
    }
}

fn inspect_schema_features(
    contract: &Contract,
    schema: &Value,
    pointer: &str,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let Some(object) = schema.as_object() else {
        return;
    };
    inspect_unknown_schema_keys(contract, schema, pointer, diagnostics);
    if let Some(properties) = object.get("properties").and_then(Value::as_object) {
        for (name, child) in properties {
            inspect_schema_features(
                contract,
                child,
                &format!("{pointer}/properties/{}", json_pointer_escape(name)),
                diagnostics,
            );
        }
    }
    for key in ["items", "not"] {
        if let Some(child) = object.get(key) {
            inspect_schema_features(contract, child, &format!("{pointer}/{key}"), diagnostics);
        }
    }
    if contract.is_openapi_31 {
        for key in ["contains", "propertyNames", "if", "then", "else"] {
            if let Some(child) = object.get(key) {
                inspect_schema_features(contract, child, &format!("{pointer}/{key}"), diagnostics);
            }
        }
        for key in ["patternProperties", "$defs", "dependentSchemas"] {
            if let Some(children) = object.get(key).and_then(Value::as_object) {
                for (name, child) in children {
                    inspect_schema_features(
                        contract,
                        child,
                        &format!("{pointer}/{key}/{}", json_pointer_escape(name)),
                        diagnostics,
                    );
                }
            }
        }
    }
    if let Some(child) = object
        .get("additionalProperties")
        .filter(|value| value.is_object())
    {
        inspect_schema_features(
            contract,
            child,
            &format!("{pointer}/additionalProperties"),
            diagnostics,
        );
    }
    for key in ["allOf", "oneOf", "anyOf", "prefixItems"] {
        if key == "prefixItems" && !contract.is_openapi_31 {
            continue;
        }
        if let Some(children) = object.get(key).and_then(Value::as_array) {
            for (index, child) in children.iter().enumerate() {
                inspect_schema_features(
                    contract,
                    child,
                    &format!("{pointer}/{key}/{index}"),
                    diagnostics,
                );
            }
        }
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

fn render_resolved_source_details(kind: &str, resolved: &ResolvedObject<'_>, html: &mut String) {
    let chain = resolved.chain.join(" → ");
    html.push_str(&format!(
        "<details><summary>Resolved {kind} source at <code>{}</code></summary><p>Reference chain: <code>{}</code></p><pre><code>{}</code></pre></details>",
        escape_text(&resolved.pointer),
        escape_text(&chain),
        escape_text(&pretty_json(resolved.value))
    ));
}

pub(crate) fn navigation_data(links: &[NavigationLink], base_path: &str) -> Vec<NavigationLink> {
    links
        .iter()
        .map(|link| NavigationLink {
            label: link.label.clone(),
            href: if link.href.starts_with('/') {
                format!("{base_path}{}", link.href)
            } else {
                link.href.clone()
            },
            icon: link.icon,
        })
        .collect()
}

#[derive(Debug, Serialize)]
pub(crate) struct SidebarLinkData {
    pub label: String,
    pub href: String,
    pub icon: Option<NavigationIcon>,
    pub initial: String,
}

#[derive(Debug, Serialize)]
pub(crate) struct SidebarGroupData {
    pub title: String,
    pub links: Vec<SidebarLinkData>,
}

fn sidebar_link_data(link: &NavigationLink, base_path: &str) -> SidebarLinkData {
    SidebarLinkData {
        label: link.label.clone(),
        href: if link.href.starts_with('/') {
            format!("{base_path}{}", link.href)
        } else {
            link.href.clone()
        },
        icon: link.icon,
        initial: link.label.chars().next().unwrap_or('?').to_string(),
    }
}

pub(crate) fn section_data(links: &[NavigationLink], base_path: &str) -> Vec<SidebarLinkData> {
    links
        .iter()
        .map(|link| sidebar_link_data(link, base_path))
        .collect()
}

pub(crate) fn sidebar_data(
    groups: &[SidebarGroup],
    fallback: &[NavigationLink],
    base_path: &str,
) -> Vec<SidebarGroupData> {
    let groups = if groups.is_empty() {
        vec![SidebarGroup {
            title: "Documentation".to_owned(),
            links: fallback.to_vec(),
        }]
    } else {
        groups.to_vec()
    };
    groups
        .into_iter()
        .map(|group| SidebarGroupData {
            title: group.title,
            links: group
                .links
                .iter()
                .map(|link| sidebar_link_data(link, base_path))
                .collect(),
        })
        .collect()
}

pub(crate) fn render_navigation() -> String {
    String::from(
        r#"<aside id="nd-sidebar" data-collapsed="false" class="reference-sidebar [grid-area:sidebar] flex min-h-0 overflow-clip text-sm md:layout:[--fd-sidebar-width:268px] max-md:hidden" aria-label="Documentation navigation"><div class="reference-sidebar-inner flex flex-col shrink-0 w-(--fd-sidebar-width)"><div class="sidebar-header flex items-center gap-2 h-(--fd-header-height) my-2 ps-5.5 pe-3"><a class="sidebar-home inline-flex items-center gap-2 min-w-0 me-auto text-[0.9375rem] font-semibold" href="{{ site.data.akibiyori_site.home_href | escape }}">{% if site.data.akibiyori_site.icon_href %}<img class="brand-icon" src="{{ site.data.akibiyori_site.icon_href | escape }}" width="20" height="20" alt="" aria-hidden="true">{% endif %}<span>{{ site.title | escape }}</span></a><button class="sidebar-collapse inline-flex items-center justify-center rounded-md p-1.5 text-sm font-medium transition-colors duration-100 hover:bg-fd-accent hover:text-fd-accent-foreground size-8 text-fd-muted-foreground" type="button" aria-label="Collapse navigation" aria-controls="nd-sidebar" aria-expanded="true"><svg aria-hidden="true" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="lucide lucide-panel-left size-4.5"><rect width="18" height="18" x="3" y="3" rx="2"></rect><path d="M9 3v18"></path></svg></button></div><div class="sidebar-controls flex flex-col gap-2 px-3 empty:hidden">{% if site.data.akibiyori_sections.size > 0 %}<details class="sidebar-sections" data-sidebar-sections><summary class="sidebar-section-summary flex w-full h-10 items-center gap-3 rounded-xl border bg-fd-secondary/50 ps-[calc(--spacing(2.5)-1px)] pe-2.5 text-start font-medium outline-none transition-colors hover:bg-fd-accent focus-visible:ring-2 focus-visible:ring-fd-ring"><span class="sidebar-section-summary-icon sidebar-icon-initial" data-section-current-icon aria-hidden="true">S</span><span class="sidebar-section-summary-label" data-section-current-label>Sections</span><svg aria-hidden="true" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="lucide lucide-chevrons-up-down"><path d="m7 15 5 5 5-5"></path><path d="m7 9 5-5 5 5"></path></svg></summary><nav class="sidebar-section-menu" aria-label="Documentation sections"><ul>{% for section in site.data.akibiyori_sections %}<li><a data-sidebar-section href="{{ section.href | escape }}"><span class="sidebar-link-icon" data-sidebar-link-icon aria-hidden="true">{% case section.icon %}{% when 'rocket' %}<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="lucide lucide-rocket"><path d="M12 15v5s3.03-.55 4-2c1.08-1.62 0-5 0-5"></path><path d="M4.5 16.5c-1.5 1.26-2 5-2 5s3.74-.5 5-2c.71-.84.7-2.13-.09-2.91a2.18 2.18 0 0 0-2.91-.09"></path><path d="M9 12a22 22 0 0 1 2-3.95A12.88 12.88 0 0 1 22 2c0 2.72-.78 7.5-6 11a22.4 22.4 0 0 1-4 2z"></path><path d="M9 12H4s.55-3.03 2-4c1.62-1.08 5 .05 5 .05"></path></svg>{% when 'circle-help' %}<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="lucide lucide-circle-help"><circle cx="12" cy="12" r="10"></circle><path d="M9.09 9a3 3 0 0 1 5.83 1c0 2-3 3-3 3"></path><path d="M12 17h.01"></path></svg>{% when 'book-open' %}<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="lucide lucide-book-open"><path d="M12 5v16"></path><path d="M20.001 19A2 2 0 0 0 22 17V5a2 2 0 0 0-1.999-2L16 3.002A5 5 0 0 0 12 5a5 5 0 0 0-4-2H4a2 2 0 0 0-2 2v12a2 2 0 0 0 1.999 2H8a5 5 0 0 1 4 2 5 5 0 0 1 4-2z"></path></svg>{% else %}<span class="sidebar-icon-initial">{{ section.initial | escape }}</span>{% endcase %}</span><span data-section-label>{{ section.label | escape }}</span></a>{% endfor %}</ul></nav></details>{% endif %}<a class="sidebar-search inline-flex items-center border bg-fd-secondary/50 p-1.5 ps-2 text-sm text-fd-muted-foreground transition-colors hover:bg-fd-accent hover:text-fd-accent-foreground h-10 gap-3 rounded-xl pe-2" href="{{ site.data.akibiyori_site.search_href | escape }}" aria-label="Search documentation"><svg aria-hidden="true" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="lucide lucide-search size-4"><circle cx="11" cy="11" r="8"></circle><path d="m21 21-4.34-4.34"></path></svg><span>Search</span></a></div><div class="sidebar-viewport min-h-0 flex-1 mt-2 overflow-auto px-3 pt-4 pb-8"><nav aria-label="Documentation">{% for group in site.data.akibiyori_sidebar %}<section class="sidebar-group"><h2>{{ group.title | escape }}</h2><ul>{% for link in group.links %}<li><a href="{{ link.href | escape }}" data-sidebar-page-link>{% if link.icon %}<span class="sidebar-link-icon" aria-hidden="true">{% case link.icon %}{% when 'rocket' %}<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="lucide lucide-rocket"><path d="M12 15v5s3.03-.55 4-2c1.08-1.62 0-5 0-5"></path><path d="M4.5 16.5c-1.5 1.26-2 5-2 5s3.74-.5 5-2c.71-.84.7-2.13-.09-2.91a2.18 2.18 0 0 0-2.91-.09"></path><path d="M9 12a22 22 0 0 1 2-3.95A12.88 12.88 0 0 1 22 2c0 2.72-.78 7.5-6 11a22.4 22.4 0 0 1-4 2z"></path><path d="M9 12H4s.55-3.03 2-4c1.62-1.08 5 .05 5 .05"></path></svg>{% when 'circle-help' %}<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="lucide lucide-circle-help"><circle cx="12" cy="12" r="10"></circle><path d="M9.09 9a3 3 0 0 1 5.83 1c0 2-3 3-3 3"></path><path d="M12 17h.01"></path></svg>{% when 'book-open' %}<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="lucide lucide-book-open"><path d="M12 5v16"></path><path d="M20.001 19A2 2 0 0 0 22 17V5a2 2 0 0 0-1.999-2L16 3.002A5 5 0 0 0 12 5a5 5 0 0 0-4-2H4a2 2 0 0 0-2 2v12a2 2 0 0 0 1.999 2H8a5 5 0 0 1 4 2 5 5 0 0 1 4-2z"></path></svg>{% else %}<span class="sidebar-icon-initial">{{ link.initial | escape }}</span>{% endcase %}</span>{% endif %}<span>{{ link.label | escape }}</span></a></li>{% endfor %}</ul></section>{% endfor %}</nav></div></div></aside>"#,
    )
}

pub(crate) fn render_page(
    title: &str,
    route: &str,
    body: &str,
    layout: &str,
    context: &RenderContext<'_>,
) -> String {
    let breadcrumbs = render_breadcrumbs(title, route, context);
    let target = route_target(route);
    format!(
        "---\ntitle: {}\npermalink: {}\nlayout: {}\ntemplated: false\ndata:\n  reference_layout: source\n---\n<div class=\"reference-context\">{}</div>\n\n{}",
        super::yaml_quote(title),
        super::yaml_quote(&format!("/{target}")),
        super::yaml_quote(layout),
        breadcrumbs,
        body,
    )
}

pub(crate) fn render_document_page(
    title: &str,
    route: &str,
    body: &str,
    document: &DocumentHeader,
) -> String {
    let description = document
        .description
        .as_deref()
        .filter(|description| !description.trim().is_empty());
    let mut markdown = format!("# {title}\n\n");
    if let Some(description) = description {
        markdown.push_str(description);
        markdown.push_str("\n\n");
    }
    markdown.push_str(body.trim_start());

    let mut header = format!(
        "<header class=\"document-header\"><h1 class=\"document-title text-[1.75em] font-semibold\">{}</h1>",
        escape_text(title)
    );
    if let Some(description) = description {
        header.push_str(&format!(
            "<p class=\"document-description text-lg text-fd-muted-foreground mb-2\">{}</p>",
            escape_text(description)
        ));
    }
    header.push_str("<div class=\"document-actions flex flex-row flex-wrap gap-2 items-center border-b pb-6 mb-4\">");
    header.push_str("<button class=\"document-copy inline-flex items-center justify-center rounded-md p-2 text-sm font-medium transition-colors duration-100 disabled:pointer-events-none disabled:opacity-50 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-fd-ring border bg-fd-secondary text-fd-secondary-foreground hover:bg-fd-accent hover:text-fd-accent-foreground gap-1 px-2 py-1.5 text-xs [&amp;_svg]:size-3.5 [&amp;_svg]:text-fd-muted-foreground\" type=\"button\" data-copy-markdown><svg aria-hidden=\"true\" viewBox=\"0 0 24 24\" fill=\"none\" stroke=\"currentColor\" stroke-width=\"2\" stroke-linecap=\"round\" stroke-linejoin=\"round\"><rect width=\"14\" height=\"14\" x=\"8\" y=\"8\" rx=\"2\"></rect><path d=\"M4 16c-1.1 0-2-.9-2-2V4c0-1.1.9-2 2-2h10c1.1 0 2 .9 2 2\"></path></svg>Copy Markdown</button>");
    if let Some(source_url) = &document.source_url {
        header.push_str(&format!(
            "<a class=\"document-source inline-flex items-center justify-center rounded-md p-2 text-sm font-medium transition-colors duration-100 disabled:pointer-events-none disabled:opacity-50 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-fd-ring border bg-fd-secondary text-fd-secondary-foreground hover:bg-fd-accent hover:text-fd-accent-foreground gap-1 px-2 py-1.5 text-xs\" href=\"{}\" rel=\"noopener noreferrer\">Open source</a>",
            escape_attr(source_url)
        ));
    }
    header.push_str("<span class=\"document-copy-status\" role=\"status\" aria-live=\"polite\"></span></div><textarea hidden data-markdown-source data-pagefind-ignore>");
    header.push_str(&escape_textarea_payload(&markdown));
    header.push_str("</textarea></header>");

    format!(
        "---\ntitle: {}\npermalink: {}\nlayout: default.liquid\ntemplated: false\ndata:\n  document_header: true\n  reference_layout: source\n---\n{}\n<div class=\"prose flex-1 text-fd-foreground/90\">\n\n{}\n\n</div>",
        super::yaml_quote(title),
        super::yaml_quote(&format!("/{}", route_target(route))),
        header,
        body
    )
}

pub(crate) fn route_target(route: &str) -> String {
    if route == "/" {
        "index.html".to_owned()
    } else {
        format!("{}index.html", route.trim_start_matches('/'))
    }
}

fn render_breadcrumbs(title: &str, route: &str, context: &RenderContext<'_>) -> String {
    let mut crumbs = String::from(
        "<nav class=\"breadcrumbs\" aria-label=\"Breadcrumb\" data-pagefind-ignore><ol>",
    );
    let home_href = if context.has_home {
        context.href("/")
    } else {
        context.href("/reference/")
    };
    let home_label = if context.has_home {
        "Home"
    } else {
        "API reference"
    };
    if route != "/" && (context.has_home || !route.starts_with("/reference/")) {
        crumbs.push_str(&format!(
            "<li><a href=\"{}\">{}</a></li>",
            escape_attr(&home_href),
            home_label
        ));
    }
    if route.starts_with("/reference/") && route != "/reference/" {
        if route == "/reference/models/" {
            crumbs.push_str(&format!(
                "<li><a href=\"{}\">API reference</a></li>",
                escape_attr(&context.href("/reference/"))
            ));
        } else if route.starts_with("/reference/models/") {
            crumbs.push_str(&format!(
                "<li><a href=\"{}\">API reference</a></li><li><a href=\"{}\">Models</a></li>",
                escape_attr(&context.href("/reference/")),
                escape_attr(&context.href("/reference/models/"))
            ));
        } else {
            crumbs.push_str(&format!(
                "<li><a href=\"{}\">API reference</a></li>",
                escape_attr(&context.href("/reference/"))
            ));
        }
    }
    crumbs.push_str(&format!(
        "<li aria-current=\"page\">{}</li></ol></nav>\n\n",
        escape_text(title)
    ));
    crumbs
}

fn page_file(
    route: &str,
    title: &str,
    body: &str,
    context: &RenderContext<'_>,
) -> Result<super::PendingFile, Error> {
    validate_route(route)?;
    Ok(super::PendingFile {
        path: generated_source_file(route),
        contents: render_page(title, route, body, "default.liquid", context).into_bytes(),
    })
}

fn render_model(
    contract: &Contract,
    name: &str,
    schema: &Value,
    context: &RenderContext<'_>,
    diagnostics: &mut Vec<Diagnostic>,
) -> Result<String, Error> {
    let mut html = format!(
        "<p class=\"eyebrow\">{} contract · model</p><h1>{}</h1>",
        escape_text(&contract.label),
        escape_text(name)
    );
    let root_pointer = format!("#/components/schemas/{}", json_pointer_escape(name));
    html.push_str(&format!(
        "<p>Source: <code>{}</code></p>",
        escape_text(&format!("{}{}", contract.source, root_pointer))
    ));
    let mut seen = BTreeSet::new();
    SchemaRenderer {
        contract,
        context,
        html: &mut html,
        seen: &mut seen,
        diagnostics,
    }
    .render_node(schema, "", &root_pointer)?;
    if contract.is_openapi_31 {
        html.push_str(
            "<section class=\"schema-resource-reading\"><h2>Schema resource reading</h2>",
        );
        let mut seen = BTreeSet::new();
        let mut remaining = 512;
        render_schema_resource_details(
            contract,
            context,
            schema,
            &root_pointer,
            &[],
            &mut seen,
            0,
            &mut remaining,
            &mut html,
        )?;
        html.push_str("</section>");
    }
    html.push_str(&format!(
        "<section><h2 id=\"source-{}\">{}</h2><pre><code>",
        stable_anchor(&root_pointer),
        match contract.input_format {
            crate::input::InputFormat::Json => "Original schema JSON",
            crate::input::InputFormat::Yaml => "JSON representation parsed from YAML",
        },
    ));
    html.push_str(&escape_text(&pretty_json(schema)));
    html.push_str("</code></pre></section>");
    Ok(html)
}

struct SchemaRenderer<'a, 'b> {
    contract: &'a Contract,
    context: &'a RenderContext<'b>,
    html: &'a mut String,
    seen: &'a mut BTreeSet<String>,
    diagnostics: &'a mut Vec<Diagnostic>,
}

impl SchemaRenderer<'_, '_> {
    fn render_node(&mut self, schema: &Value, prefix: &str, pointer: &str) -> Result<(), Error> {
        if !self.seen.insert(pointer.to_owned()) {
            return Ok(());
        }
        inspect_unknown_schema_keys(self.contract, schema, pointer, self.diagnostics);
        render_schema_facts(schema, self.context, self.html)?;
        if let Some(reference) = schema.get("$ref").and_then(Value::as_str) {
            render_ref_link(self.contract, reference, self.context, self.html, pointer);
            return Ok(());
        }
        let Some(object) = schema.as_object() else {
            if let Some(allowed) = schema.as_bool() {
                if pointer.ends_with("/additionalProperties") {
                    self.html.push_str(if allowed {
                        "<p>Boolean schema for additional properties: accepts all values.</p>"
                    } else {
                        "<p>Boolean schema for additional properties: accepts no values.</p>"
                    });
                } else {
                    self.html.push_str(if allowed {
                        "<p>Boolean schema: accepts all instances.</p>"
                    } else {
                        "<p>Boolean schema: accepts no instances.</p>"
                    });
                }
            }
            return Ok(());
        };

        if let Some(properties) = object.get("properties").and_then(Value::as_object) {
            let required: BTreeSet<&str> = object
                .get("required")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
                .filter_map(Value::as_str)
                .collect();
            self.html.push_str("<section class=\"fields\">");
            if self.context.reference.layout == ReferenceLayout::Columns {
                self.html.push_str("<div class=\"schema-fields-header\"><h2>Fields</h2><label><span class=\"visually-hidden\">Filter Properties</span><input type=\"search\" class=\"property-filter\" data-property-filter disabled autocomplete=\"off\" aria-label=\"Filter Properties\" placeholder=\"Filter Properties\"></label></div><p class=\"property-filter-empty\" data-property-filter-empty hidden>No properties match this filter.</p>");
            } else {
                self.html.push_str("<h2>Fields</h2>");
            }
            for (field, property) in properties {
                let full_path = if prefix.is_empty() {
                    field.clone()
                } else {
                    format!("{prefix}.{field}")
                };
                let field_pointer = format!("{pointer}/properties/{}", json_pointer_escape(field));
                let anchor = stable_anchor(&field_pointer);
                self.html.push_str(&format!(
                    "<section class=\"field\"{}><h3 id=\"{anchor}\">{}{}</h3>",
                    if self.context.reference.layout == ReferenceLayout::Columns {
                        format!(
                            " data-property-row data-property-name=\"{}\"",
                            escape_attr(field)
                        )
                    } else {
                        String::new()
                    },
                    escape_text(&full_path),
                    if required.contains(field.as_str()) {
                        " <span class=\"required\">required</span>"
                    } else {
                        ""
                    }
                ));
                self.render_node(property, &full_path, &field_pointer)?;
                self.html.push_str("</section>");
            }
            self.html.push_str("</section>");
        }

        if let Some(items) = object.get("items") {
            self.html.push_str("<section><h2>Array items</h2>");
            self.render_node(items, prefix, &format!("{pointer}/items"))?;
            self.html.push_str("</section>");
        }
        if let Some(additional) = object.get("additionalProperties") {
            self.html
                .push_str("<section><h2>Additional properties</h2>");
            self.render_node(
                additional,
                prefix,
                &format!("{pointer}/additionalProperties"),
            )?;
            self.html.push_str("</section>");
        }
        for (keyword, heading, explanation) in [
            ("allOf", "allOf", "All schemas must apply"),
            ("oneOf", "oneOf", "Exactly one schema must apply"),
            ("anyOf", "anyOf", "One or more schemas must apply"),
        ] {
            if let Some(alternatives) = object.get(keyword).and_then(Value::as_array) {
                self.html
                    .push_str(&format!("<section><h2>{heading}: {explanation}</h2><ul>"));
                for (index, alternative) in alternatives.iter().enumerate() {
                    let alt_pointer = format!("{pointer}/{keyword}/{index}");
                    self.html
                        .push_str(&format!("<li><strong>Alternative {}</strong>", index + 1));
                    self.render_node(alternative, prefix, &alt_pointer)?;
                    self.html.push_str("</li>");
                }
                self.html.push_str("</ul></section>");
            }
        }
        Ok(())
    }
}

#[allow(clippy::too_many_arguments)] // The recursive walk carries its scope, cycle set, and bounds explicitly.
fn render_schema_resource_details(
    contract: &Contract,
    context: &RenderContext<'_>,
    schema: &Value,
    pointer: &str,
    inherited_scope: &[String],
    active: &mut BTreeSet<String>,
    depth: usize,
    remaining: &mut usize,
    html: &mut String,
) -> Result<(), Error> {
    if *remaining == 0 {
        html.push_str("<p>Schema detail expansion stopped at its bounded reading limit.</p>");
        return Ok(());
    }
    if depth >= 32 {
        html.push_str(&format!(
            "<p>Further schema expansion is available from source location <code>{}</code>.</p>",
            escape_text(pointer)
        ));
        return Ok(());
    }
    let mut scope = if inherited_scope.is_empty() {
        contract.schema_resources.lexical_scope(pointer)
    } else {
        inherited_scope.to_vec()
    };
    let current_resource = contract.schema_resources.resource_for_pointer(pointer);
    if !scope.contains(&current_resource) {
        scope.push(current_resource);
    }
    let active_key = format!("{pointer}\0{}", scope.join("\0"));
    if !active.insert(active_key.clone()) {
        html.push_str(&format!(
            "<p>Recursive schema source at <code>{}</code>.</p>",
            escape_text(pointer)
        ));
        return Ok(());
    }
    *remaining -= 1;
    let result = (|| {
        if let Some(boolean) = schema.as_bool() {
            html.push_str(if boolean {
                "<p>Boolean schema: accepts all instances.</p>"
            } else {
                "<p>Boolean schema: accepts no instances.</p>"
            });
            return Ok(());
        }
        let Some(object) = schema.as_object() else {
            return Ok(());
        };
        render_schema_facts(schema, context, html)?;
        if let Some(id) = object.get("$id").and_then(Value::as_str)
            && !id.starts_with("file:")
        {
            html.push_str(&format!(
                "<p>Schema resource identifier: <code>{}</code></p>",
                escape_text(id)
            ));
        }
        for (keyword, dynamic) in [("$ref", false), ("$dynamicRef", true)] {
            let Some(reference) = object.get(keyword).and_then(Value::as_str) else {
                continue;
            };
            let target = contract.resolve_schema_reference(pointer, reference, dynamic, &scope)?;
            let target_value = contract
                .schema_resources
                .schema_at(&contract.document, &target.pointer)
                .ok_or_else(|| {
                    Error::Invalid(format!(
                        "resolved schema target {} is missing",
                        target.pointer
                    ))
                })?;
            let binding = if dynamic {
                if target.dynamic {
                    "Dynamic binding"
                } else {
                    "Dynamic reference static fallback"
                }
            } else {
                "Schema reference"
            };
            html.push_str(&format!(
                "<p>{binding}: <code>{}</code> → {}{}</p>",
                escape_text(reference),
                schema_target_link(contract, context, &target.pointer),
                if target.dynamic {
                    " (outer dynamic anchor)"
                } else {
                    ""
                }
            ));
            let mut target_scope = scope.clone();
            if !target_scope.contains(&target.uri) {
                target_scope.push(target.uri.clone());
            }
            html.push_str("<details><summary>Resolved schema facts</summary>");
            render_schema_resource_details(
                contract,
                context,
                target_value,
                &target.pointer,
                &target_scope,
                active,
                depth + 1,
                remaining,
                html,
            )?;
            html.push_str("</details>");
        }
        for key in [
            "properties",
            "patternProperties",
            "$defs",
            "dependentSchemas",
        ] {
            if let Some(children) = object.get(key).and_then(Value::as_object) {
                html.push_str(&format!(
                    "<section><h4>{}</h4>",
                    if key == "$defs" {
                        "Definitions (not applied automatically)"
                    } else {
                        key
                    }
                ));
                for (name, child) in children {
                    let child_pointer = format!("{pointer}/{key}/{}", json_pointer_escape(name));
                    html.push_str(&format!("<section><h5>{}: <code>{}</code></h5><p>Schema source location: <code>{}</code></p>", escape_text(key.trim_start_matches('$')), escape_text(name), escape_text(&child_pointer)));
                    render_schema_resource_details(
                        contract,
                        context,
                        child,
                        &child_pointer,
                        &scope,
                        active,
                        depth + 1,
                        remaining,
                        html,
                    )?;
                    html.push_str("</section>");
                }
                html.push_str("</section>");
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
            if let Some(child) = object.get(key) {
                html.push_str(&format!("<section><h4>{}</h4>", escape_text(key)));
                render_schema_resource_details(
                    contract,
                    context,
                    child,
                    &format!("{pointer}/{key}"),
                    &scope,
                    active,
                    depth + 1,
                    remaining,
                    html,
                )?;
                html.push_str("</section>");
            }
        }
        for key in ["allOf", "oneOf", "anyOf", "prefixItems"] {
            if let Some(children) = object.get(key).and_then(Value::as_array) {
                html.push_str(&format!("<section><h4>{}</h4>", escape_text(key)));
                for (index, child) in children.iter().enumerate() {
                    let child_pointer = format!("{pointer}/{key}/{index}");
                    html.push_str(&format!("<section><h5>{} alternative {}</h5><p>Schema source location: <code>{}</code></p>", escape_text(key), index + 1, escape_text(&child_pointer)));
                    render_schema_resource_details(
                        contract,
                        context,
                        child,
                        &child_pointer,
                        &scope,
                        active,
                        depth + 1,
                        remaining,
                        html,
                    )?;
                    html.push_str("</section>");
                }
                html.push_str("</section>");
            }
        }
        Ok(())
    })();
    active.remove(&active_key);
    result
}

fn schema_target_link(contract: &Contract, context: &RenderContext<'_>, pointer: &str) -> String {
    if let Some(name) = pointer.strip_prefix("#/components/schemas/") {
        let name = name
            .split('/')
            .next()
            .unwrap_or(name)
            .replace("~1", "/")
            .replace("~0", "~");
        if contract.schemas.contains_key(&name) {
            let href = context.href(&format!("/reference/models/{}/{name}/", contract.id));
            return format!(
                "<a href=\"{}\">{}</a> <code>{}</code>",
                escape_attr(&href),
                escape_text(&name),
                escape_text(pointer)
            );
        }
    }
    format!("<code>{}</code>", escape_text(pointer))
}

fn render_schema_facts(
    schema: &Value,
    context: &RenderContext<'_>,
    html: &mut String,
) -> Result<(), Error> {
    for keyword in ["type", "format", "title", "description"] {
        if let Some(value) = schema.get(keyword) {
            let value = value
                .as_str()
                .map(str::to_owned)
                .unwrap_or_else(|| value.to_string());
            if keyword == "description" {
                if context.reference.descriptions == DescriptionMode::Markdown {
                    html.push_str(&context.description(&value)?);
                } else {
                    html.push_str(&format!(
                        "<p><strong>description</strong>: {}</p>",
                        escape_text(&value)
                    ));
                }
            } else {
                html.push_str(&format!(
                    "<p><strong>{}</strong>: {}</p>",
                    escape_text(keyword),
                    escape_text(&value)
                ));
            }
        }
    }
    render_constraints(schema, html);
    Ok(())
}

fn inspect_unknown_schema_keys(
    contract: &Contract,
    schema: &Value,
    pointer: &str,
    diagnostics: &mut Vec<Diagnostic>,
) {
    // This list is the exact schema vocabulary represented by render_schema_facts
    // and render_schema_node. Other keywords stay visible in raw JSON and diagnose.
    const RENDERED: &[&str] = &[
        "$ref",
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
        "enum",
        "nullable",
        "default",
        "example",
        "readOnly",
        "writeOnly",
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
    ];
    let Some(object) = schema.as_object() else {
        return;
    };
    for key in object.keys().filter(|key| {
        !(RENDERED.contains(&key.as_str())
            || contract.is_openapi_31
                && matches!(
                    key.as_str(),
                    "$id"
                        | "$schema"
                        | "$anchor"
                        | "$dynamicAnchor"
                        | "$dynamicRef"
                        | "$defs"
                        | "const"
                        | "examples"
                        | "not"
                        | "contains"
                        | "propertyNames"
                        | "if"
                        | "then"
                        | "else"
                        | "dependentSchemas"
                        | "patternProperties"
                        | "prefixItems"
                ))
    }) {
        diagnostics.push(Diagnostic {
            contract: contract.id.clone(),
            pointer: format!("{pointer}/{}", json_pointer_escape(key)),
            message: format!("schema keyword {key:?} is retained in original JSON but not interpreted in the concise view"),
        });
    }
}

fn render_constraints(schema: &Value, html: &mut String) {
    const CONSTRAINTS: &[&str] = &[
        "nullable",
        "const",
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
        "examples",
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
) -> Result<String, Error> {
    let mut html = format!(
        "<p class=\"eyebrow\">{} contract · operation</p><h1>{}</h1>",
        escape_text(&contract.label),
        escape_text(operation.id.as_str())
    );
    html.push_str(&match &operation.kind {
        OperationKind::Path => format!("<p><code>{} {}</code></p>", escape_text(&operation.method), escape_text(&operation.path)),
        OperationKind::Webhook { name } => format!("<p><strong>Webhook</strong> <code>{}</code> · method <code>{}</code> · the delivery destination is configured by the caller</p>", escape_text(name), escape_text(&operation.method)),
    });
    if let Some(summary) = operation.value.get("summary").and_then(Value::as_str) {
        html.push_str(&format!("<p>{}</p>", escape_text(summary)));
    }
    if let Some(description) = operation.value.get("description").and_then(Value::as_str) {
        html.push_str(&context.description(description)?);
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
    let parameters = effective_parameters(operation, contract)?;
    if !parameters.is_empty() {
        html.push_str("<section><h2>Parameters</h2><dl>");
        for parameter in parameters {
            let value = parameter.value;
            let name = value
                .get("name")
                .and_then(Value::as_str)
                .unwrap_or("(unnamed)");
            let location = value.get("in").and_then(Value::as_str).unwrap_or("unknown");
            html.push_str(&format!(
                "<dt>{} <span class=\"muted\">in {}</span>{}</dt><dd>",
                escape_text(name),
                escape_text(location),
                if value
                    .get("required")
                    .and_then(Value::as_bool)
                    .unwrap_or(false)
                {
                    " <span class=\"required\">required</span>"
                } else {
                    ""
                }
            ));
            if let Some(description) = value.get("description").and_then(Value::as_str) {
                html.push_str(&context.description(description)?);
            }
            if let Some(schema) = value.get("schema") {
                html.push_str(&format!(
                    "<p>Schema: {}</p>",
                    summarize_refs(schema, contract, context)
                ));
                html.push_str(&format!(
                    "<pre><code>{}</code></pre>",
                    escape_text(&pretty_json(schema))
                ));
            }
            render_resolved_source_details("parameter", &parameter, &mut html);
            html.push_str("</dd>");
        }
        html.push_str("</dl></section>");
    }
    if let Some(request_body) = operation.value.get("requestBody") {
        let resolved = contract.resolve_component_object(
            request_body,
            &format!("{}/requestBody", operation.pointer),
            "requestBodies",
        )?;
        html.push_str("<section><h2>Request body</h2>");
        if let Some(description) = resolved.value.get("description").and_then(Value::as_str) {
            html.push_str(&context.description(description)?);
        }
        let required = resolved
            .value
            .get("required")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        html.push_str(&format!(
            "<p>Required: <strong>{required}</strong>{}</p>",
            if resolved.value.get("required").is_none() {
                " (default)"
            } else {
                ""
            }
        ));
        render_content(
            resolved.value.get("content"),
            &format!("{}/content", resolved.pointer),
            contract,
            context,
            &mut html,
        )?;
        render_resolved_source_details("request body", &resolved, &mut html);
        html.push_str("</section>");
    }
    if let Some(responses) = operation.value.get("responses").and_then(Value::as_object) {
        html.push_str("<section><h2>Responses</h2>");
        for (status, response) in responses {
            let resolved = contract.resolve_component_object(
                response,
                &format!(
                    "{}/responses/{}",
                    operation.pointer,
                    json_pointer_escape(status)
                ),
                "responses",
            )?;
            html.push_str(&format!("<h3>Status {}</h3>", escape_text(status)));
            if let Some(description) = resolved.value.get("description").and_then(Value::as_str) {
                html.push_str(&context.description(description)?);
            }
            render_content(
                resolved.value.get("content"),
                &format!("{}/content", resolved.pointer),
                contract,
                context,
                &mut html,
            )?;
            render_resolved_source_details("response", &resolved, &mut html);
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
    Ok(html)
}

fn effective_parameters<'a>(
    operation: &'a Operation,
    contract: &'a Contract,
) -> Result<Vec<ResolvedObject<'a>>, Error> {
    let mut positions = HashMap::<(String, String), usize>::new();
    let mut merged = Vec::<ResolvedObject<'a>>::new();
    for (source_pointer, parameter) in operation_parameters(operation) {
        let resolved =
            contract.resolve_component_object(parameter, &source_pointer, "parameters")?;
        let name = resolved
            .value
            .get("name")
            .and_then(Value::as_str)
            .ok_or_else(|| {
                Error::Invalid(format!(
                    "parameter at {} is missing string name",
                    resolved.pointer
                ))
            })?;
        let location = resolved
            .value
            .get("in")
            .and_then(Value::as_str)
            .ok_or_else(|| {
                Error::Invalid(format!(
                    "parameter at {} is missing string in",
                    resolved.pointer
                ))
            })?;
        let key = (name.to_owned(), location.to_owned());
        if let Some(index) = positions.get(&key).copied() {
            merged[index] = resolved;
        } else {
            positions.insert(key, merged.len());
            merged.push(resolved);
        }
    }
    Ok(merged)
}

fn render_content(
    content: Option<&Value>,
    pointer: &str,
    contract: &Contract,
    context: &RenderContext<'_>,
    html: &mut String,
) -> Result<(), Error> {
    let Some(media) = content.and_then(Value::as_object) else {
        return Ok(());
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
            if contract.is_openapi_31 {
                html.push_str(
                    "<div class=\"schema-resource-reading\"><h5>Resolved schema reading</h5>",
                );
                let mut seen = BTreeSet::new();
                let mut remaining = 512;
                render_schema_resource_details(
                    contract,
                    context,
                    schema,
                    &format!("{pointer}/{}/schema", json_pointer_escape(media_type)),
                    &[],
                    &mut seen,
                    0,
                    &mut remaining,
                    html,
                )?;
                html.push_str("</div>");
            }
        }
        for example_key in ["example", "examples"] {
            if let Some(examples) = media_object.get(example_key) {
                html.push_str(&format!("<p>Contract-provided {example_key}; validity is not independently asserted:</p><pre><code>{}</code></pre>", escape_text(&pretty_json(examples))));
            }
        }
    }
    Ok(())
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
    let Some(object) = value.as_object() else {
        return;
    };
    if let Some(reference) = object.get("$ref").and_then(Value::as_str) {
        refs.push(reference.to_owned());
        return;
    }
    if let Some(properties) = object.get("properties").and_then(Value::as_object) {
        for schema in properties.values() {
            collect_schema_refs(schema, refs);
        }
    }
    for keyword in ["items", "additionalProperties", "not"] {
        if let Some(schema) = object.get(keyword).filter(|value| value.is_object()) {
            collect_schema_refs(schema, refs);
        }
    }
    for keyword in ["allOf", "oneOf", "anyOf"] {
        if let Some(schemas) = object.get(keyword).and_then(Value::as_array) {
            for schema in schemas {
                collect_schema_refs(schema, refs);
            }
        }
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

fn escape_textarea_payload(value: &str) -> String {
    value
        .chars()
        .map(|character| format!("&#{};", u32::from(character)))
        .collect()
}

pub(crate) fn generated_source_file(route: &str) -> PathBuf {
    let encoded = route
        .bytes()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    PathBuf::from("pages").join(format!("{encoded}.md"))
}

pub(crate) fn render_indexes(
    contracts: &[Contract],
    context: &RenderContext<'_>,
) -> Result<Vec<super::PendingFile>, Error> {
    let mut operations = String::from("<p>Operations in the selected contracts.</p>");
    for contract in contracts {
        if let Some(description) = contract
            .document
            .pointer("/info/description")
            .and_then(Value::as_str)
        {
            operations.push_str(&format!(
                "<details class=\"contract-about\"><summary>About {}</summary>{}</details>",
                escape_text(&contract.label),
                context.description(description)?
            ));
        }
    }
    operations.push_str("<ul>");
    for contract in contracts {
        for operation in &contract.operations {
            let route = context.href(&format!("/reference/{}/", operation.id));
            operations.push_str(&format!(
                "<li><a href=\"{}\"><code>{}</code> · {}</a></li>",
                escape_attr(&route),
                escape_text(&operation_title(operation)),
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
        page_file("/reference/operations/", "Operations", &operations, context)?,
        page_file("/reference/models/", "Models", &models, context)?,
    ])
}
