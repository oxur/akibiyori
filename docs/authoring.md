# Author Markdown

An authored page is ordinary Markdown from a path relative to the configuration file. Its route must be a safe internal path, such as `/docs/guide/`, and the referenced source must remain inside the config directory. The generator strips one complete leading frontmatter block from authored source and emits its own complete Cobalt frontmatter with `templated: false`; an unterminated leading block is an error.

## Write links for routes

Use the published route structure when linking to another page. In a page at `/docs/guide/`, `../quickstart/` reaches `/docs/quickstart/` and continues to work under a configured prefix. Do not use the source filename or a local preview port as a published destination. Generated route and asset collisions are rejected before output is written.

## Authored Markdown and contract descriptions differ

Authored Markdown is trusted site source and is copied through the authored-content path. It is not the contract-description sanitizer. Cobalt evaluates Liquid before converting Markdown, and generated authored pages disable Liquid evaluation with `templated: false`; Liquid-looking text therefore remains literal in the generated authored body. Keep untrusted text out of trusted authored source.

OpenAPI descriptions can instead use the optional `reference.descriptions: "markdown"` mode. That Markdown is parsed and sanitized with a bounded element, attribute and URL policy. The [reference reader guide](../reference-reader/) documents the allowlist and its limits; it is not a general HTML sanitizer service.

When an authored page uses the optional `document` header, its configured page `title` supplies the header h1; do not repeat that heading in the body. Without `document`, the configured title is metadata only, so put one `#` heading in the authored Markdown body and keep it aligned with the title. Images and links should have meaningful text and target real routes or explicit HTTPS URLs. See the [configuration guide](../configuration/) for a complete minimal config and the [reading shell guide](../reading-shell/) for the native controls and no-JavaScript behavior.
