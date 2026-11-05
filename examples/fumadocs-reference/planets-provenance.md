# Planets fixture provenance

`scalar.yaml` is the preserved Scalar Galaxy OpenAPI 3.1.1 source fixture from the pinned Fumadocs Scalar example. Its source and license provenance are recorded in this directory and `theme/vendor/licenses/scalar-MIT.txt`.

`planets-corrected.yaml` is an additive reading fixture. It preserves the source file's bytes and ordering except for the four reference values listed below. `planets-corrected.diff` is the complete unified diff.

| Source location | Original value | Corrected value |
| --- | --- | --- |
| `#/paths/~1planets/get/responses/200/content/application~1json/schema/$ref` | `#/components/schemas/PaginatedResource` | `https://galaxy.scalar.com/schemas/PaginatedResource` |
| `#/paths/~1planets/get/responses/200/content/application~1json/schema/$defs/itemType/$ref` | `#/components/schemas/Planet` | `https://example.invalid/planets.openapi#/components/schemas/Planet` |
| `#/paths/~1planets/get/responses/200/content/application~1xml/schema/$ref` | `#/components/schemas/PaginatedResource` | `https://galaxy.scalar.com/schemas/PaginatedResource` |
| `#/paths/~1planets/get/responses/200/content/application~1xml/schema/$defs/itemType/$ref` | `#/components/schemas/Planet` | `https://example.invalid/planets.openapi#/components/schemas/Planet` |

The original response schemas assign `$id` values such as `https://galaxy.scalar.com/schemas/PaginatedPlanets`. Under JSON Schema resource resolution, a fragment-only pointer is relative to that resource. The original `$ref` therefore asks for a component pointer inside the `PaginatedPlanets` resource, where it has no target. The derivative names the registered generic resource and supplies an explicit logical document URI for the entry-document Planet component. The `$dynamicAnchor` then binds the generic `itemType` to Planet in both media entries.

| File | Bytes | SHA-256 |
| --- | ---: | --- |
| `scalar.yaml` | 46,552 | `00064ef711f52d867b171dd70720f804d99908d322f67ded3e7020a6dbba1702` |
| `planets-corrected.yaml` | 46,656 | `341ac03c786df114a0491f10d92f706fd6b3289e6bf208b83feafdd264ed6f39` |
