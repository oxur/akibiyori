use std::borrow::Cow;
use std::collections::{HashMap, HashSet};

use ammonia::Builder;
use pulldown_cmark::{CodeBlockKind, Event, HeadingLevel, Options, Parser, Tag, TagEnd, html};
use syntect::html::{ClassStyle, ClassedHTMLGenerator};
use syntect::parsing::SyntaxSet;

use crate::Error;
use crate::config::ImageMode;

pub(crate) struct DescriptionRenderer {
    syntaxes: SyntaxSet,
}

impl DescriptionRenderer {
    pub(crate) fn new() -> Self {
        Self {
            syntaxes: SyntaxSet::load_defaults_newlines(),
        }
    }

    pub(crate) fn render(
        &self,
        source: &str,
        base_path: &str,
        images: ImageMode,
    ) -> Result<String, Error> {
        let mut options = Options::empty();
        options.insert(Options::ENABLE_TABLES);
        options.insert(Options::ENABLE_STRIKETHROUGH);
        options.insert(Options::ENABLE_TASKLISTS);
        options.insert(Options::ENABLE_GFM);

        let events = Parser::new_ext(source, options);
        let mut output = String::new();
        let mut ordinary_events = Vec::new();
        let mut events = events.peekable();
        while let Some(event) = events.next() {
            match event {
                Event::Start(Tag::CodeBlock(kind)) => {
                    flush_events(&mut output, &mut ordinary_events);
                    let mut code = String::new();
                    for next in events.by_ref() {
                        match next {
                            Event::End(TagEnd::CodeBlock) => break,
                            Event::Text(text) => code.push_str(&text),
                            _ => {}
                        }
                    }
                    let language = match kind {
                        CodeBlockKind::Fenced(info) => normalize_language(&info),
                        CodeBlockKind::Indented => "text",
                    };
                    output.push_str(&self.render_code(&code, language)?);
                }
                Event::Start(Tag::Heading {
                    level: HeadingLevel::H1,
                    id,
                    classes,
                    attrs,
                }) => ordinary_events.push(Event::Start(Tag::Heading {
                    level: HeadingLevel::H2,
                    id,
                    classes,
                    attrs,
                })),
                Event::Start(Tag::Image {
                    link_type,
                    dest_url,
                    title,
                    id,
                }) if images == ImageMode::Link => {
                    ordinary_events.push(Event::Start(Tag::Link {
                        link_type,
                        dest_url,
                        title,
                        id,
                    }));
                }
                Event::End(TagEnd::Image) if images == ImageMode::Link => {
                    ordinary_events.push(Event::End(TagEnd::Link));
                }
                other => ordinary_events.push(other),
            }
        }
        flush_events(&mut output, &mut ordinary_events);
        sanitize_html(&output, base_path, images)
    }

    fn render_code(&self, code: &str, language: &str) -> Result<String, Error> {
        let syntax = self.syntaxes.find_syntax_by_token(language);
        let Some(syntax) = syntax else {
            return Ok(format!(
                "<pre class=\"highlighter-syntect\"><code class=\"language-text\">{}</code></pre>",
                escape_html(code)
            ));
        };
        let mut generator = ClassedHTMLGenerator::new_with_class_style(
            syntax,
            &self.syntaxes,
            ClassStyle::SpacedPrefixed { prefix: "c-" },
        );
        for line in code.split_inclusive('\n') {
            generator
                .parse_html_for_line_which_includes_newline(line)
                .map_err(|error| {
                    Error::Invalid(format!("failed to emit highlighted Markdown code: {error}"))
                })?;
        }
        let highlighted = generator.finalize();
        Ok(format!(
            "<pre class=\"highlighter-syntect\"><code class=\"language-{language}\">{highlighted}</code></pre>"
        ))
    }
}

fn flush_events(output: &mut String, events: &mut Vec<Event<'_>>) {
    if !events.is_empty() {
        html::push_html(output, std::mem::take(events).into_iter());
    }
}

fn normalize_language(info: &str) -> &'static str {
    match info
        .split_whitespace()
        .next()
        .unwrap_or_default()
        .to_ascii_lowercase()
        .as_str()
    {
        "json" => "json",
        "bash" | "sh" | "shell" | "curl" => "bash",
        _ => "text",
    }
}

fn sanitize_html(source: &str, base_path: &str, images: ImageMode) -> Result<String, Error> {
    let tags: HashSet<&str> = [
        "p",
        "br",
        "hr",
        "h2",
        "h3",
        "h4",
        "h5",
        "h6",
        "em",
        "strong",
        "b",
        "i",
        "s",
        "del",
        "blockquote",
        "ul",
        "ol",
        "li",
        "dl",
        "dt",
        "dd",
        "table",
        "thead",
        "tbody",
        "tr",
        "th",
        "td",
        "caption",
        "details",
        "summary",
        "a",
        "pre",
        "code",
        "span",
        "div",
        "img",
        "input",
    ]
    .into_iter()
    .collect();
    let clean_content_tags: HashSet<&str> = [
        "script", "style", "iframe", "object", "embed", "svg", "math", "form", "template",
    ]
    .into_iter()
    .collect();
    let mut attributes = HashMap::new();
    attributes.insert("a", HashSet::from(["href", "title"]));
    attributes.insert(
        "img",
        HashSet::from(["src", "alt", "width", "height", "loading", "referrerpolicy"]),
    );
    attributes.insert("input", HashSet::from(["type", "disabled", "checked"]));
    attributes.insert(
        "td",
        HashSet::from(["colspan", "rowspan", "align", "title"]),
    );
    attributes.insert(
        "th",
        HashSet::from(["colspan", "rowspan", "align", "title"]),
    );
    attributes.insert("details", HashSet::from(["open"]));

    let syntax_classes: HashSet<&str> = [
        "c-source",
        "c-string",
        "c-punctuation",
        "c-keyword",
        "c-storage",
        "c-variable",
        "c-function",
        "c-type",
        "c-comment",
        "c-constant",
        "c-number",
        "c-numeric",
        "c-operator",
        "c-support",
        "c-property-name",
        "c-meta",
        "c-key",
        "c-quoted",
        "c-single",
        "c-double",
        "c-dictionary",
        "c-mapping",
    ]
    .into_iter()
    .collect();
    let alert_classes: HashSet<&str> = [
        "markdown-alert",
        "markdown-alert-note",
        "markdown-alert-tip",
        "markdown-alert-important",
        "markdown-alert-warning",
        "markdown-alert-caution",
    ]
    .into_iter()
    .collect();
    let classes = HashMap::from([
        ("span", syntax_classes),
        ("pre", HashSet::from(["highlighter-syntect"])),
        (
            "code",
            HashSet::from(["language-json", "language-bash", "language-text"]),
        ),
        ("blockquote", alert_classes.clone()),
        ("div", alert_classes),
    ]);
    let allowed_schemes = HashSet::from(["https"]);
    let generic_attributes = HashSet::from(["title"]);
    let filter_attributes = attributes.clone();
    let filter_generic = generic_attributes.clone();
    let base = base_path.to_owned();
    let image_mode = images;
    let forced_values = HashMap::from([
        (
            "input",
            HashMap::from([("type", "checkbox"), ("disabled", "")]),
        ),
        (
            "img",
            HashMap::from([("loading", "lazy"), ("referrerpolicy", "no-referrer")]),
        ),
    ]);

    Ok(Builder::default()
        .tags(tags)
        .clean_content_tags(clean_content_tags)
        .tag_attributes(attributes)
        .set_tag_attribute_values(forced_values)
        .generic_attributes(generic_attributes)
        .allowed_classes(classes)
        .url_schemes(allowed_schemes)
        .link_rel(Some("noopener noreferrer"))
        .attribute_filter(move |element, attribute, value| {
            if element == "input" && attribute == "checked" {
                return Some(Cow::Borrowed(""));
            }
            if (element == "a" && attribute == "href") || (element == "img" && attribute == "src") {
                let image = element == "img";
                if image && image_mode != ImageMode::Https {
                    return None;
                }
                return safe_url(value, &base, image).map(Cow::Owned);
            }
            if matches!((element, attribute), ("td" | "th", "colspan" | "rowspan")) {
                return value
                    .parse::<u16>()
                    .ok()
                    .filter(|number| (1..=100).contains(number))
                    .map(|_| Cow::Borrowed(value));
            }
            if matches!((element, attribute), ("td" | "th", "align")) {
                return matches!(
                    value.to_ascii_lowercase().as_str(),
                    "left" | "center" | "right" | "justify"
                )
                .then_some(Cow::Borrowed(value));
            }
            if element == "img" && matches!(attribute, "width" | "height") {
                return value
                    .parse::<u16>()
                    .ok()
                    .filter(|number| (1..=4096).contains(number))
                    .map(|_| Cow::Borrowed(value));
            }
            if attribute == "class"
                || filter_generic.contains(attribute)
                || filter_attributes
                    .get(element)
                    .is_some_and(|allowed| allowed.contains(attribute))
            {
                return Some(Cow::Borrowed(value));
            }
            None
        })
        .clean(source)
        .to_string())
}

fn safe_url(value: &str, base_path: &str, image: bool) -> Option<String> {
    if value.is_empty()
        || value
            .chars()
            .any(|ch| ch.is_whitespace() || ch.is_control() || ch == '\\')
        || percent_encoding::percent_decode_str(value)
            .decode_utf8()
            .is_err()
    {
        return None;
    }
    if value.starts_with('#') && !image {
        return Some(value.to_owned());
    }
    if value.starts_with('/') {
        if image || value.starts_with("//") {
            return None;
        }
        let path = value.split(['?', '#']).next().unwrap_or_default();
        let decoded = percent_encoding::percent_decode_str(path)
            .decode_utf8()
            .ok()?;
        if decoded.contains('\\')
            || decoded
                .chars()
                .any(|ch| ch.is_whitespace() || ch.is_control())
            || decoded
                .split('/')
                .any(|segment| segment == "." || segment == "..")
        {
            return None;
        }
        if base_path.is_empty() || path == base_path || path.starts_with(&format!("{base_path}/")) {
            return Some(value.to_owned());
        }
        return Some(format!("{base_path}{value}"));
    }
    let url = url::Url::parse(value).ok()?;
    if url.scheme() != "https" || !url.username().is_empty() || url.password().is_some() {
        return None;
    }
    url.host_str()?;
    Some(value.to_owned())
}

fn escape_html(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('\"', "&quot;")
        .replace('\'', "&#39;")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitizer_keeps_declared_markdown_roles_without_allowing_active_markup() {
        let renderer = DescriptionRenderer::new();
        let source = r#"
# Intro

| A | B |
|---|---|
| one | two |

<details open><summary>More</summary><p title="kept" onclick="run()">safe</p></details>

> [!TIP]
> A useful note.

- [x] retained task state

```json
{"ok": true, "count": 2}
```

<img src="https://example.test/image.png" alt="planet" onerror="run()">
<script>run_script()</script>
<div class="unknown theme" id="x" style="color:red" data-x="1">retained text</div>
<input type="text" checked="false" name="secret" value="x">
"#;
        let rendered = renderer
            .render(source, "/docs", ImageMode::Https)
            .expect("sanitize");

        assert!(rendered.contains("<h2>Intro</h2>"));
        assert!(rendered.contains("<table>"));
        assert!(rendered.contains("<details open=\"\">"));
        assert!(rendered.contains("title=\"kept\""));
        assert!(!rendered.contains("onclick"));
        assert!(rendered.contains("markdown-alert-tip"));
        assert!(rendered.contains("type=\"checkbox\""));
        assert!(rendered.contains("disabled=\"\""));
        assert!(rendered.contains("checked=\"\""));
        assert!(!rendered.contains("name=\"secret\""));
        assert!(!rendered.contains("value=\"x\""));
        assert!(rendered.contains("alt=\"planet\""));
        assert!(rendered.contains("loading=\"lazy\""));
        assert!(rendered.contains("referrerpolicy=\"no-referrer\""));
        assert!(!rendered.contains("onerror"));
        assert!(!rendered.contains("run_script"));
        assert!(rendered.contains("retained text"));
        assert!(!rendered.contains("unknown"));
        assert!(!rendered.contains("id=\"x\""));
        assert!(!rendered.contains("style="));
        assert!(!rendered.contains("data-x"));
        assert!(rendered.contains("language-json"));
        assert!(rendered.contains("class=\"c-"));

        // Isolate the raw input so the Markdown task-list checkbox above cannot
        // make this presence-based HTML boolean assertion pass accidentally.
        let raw_checked = renderer
            .render(
                "<input type=\"text\" checked=\"false\">",
                "",
                ImageMode::Link,
            )
            .expect("sanitize raw checkbox");
        assert!(raw_checked.contains("checked=\"\""));
        assert!(raw_checked.contains("disabled=\"\""));
        assert!(raw_checked.contains("type=\"checkbox\""));
        assert!(!raw_checked.contains("type=\"text\""));
    }

    #[test]
    fn image_link_mode_and_url_policy_are_strict_and_prefix_once() {
        let renderer = DescriptionRenderer::new();
        let source = r#"[local](/guide/one/?q=1#part) [external](https://docs.example/path) [bad](//evil.test/x) [traversal](/a/%2e%2e/secret) [userinfo](https://name:secret@docs.example/x) ![alt text](https://images.example/planet.png) ![unsafe](javascript:alert(1))"#;
        let rendered = renderer
            .render(source, "/docs", ImageMode::Link)
            .expect("sanitize");
        assert!(rendered.contains("href=\"/docs/guide/one/?q=1#part\""));
        assert!(rendered.contains("href=\"https://docs.example/path\""));
        assert!(rendered.contains(">alt text</a>"));
        assert!(rendered.contains("unsafe"));
        assert!(!rendered.contains("//evil.test"));
        assert!(!rendered.contains("/a/%2e%2e/secret"));
        assert!(!rendered.contains("name:secret"));
        assert!(!rendered.contains("<img"));
        assert!(!rendered.contains("javascript:"));
    }

    #[test]
    fn code_fence_fallback_and_literal_liquid_are_kept_as_text() {
        let renderer = DescriptionRenderer::new();
        let rendered = renderer
            .render(
                "```made-up\n{{ 4 | plus: 1 }} <script>\n```",
                "",
                ImageMode::Link,
            )
            .expect("sanitize fallback");
        assert!(rendered.contains("language-text"));
        assert!(rendered.contains("{{ 4 | plus: 1 }} &lt;script&gt;"));
        assert!(!rendered.contains("<script>"));
    }
}
