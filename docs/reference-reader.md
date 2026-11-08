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

The optional top-level `reference_sidebar` setting replaces the manual navigation tree only on generated `/reference/` pages. It starts with a direct overview link to the authored `/reference/` page, then shows generated operation groups. Ordinary guides and the default manual sidebar stay unchanged. `overview_label` names the direct overview link; `untagged_label` optionally names operations without tags (default: `Other operations`). Within each contract, groups follow the first use of each tag. Path operations follow source path order and the sidebar method order `GET`, `POST`, `PATCH`, `DELETE`, `HEAD`, `PUT`, `OPTIONS`, `TRACE`; webhooks follow the path operations. This only orders the opt-in sidebar: generated operation pages and the contract's operation records keep their source order. In the default flat presentation, multiple contracts remain separate, in config order, and receive `{contract label} — {tag}` headings; the optional contract-accordion presentation is described in the [configuration guide](../configuration/). Tag `x-displayName` or `summary` supplies the visible heading when present; otherwise the source tag name is used. The Planets example sets `overview_label` to `Scalar Example` and its section label to `OpenAPI Example`.

Sidebar webhook labels and columns-page titles use a nonempty supplied summary when available; otherwise the source webhook key is formatted as a title (for example, `newPlanet` becomes `New Planet`) and keeps its canonical generated route. That small presentation convention follows the pinned Fumadocs [`id-to-title.ts` helper](https://github.com/fuma-nama/fumadocs/blob/0cd7bd4a0b7a1fa911e61bbce1242695b1c9e9ef/packages/shared-api/src/utils/id-to-title.ts); source keys and operation records are unchanged.

In columns output, each rendered schema property has a linkable, context-qualified anchor. Its identity includes the contract, the operation/status/media occurrence, the schema source pointer and the nested traversal path, so repeated component uses do not collapse onto one fragment. The reading script opens the containing response/media choices and nested disclosures, clears only filters that hide the requested field, then focuses and scrolls to the row. Existing model field IDs remain stable for older links.

Reference links use canonical schema pointers. A model field fragment is emitted only when the model page actually contains that field heading. When a valid target has no emitted heading, the link opens the model page and retains the full canonical pointer alongside it. Standalone model-page annotations use the same conservative model-root link because their output is rendered before the complete set of model headings is known.

## Description Markdown policy

The renderer uses pulldown-cmark with tables, strikethrough, task lists and GFM alerts enabled, then applies a fixed Ammonia allowlist. The allowed elements cover ordinary headings (h1 becomes h2), paragraphs, emphasis, lists, tables, details/summary, links, code and selected alert blocks. Active elements such as scripts, styles, forms, frames and embedded objects are removed with their contents. Event handlers, IDs, inline styles, arbitrary data attributes and unknown CSS classes are not retained. Task-list inputs are forced to disabled checkboxes; `checked` is preserved as a boolean attribute by presence.

Links may target a fragment, a validated root-relative path, or an HTTPS URL with a host and no user information. A base path is applied once to root-relative links. Protocol-relative paths, dot-segment traversal, malformed escapes, whitespace/control characters, backslashes, non-HTTPS schemes and credential-bearing URLs are removed. Links receive `rel="noopener noreferrer"`.

With `images: "https"`, images require HTTPS and receive `loading="lazy"` and `referrerpolicy="no-referrer"`. `alt` is retained; width and height are retained only in the bounded range 1–4096. Images are not fetched during generation. With `images: "link"`, Markdown image syntax becomes a link whose text is the image alt text; raw HTML image elements are dropped.

JSON and Bash fences are highlighted locally with Syntect. Other fences remain escaped plaintext. The emitted syntax classes are limited to the reader's allowlist. Literal Liquid-looking text remains text and is not evaluated. This is a bounded Markdown renderer, not a general HTML or CSS sanitizer service.

## Operation pages and examples

Columns pages derive labels, routes, methods, security alternatives, media types, statuses, schema fields and examples from the input contract. API operation order and source object/property order are retained. An operation's security array is shown as OR alternatives; multiple schemes inside one alternative are shown together, and an explicit empty array means no authentication requirement.

Callbacks declared on a columns operation appear as source-ordered, read-only readers inside that operation page. Their callback names, runtime expressions, method, security override, request body and responses are displayed without treating the expression as a callable URL. Request delivery examples retain their media choices and source notes; the active example card shows the selected schema description and a single copy control. Without JavaScript, the media-specific labels and descriptions remain with their visible example panels, while selectors and copy controls stay disabled. With JavaScript, the same rendered reader content opens in a native modal dialog; Escape and Close return focus to its trigger. Without JavaScript, each callback remains an ordinary closed `details` disclosure. Callback readers do not add operation pages or navigation entries, do not execute a delivery, and do not inherit the parent operation's security when the callback declares `security: []`. This is a bounded one-level presentation; deeper callback content remains a source diagnostic.

An opened callback reader keeps its caller path as context for the delivery examples; that path is not a destination. Empty effective callback authorization is retained in the closed source disclosure without an empty visible card. Callback and webhook deliveries omit the response-example card only when their resolved responses declare no media; component-referenced response media and declared-but-unavailable examples remain visible. Bodyless response statuses remain in the operation flow. Callback source pointers wrap as prose, while raw JSON retains its own horizontal scrolling.

Columns response panels also retain referenced Response Objects and their declared Header Objects in a closed `Response contract source` disclosure. The operation's existing source section keeps the original response reference; the response disclosure preserves the resolved Response Object, original header references, resolved local header chains, canonical JSON Pointers, and complete resolved Header Objects. Header values are preserved as source facts only; this reader does not infer effective HTTP header behavior or compose header schemas.

The example rail distinguishes input-provided values from bounded schema-derived reading aids. It respects request/response direction, required and read-only/write-only fields, and uses JSON number/value preservation from the input loader. A schema-derived JSON request sample includes required writable properties only; it is not checked for semantic validity. XML is not synthesized from a JSON schema: if the input has no XML example, the reader says one is unavailable. External example URLs are shown without fetching them.

For a declared request body, the existing cURL view is offered only when an available request example uses a concrete JSON media type: `application/json` or a concrete subtype ending in `+json`. This is a narrow reader policy, not a complete MIME parser or a wire encoder. Separately, an ordinary Path operation with no `requestBody` field may show a metadata-derived request template. That template uses the selected first server and operation path, passes the quoted URL with `--url`, disables curl URL globbing, and uses `--head` for HEAD. It has no payload or Content-Type header. Its optional Accept header comes from the first source-ordered successful (`2xx` or literal `2XX`) response with nonempty content, when that first media label is printable ASCII; response selector changes do not change the template. Server variables and required path, query, header, or cookie values remain unfilled. Only the first single HTTP bearer alternative adds a placeholder; other required authentication is described without guessing a header. A declared but unavailable request body does not qualify, and callback/webhook delivery destinations do not receive a callable template. These examples never contain a real credential, are not executed, and are not live request controls.

Contract-provided media examples remain visible as supplied. If a provided non-JSON value is shown, the code panel is labelled `Source value (JSON)` and represents the source value as JSON; it does not claim to encode CBOR, protobuf, YAML, XML, or another wire format. Without a provided example, the reader does not derive a JSON sample for unsupported media and instead reports the limitation beside its source pointer. A label such as `application/json;stream=watch` remains a source label; stream framing is not interpreted. XML retains its separate no-synthesis message.

Media/example selectors and property filters are native disabled controls until the small self-hosted reader script initializes them. Without JavaScript, all source panels and examples remain visible; filter inputs and selectors remain disabled rather than implying that they work.

## Runtime and dependencies

The Rust generator uses pinned pulldown-cmark 0.13.6 for Markdown events, Ammonia 4.2.4 for the fixed HTML allowlist, and Syntect 5.2.0 with its `default-fancy` syntax support. The generated pages use the already bundled `theme/js/akibiyori-reading.js` for TOC, clipboard, media/example selection, property filtering, field-fragment reveal and callback dialog enhancement. It adds no Node runtime, remote JavaScript, API request, proxy, storage or authentication service.

The complete Planets OpenAPI input and operation/schema resource limitations are described in [Planets input](../planets-input/). Native Cobalt and Pagefind builds remain the output verification substrate; browser visual and interaction review is a separate acceptance step.
