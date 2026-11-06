# Reading the Scalar Galaxy contract

The Planets specimen builds from the pinned Scalar Galaxy OpenAPI 3.1.1 source in `examples/fumadocs-reference/scalar.yaml`. The original remains unchanged; its four unresolved response references are documented beside it. `examples/fumadocs-reference/planets-corrected.yaml` is the separate four-reference derivative described in the checkout provenance note `examples/fumadocs-reference/planets-provenance.md`. `examples/planets.json` supplies the derivative's logical document URI and navigation configuration.

## Input and resource behavior

The reader preserves source order and JSON-shaped values. It accepts JSON and the supported JSON-compatible YAML subset, including strings, ordered mappings and sequences, nulls, booleans, and exact numeric values. It rejects unsupported YAML features such as tags, aliases, merge keys and multiple documents, as well as duplicate decoded mapping keys. Reads are bounded to 16 MiB; nesting is bounded to 64 levels, parsed nodes to 1,000,000, and scalar values to 4 MiB. The original JSON and YAML inputs are unchanged; `planets-corrected.yaml` remains the separately identified four-reference derivative.

OpenAPI 3.0.x and 3.1.0/3.1.1 are recognized. The 3.1 reader displays supported schema facts, references, resource IDs, anchors, dynamic scope, compositions and boolean schemas. This is structural reading, not a JSON Schema instance validator. References resolve only to resources declared within the same input; the build does not fetch network resources. The configured `document_uri` is a logical identifier and does not authorize a request.

Contract names, summaries, examples and data values are escaped source values. The Planets configuration opts descriptions into sanitized Markdown; other configurations default to escaped plain text. JSON inputs retain an “Original schema JSON” source section; YAML inputs show a “JSON representation parsed from YAML” projection of the parsed schema value. The pinned YAML file remains the original source, and the projection does not preserve its comments, quoting, or formatting. These sections retain facts outside the concise view. Diagnostics identify unsupported presentation keywords; they do not mean the contract has been validated against payloads.

JSON errors identify the input file. When serde decodes a value captured separately from the document, its line and column may be relative to that value rather than to the full file; the original serde coordinates are preserved.

## Build

Run `make planets-example` for a root deployment or `make planets-example-prefix` for a `/planets-reference` deployment. Both use the configured native Cobalt and Pagefind executables and perform no network fetch during generation. The complete contract contributes 10 path operations, one webhook operation, and 15 named models. The webhook route is a documentation identifier; it does not invent an HTTP destination.

The Planets configuration also opts operation pages into the reference columns layout, sanitized description Markdown, and HTTPS-only Markdown images. Read-only request and response examples are labeled as provided or schema-derived; they are never validated or sent. A schema-derived sample is a bounded reading aid, not a payload generator. JSON examples can be derived from source examples/defaults and writable/required schema fields; an XML sample is shown only when the input provides one. The operation reader and its limits are documented in [the reference-reader guide](../reference-reader/).
