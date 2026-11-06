# Public fixture provenance and bounds

The repository keeps its examples additive and independently reproducible. The Observatory demonstrates the default static-site flow; the Planets specimen demonstrates the smaller reference contract; the Fumadocs source specimen records its pinned upstream attribution. The Kubernetes API Extensions fixture adds a larger public contract to exercise the existing reader against more operations, models, references, maps, and recursive shapes. It is source material for a static reading example, not a Kubernetes client.

## Kubernetes API Extensions OpenAPI input

`examples/kubernetes/contracts/apiextensions-v1.json` is an unchanged copy of `api/openapi-spec/v3/apis__apiextensions.k8s.io__v1_openapi.json` from the Kubernetes repository at commit `00ca73359fdd9662e71af5a72ce9ff4e8ad5ed6f` (blob `67187166f68ae7860385feaf0c3744065a199d3e`). It is 182,469 bytes with SHA-256 `d12a971f7d1da940f80b21e1450d610189c3888ec3f1bb39a3e50d54920e62d2`. The complete Apache License 2.0 text from that repository's root is retained at `examples/kubernetes/upstream/LICENSE`; its SHA-256 is `cfc7749b96f63bd31c3c42b5c471bf756814053e847c10f3eb003417bc523d30`. Kubernetes project and contributor attribution, source path, and revision are also recorded in [the third-party notices](../THIRD_PARTY_NOTICES.md). No copyright year is inferred from placeholder text in the license appendix.

The inspected source tree had no applicable `NOTICE`, `COPYING`, or ancestor license override in the input path. This is the observed provenance disposition for this file, not a comprehensive legal audit. The copied path contains the OpenAPI document, not code from the wider repository’s vendor or third-party trees. The path-specific inspection found no license override; this is not a blanket conclusion about other Kubernetes materials.

## Measured reader coverage

The complete input declares OpenAPI 3.0.0 with an unversioned `info.version`, six paths, 13 operations, and 41 named models. A context-aware inventory found 442 schema nodes, 154 references, 26 actual `allOf` occurrences, and six actual `additionalProperties` occurrences. The source includes recursive references, object maps, long qualified model/property names, and multiple request/response media labels. These figures describe this pinned document; they are not a general feature-coverage score.

In particular, no applied `oneOf`, `anyOf`, `not`, or `nullable` constructs were found in the inspected schema positions. `JSONSchemaProps` describes another schema format as data: its fields named `$ref`, `$schema`, `oneOf`, `anyOf`, `not`, `allOf`, `properties`, and `default` remain ordinary source fields in this OpenAPI reader. Their names do not activate another schema dialect or authorize a Kubernetes/Draft 4 evaluator. The original JSON source section remains available alongside the concise view. Unsupported details are retained in source and bounded diagnostics.

## Media and request limits

The full contract's media labels are preserved. The reader derives JSON samples and may render a cURL reading aid only for `application/json` or another concrete media type whose subtype ends in `+json`. The eligibility check is deliberately narrow; it is not a complete MIME parser or wire-format encoder. An `application/json;stream=watch` label is not interpreted as stream framing. CBOR, protobuf, YAML, wildcards, and other unsupported media are not synthesized from a schema. A source-provided value under those media labels remains visible as a JSON representation of the source value, not as a claim that it is valid encoded wire data. External examples are never fetched. XML is not invented from a JSON schema.

This contract declares neither root servers nor a security requirement. The example's reserved `https://api.example.invalid` value is only a placeholder used by generated request text; it is not a deployed Kubernetes endpoint. No generated command is executed, no service is contacted, and no authentication requirement is inferred from an unused component. The example does not validate payloads or execute Kubernetes semantics.

## Rebuilding the example

From the package root, run `make kubernetes-example` for the root build or `make kubernetes-example-prefix` for the `/kubernetes-reference` prefix build. Both recipes use the actual Akibiyori CLI, Cobalt, and Pagefind. The generated cURL host `https://api.example.invalid` is reserved for examples and is never contacted.
