# OpenAPI demos

These four examples show how the reader presents different real OpenAPI inputs. Each has its own generated pages, operation and model routes, assets and local search index. The complete showcase builds them as separate sites under this category; it does not combine their contracts or execute their APIs.

## Examples

- [Observatory tutorial catalog](../observatory/reference/) — an independently authored two-operation tutorial API. Its [observation walkthrough](../observatory/docs/reading-an-observation/) follows `ObservationEnvelope` through a selected response variant to `Calibration.calibration_epoch`.
- [Scalar Galaxy API reference](../planets/reference/) — a pinned OpenAPI 3.1.1 example with resource IDs, dynamic references, compositions and a separately documented four-reference derivative.
- [Kubernetes API Extensions](../kubernetes/reference/) — a pinned OpenAPI 3.0.0 source with long model names, recursive references, maps and 13 operations. Its [reading guide](../kubernetes/guide/) explains the static specimen; this is not a Kubernetes client.
- [Swagger Petstore API](../petstore/reference/) — an unchanged, pinned OpenAPI 3.0.4 source with 19 operations and 6 models. Its [reading guide](../petstore/guide/) follows `getPetById`, its `Pet` response and searchable model fields; this is not a Petstore client.

## Separate upstream specimen

The [upstream Fumadocs Quick Start prose and visual specimen](../planets/) preserves credited reference material from the Planets example. Its Node/npm instructions describe that upstream project; they are not Akibiyori installation instructions. The Scalar Galaxy API reference above is the primary API destination.

## Build and return

`make docs` and `make docs-prefix` build project documentation only; they do not mount these API sites. Build the complete catalogue with `make demo-site` or `make demo-site-prefix`. See [Akibiyori Docs](../../docs/) for the actual output roots and serving paths.

Return to [Public demos](../), [fixture provenance and bounds](../../docs/public-fixtures/) or [Credits](../../docs/credits/).
