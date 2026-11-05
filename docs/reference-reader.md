# Reference reader

The operation and schema reader turns a supplied OpenAPI contract into static Cobalt pages. It does not call the described API, validate example payloads, authenticate to a service, or execute generated commands. Unsupported source details remain in the closed source sections and are reported as presentation diagnostics where applicable.

## Optional configuration

The `reference` object is optional. Each setting defaults independently; unknown fields or enum values fail configuration parsing.

```json
{
  "reference": {
    "layout": "columns",
    "descriptions": "markdown",
    "images": "https"
  }
}
```

| Setting | Values | Default | Effect |
| --- | --- | --- | --- |
| `layout` | `source`, `columns` | `source` | `source` keeps the existing concise operation page. `columns` adds a structured operation page with source facts, media/schema controls and an example rail. |
| `descriptions` | `plain`, `markdown` | `plain` | `plain` escapes descriptions as text. `markdown` parses and sanitizes description Markdown. |
| `images` | `link`, `https` | `link` | `link` converts Markdown images to links and blocks raw HTML images. `https` allows validated HTTPS images. It has an effect only when descriptions use Markdown. |

The settings do not change caller-owned authored Markdown. That content follows the existing trusted-source path and is outside the contract-description sanitizer.

## Description Markdown policy

The renderer uses pulldown-cmark with tables, strikethrough, task lists and GFM alerts enabled, then applies a fixed Ammonia allowlist. The allowed elements cover ordinary headings (h1 becomes h2), paragraphs, emphasis, lists, tables, details/summary, links, code and selected alert blocks. Active elements such as scripts, styles, forms, frames and embedded objects are removed with their contents. Event handlers, IDs, inline styles, arbitrary data attributes and unknown CSS classes are not retained. Task-list inputs are forced to disabled checkboxes; `checked` is preserved as a boolean attribute by presence.

Links may target a fragment, a validated root-relative path, or an HTTPS URL with a host and no user information. A base path is applied once to root-relative links. Protocol-relative paths, dot-segment traversal, malformed escapes, whitespace/control characters, backslashes, non-HTTPS schemes and credential-bearing URLs are removed. Links receive `rel="noopener noreferrer"`.

With `images: "https"`, images require HTTPS and receive `loading="lazy"` and `referrerpolicy="no-referrer"`. `alt` is retained; width and height are retained only in the bounded range 1–4096. Images are not fetched during generation. With `images: "link"`, Markdown image syntax becomes a link whose text is the image alt text; raw HTML image elements are dropped.

JSON and Bash fences are highlighted locally with Syntect. Other fences remain escaped plaintext. The emitted syntax classes are limited to the reader's allowlist. Literal Liquid-looking text remains text and is not evaluated. This is a bounded Markdown renderer, not a general HTML or CSS sanitizer service.

## Operation pages and examples

Columns pages derive labels, routes, methods, security alternatives, media types, statuses, schema fields and examples from the input contract. API operation order and source object/property order are retained. An operation's security array is shown as OR alternatives; multiple schemes inside one alternative are shown together, and an explicit empty array means no authentication requirement.

The example rail distinguishes input-provided values from bounded schema-derived reading aids. It respects request/response direction, required and read-only/write-only fields, and uses JSON number/value preservation from the input loader. A schema-derived JSON request sample includes required writable properties only; it is not checked for semantic validity. XML is not synthesized from a JSON schema: if the input has no XML example, the reader says one is unavailable. External example URLs are shown without fetching them.

Generated cURL text uses the first declared server, operation method and path, with safely quoted values and a bearer placeholder only when the first security alternative includes an HTTP bearer scheme. It never contains a real credential, is not executed, and is not a live request control. Media/example selectors and property filters are native disabled controls until the small self-hosted reader script initializes them. Without JavaScript, all source panels and examples remain visible; filter inputs and selectors remain disabled rather than implying that they work.

## Runtime and dependencies

The Rust generator uses pinned pulldown-cmark 0.13.6 for Markdown events, Ammonia 4.2.4 for the fixed HTML allowlist, and Syntect 5.2.0 with its `default-fancy` syntax support. The generated pages use the already bundled `theme/js/akibiyori-reading.js` for TOC, clipboard, media/example selection and property filtering. It adds no Node runtime, remote JavaScript, API request, proxy, storage or authentication service.

The complete Planets OpenAPI input and operation/schema resource limitations are described in [Planets input](planets-input.md). Native Cobalt and Pagefind builds remain the output verification substrate; browser visual and interaction review is a separate acceptance step.
