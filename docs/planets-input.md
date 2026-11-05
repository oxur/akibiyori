# Reading the Scalar Galaxy contract

The Planets specimen builds from the pinned Scalar Galaxy OpenAPI 3.1.1 source in `examples/fumadocs-reference/scalar.yaml`. The original remains unchanged; its four unresolved response references are documented beside it. `examples/fumadocs-reference/planets-corrected.yaml` is the separate four-reference derivative described in [its provenance note](../examples/fumadocs-reference/planets-provenance.md). `examples/planets.json` supplies the derivative's logical document URI and navigation configuration.

## Input and resource behavior

The reader preserves source order and JSON-shaped values. It accepts JSON and the supported JSON-compatible YAML subset, including strings, ordered mappings and sequences, nulls, booleans, and exact numeric values. It rejects unsupported YAML features such as tags, aliases, merge keys and multiple documents, as well as duplicate decoded mapping keys. Reads are bounded to 16 MiB; nesting is bounded to 64 levels, parsed nodes to 1,000,000, and scalar values to 4 MiB.

OpenAPI 3.0.x and 3.1.0/3.1.1 are recognized. The 3.1 reader displays supported schema facts, references, resource IDs, anchors, dynamic scope, compositions and boolean schemas. This is structural reading, not a JSON Schema instance validator. References resolve only to resources declared within the same input; the build does not fetch network resources. The configured `document_uri` is a logical identifier and does not authorize a request.

Contract descriptions and values are escaped for HTML output. JSON inputs retain an “Original schema JSON” source section; YAML inputs show a “JSON representation parsed from YAML” projection of the parsed schema value. The pinned YAML file remains the original source, and the projection does not preserve its comments, quoting, or formatting. These sections retain facts outside the concise view. Diagnostics identify unsupported presentation keywords; they do not mean the contract has been validated against payloads.

## Build

Run `make planets-example` for a root deployment or `make planets-example-prefix` for a `/planets-reference` deployment. Both use the configured native Cobalt and Pagefind executables and perform no network fetch during generation. The complete contract contributes 10 path operations, one webhook operation, and 15 named models. The webhook route is a documentation identifier; it does not invent an HTTP destination.

This increment focuses on ordered input and schema-resource reading. The operation pages retain their current concise layout; richer operation and example column composition is outside this increment.
