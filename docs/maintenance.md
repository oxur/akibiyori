# Build and maintain

## Keep builds reproducible

The repository's `Cargo.lock` pins the Rust dependency graph. Use `--locked` for local install or build commands. Native sites use Cobalt 0.20.4 and Pagefind 1.5.2; Akibiyori does not fetch either tool or make requests to the example endpoints. The generated search index is local to each built site.

`make example`, `make planets-example`, `make kubernetes-example`, and `make petstore-example` exercise the four public demos. Each has a `-prefix` variant. `make docs` builds the project documentation alone; `make demo-site` assembles the project docs and all four demos. Add `-prefix` to the hub targets to build beneath `/akibiyori-docs`.

The assembled hub passes the opt-in `akibiyori-showcase.json` manifest to each of its five native builds so readers can move between Docs, the OpenAPI demo category, and each API's own overview. The standalone `example` and per-demo targets omit showcase context and keep their local navigation. See [site configuration](../configuration/) for the manifest and CLI options.

## Respect output ownership

The native Akibiyori CLI requires its build output to be absent or an empty real directory. Make recipes use ownership markers for their managed outputs. The demo-site script also refuses a symlink or an output that it cannot prove it owns; inspect such a path rather than deleting or adopting it. If a build fails partway, it leaves the partial output for inspection and does not write a success marker. Remove only a confirmed task-owned output, then rerun.

`make clean-example` removes only the existing marked example outputs. It does not promise to clean arbitrary directories or user data. The documentation/demo hub uses its own marked output; do not point it at a source checkout or a directory containing unrelated files.

## Read changes as source changes

Before updating an example, check its pinned source, contract route counts, license and notices. Keep the original input and any separately identified correction distinct. A rendered summary or generated sample is a bounded reading aid, not proof of full OpenAPI/JSON Schema validity. See [public fixture provenance](../public-fixtures/) and [credits](../credits/).
