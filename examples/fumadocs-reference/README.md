# Fumadocs reference specimen

This public specimen adapts the pinned Fumadocs Quick Start and “What is Fumadocs” articles into ordinary Markdown and trusted HTML for native Cobalt rendering. The source prose is from Fumadocs commit `0cd7bd4a0b7a1fa911e61bbce1242695b1c9e9ef`; the pages identify the source and link to the original documentation. Interactive React components are represented as native document structures so their reading styles can be reviewed without shipping the upstream application runtime.

The configuration is at `examples/fumadocs-reference.json`. It reads the independently authored Observatory contract from `examples/observatory/contracts/catalog.json`; that contract is retained as Observatory and is not renamed as upstream material.

`scalar.yaml` and `openapi.ts` are exact source copies from Fumadocs' pinned Scalar example. The YAML declares OpenAPI 3.1.1 and includes paths, webhooks and components. They are retained for source and license provenance; the generator does not execute `openapi.ts` or render the YAML. The file's license text is in `theme/vendor/licenses/scalar-MIT.txt`.

Use `make reference-example` or `make reference-example-prefix` from the repository root to build this specimen with the installed native Cobalt and Pagefind tools.
