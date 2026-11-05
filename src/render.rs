use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

use serde_json::Value;

use crate::config::{DocumentHeader, NavigationLink, SidebarGroup, validate_route};
use crate::openapi::{Contract, Operation, ResolvedObject};
use crate::{Diagnostic, Error};

pub(crate) struct RenderContext<'a> {
    pub base_path: &'a str,
    pub has_home: bool,
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
        inspect_unrendered_operation(contract, operation, diagnostics)?;
        let route = format!("/reference/{}/", operation.id);
        output.push(page_file(
            &route,
            &format!("{} {}", operation.method, operation.path),
            &render_operation(contract, operation, context)?,
            context,
        )?);
    }
    for (name, schema) in &contract.schemas {
        let route = format!("/reference/models/{}/{name}/", contract.id);
        output.push(page_file(
            &route,
            name,
            &render_model(contract, name, schema, context, diagnostics),
            context,
        )?);
    }
    Ok(())
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
    for key in ["allOf", "oneOf", "anyOf"] {
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
        })
        .collect()
}

pub(crate) fn sidebar_data(
    groups: &[SidebarGroup],
    fallback: &[NavigationLink],
    base_path: &str,
) -> Vec<SidebarGroup> {
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
        .map(|group| SidebarGroup {
            title: group.title,
            links: navigation_data(&group.links, base_path),
        })
        .collect()
}

pub(crate) fn render_navigation() -> String {
    String::from(
        r#"<aside id="nd-sidebar" class="reference-sidebar [grid-area:sidebar] flex min-h-0 overflow-clip text-sm md:layout:[--fd-sidebar-width:268px] max-md:hidden" aria-label="Documentation navigation"><div class="reference-sidebar-inner flex flex-col shrink-0 w-(--fd-sidebar-width)"><div class="sidebar-header"><a class="sidebar-home" href="{{ site.data.akibiyori_site.home_href | escape }}">{{ site.title | escape }}</a></div><div class="sidebar-controls"><a class="sidebar-search" href="{{ site.data.akibiyori_site.search_href | escape }}"><svg aria-hidden="true" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="m21 21-4.34-4.34"/><circle cx="11" cy="11" r="8"/></svg>Search documentation</a></div><div class="sidebar-viewport"><nav aria-label="Documentation">{% for group in site.data.akibiyori_sidebar %}<section class="sidebar-group"><h2>{{ group.title | escape }}</h2><ul>{% for link in group.links %}<li><a href="{{ link.href | escape }}">{{ link.label | escape }}</a></li>{% endfor %}</ul></section>{% endfor %}</nav></div></div></aside>"#,
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
        "---\ntitle: {}\npermalink: {}\nlayout: {}\ntemplated: false\n---\n<div class=\"reference-context\">{}</div>\n\n{}",
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
        "---\ntitle: {}\npermalink: {}\nlayout: default.liquid\ntemplated: false\ndata:\n  document_header: true\n---\n{}\n<div class=\"prose flex-1 text-fd-foreground/90\">\n\n{}\n\n</div>",
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
) -> String {
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
    .render_node(schema, "", &root_pointer);
    html.push_str(&format!(
        "<section><h2 id=\"source-{}\">Original schema JSON</h2><pre><code>",
        stable_anchor(&root_pointer)
    ));
    html.push_str(&escape_text(&pretty_json(schema)));
    html.push_str("</code></pre></section>");
    html
}

struct SchemaRenderer<'a, 'b> {
    contract: &'a Contract,
    context: &'a RenderContext<'b>,
    html: &'a mut String,
    seen: &'a mut BTreeSet<String>,
    diagnostics: &'a mut Vec<Diagnostic>,
}

impl SchemaRenderer<'_, '_> {
    fn render_node(&mut self, schema: &Value, prefix: &str, pointer: &str) {
        if !self.seen.insert(pointer.to_owned()) {
            return;
        }
        inspect_unknown_schema_keys(self.contract, schema, pointer, self.diagnostics);
        render_schema_facts(schema, self.html);
        if let Some(reference) = schema.get("$ref").and_then(Value::as_str) {
            render_ref_link(self.contract, reference, self.context, self.html, pointer);
            return;
        }
        let Some(object) = schema.as_object() else {
            if let Some(allowed) = schema.as_bool() {
                self.html.push_str(&format!(
                    "<p>Additional properties allowed: <code>{allowed}</code></p>"
                ));
            }
            return;
        };

        if let Some(properties) = object.get("properties").and_then(Value::as_object) {
            let required: BTreeSet<&str> = object
                .get("required")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
                .filter_map(Value::as_str)
                .collect();
            self.html
                .push_str("<section class=\"fields\"><h2>Fields</h2>");
            for (field, property) in properties {
                let full_path = if prefix.is_empty() {
                    field.clone()
                } else {
                    format!("{prefix}.{field}")
                };
                let field_pointer = format!("{pointer}/properties/{}", json_pointer_escape(field));
                let anchor = stable_anchor(&field_pointer);
                self.html.push_str(&format!(
                    "<section class=\"field\"><h3 id=\"{anchor}\">{}{}</h3>",
                    escape_text(&full_path),
                    if required.contains(field.as_str()) {
                        " <span class=\"required\">required</span>"
                    } else {
                        ""
                    }
                ));
                self.render_node(property, &full_path, &field_pointer);
                self.html.push_str("</section>");
            }
            self.html.push_str("</section>");
        }

        if let Some(items) = object.get("items") {
            self.html.push_str("<section><h2>Array items</h2>");
            self.render_node(items, prefix, &format!("{pointer}/items"));
            self.html.push_str("</section>");
        }
        if let Some(additional) = object.get("additionalProperties") {
            self.html
                .push_str("<section><h2>Additional properties</h2>");
            self.render_node(
                additional,
                prefix,
                &format!("{pointer}/additionalProperties"),
            );
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
                    self.render_node(alternative, prefix, &alt_pointer);
                    self.html.push_str("</li>");
                }
                self.html.push_str("</ul></section>");
            }
        }
    }
}

fn render_schema_facts(schema: &Value, html: &mut String) {
    for keyword in ["type", "format", "title", "description"] {
        if let Some(value) = schema.get(keyword) {
            let value = value
                .as_str()
                .map(str::to_owned)
                .unwrap_or_else(|| value.to_string());
            html.push_str(&format!(
                "<p><strong>{}</strong>: {}</p>",
                escape_text(keyword),
                escape_text(&value)
            ));
        }
    }
    render_constraints(schema, html);
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
    for key in object
        .keys()
        .filter(|key| !RENDERED.contains(&key.as_str()))
    {
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
                html.push_str(&format!("<p>{}</p>", escape_text(description)));
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
            html.push_str(&format!("<p>{}</p>", escape_text(description)));
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
        render_content(resolved.value.get("content"), contract, context, &mut html);
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
                html.push_str(&format!("<p>{}</p>", escape_text(description)));
            }
            render_content(resolved.value.get("content"), contract, context, &mut html);
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
    let mut merged: BTreeMap<(String, String), ResolvedObject<'a>> = BTreeMap::new();
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
        merged.insert((name.to_owned(), location.to_owned()), resolved);
    }
    Ok(merged.into_values().collect())
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
        page_file("/reference/operations/", "Operations", &operations, context)?,
        page_file("/reference/models/", "Models", &models, context)?,
    ])
}
