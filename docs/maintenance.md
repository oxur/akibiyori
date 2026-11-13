# Build and maintain

## Keep builds reproducible

The repository's `Cargo.lock` pins the Rust dependency graph. Use `--locked` for local install or build commands. Install the [native build prerequisites](../install/), including Cobalt 0.20.4 and Pagefind 1.5.2. Akibiyori does not fetch either tool or make requests to the example endpoints; each built site has its own generated search index. Make defaults `COBALT` to `cobalt` and `PAGEFIND` to `pagefind`, resolved from `PATH`. When a nondefault executable is needed, pass its path for that command, for example `COBALT=/opt/tools/cobalt PAGEFIND=/opt/tools/pagefind make docs`; this is an optional override, not part of the normal build.

`make example`, `make planets-example`, `make kubernetes-example`, and `make petstore-example` exercise the four public demos. Each has a `-prefix` variant. `make docs` builds the project documentation alone; `make demo-site` assembles the project docs and all four demos. Add `-prefix` to the hub targets to build beneath `/akibiyori-docs`.

The assembled hub passes one `SHOWCASE_MANIFEST` (defaulting to `akibiyori-showcase.json`) and its exact five site IDs to each native build, so readers can move between Docs, the OpenAPI demo category, and each API's own overview. Before replacing an output whose ownership marker is valid, the script runs real `generate` preflights for all five fixed owners into fresh temporary directories. A manifest or theme-selection error therefore leaves an existing owned output and its marker unchanged. Later native build or filesystem failures can leave a partial replacement for inspection. The standalone `example` and per-demo targets omit showcase context and keep their local navigation. See [site configuration](../configuration/) for the manifest and CLI options.

## Understand documentation build output

The docs targets use separate Make output directories: `target/project-docs-build` for `make docs` and `target/project-docs-prefix-build` for `make docs-prefix`. These names are Make conventions that keep the two builds separate; the generator does not require those names. A direct CLI caller chooses the build directory with `--output` and the URL prefix with `--base-path`. A URL prefix is the leading path used by a site hosted below a domain root; for example, the page at `/docs/` becomes `/akibiyori-docs/docs/` when the prefix is `/akibiyori-docs/`.

Each build directory has two stages:

```text
target/project-docs-build/
├── source/    generated Cobalt configuration, Markdown, layouts and assets
└── site/      completed site to preview or deploy

target/project-docs-prefix-build/
├── source/    generated Cobalt input for the prefixed build
└── site/
    └── akibiyori-docs/
        └── docs/
            └── index.html
```

Akibiyori generates the files under `source/`. Cobalt reads that directory and writes the finished pages and assets into `site/`; Pagefind then indexes the served site directory. These are successive pipeline stages, not two separate websites to publish. The deployable tree is `site/`. Keeping `source/` lets you inspect or reuse the generated Cobalt project separately from its final HTML—for example, to examine the generated configuration and Markdown or to run Cobalt again while adjusting its project files.

The file-to-URL mapping shows which directory belongs at the server root:

| Build | File in generated site | URL |
| --- | --- | --- |
| `make docs` | `target/project-docs-build/site/docs/index.html` | `/docs/` |
| `make docs-prefix` | `target/project-docs-prefix-build/site/akibiyori-docs/docs/index.html` | `/akibiyori-docs/docs/` |

For both builds, serve the containing `site/` directory. For the prefix build, serving `site/akibiyori-docs/` as the server root strips the `akibiyori-docs` path segment, while generated links and assets still request URLs beginning with `/akibiyori-docs/`; those requests then look for a second `akibiyori-docs/` child and fail. The copyable Python 3 preview commands and exact local URLs are in the [project Docs overview](../).

For a standalone site, keep authored content and custom files in the original project directory. Akibiyori's config has no generic asset-copy option, so copy author-owned assets into a new output only after the build succeeds:

```sh
akibiyori build --config config.json --output target/local-build
cp -R assets target/local-build/site/assets
```

For a prefixed output, stage files beneath the prefix in the site tree. For example, with `--base-path /weather-docs`, copy to `target/prefix-build/site/weather-docs/assets/`. The physical-to-URL mapping is:

| Output file | Public URL |
| --- | --- |
| `site/index.html` | `/` |
| `site/assets/station.svg` | `/assets/station.svg` |
| `site/weather-docs/index.html` | `/weather-docs/` |
| `site/weather-docs/assets/station.svg` | `/weather-docs/assets/station.svg` |

Do not copy over generated CSS, scripts, fonts or the Pagefind bundle. For a direct Cobalt pipeline, run `akibiyori generate`, copy the original `assets/` into generated `source/`, run `cobalt build --config _cobalt.yml --destination ../site` from that source directory, then run `pagefind --site target/manual-build/site`. Cobalt copies non-page source files into its destination. This leaves authors editing the original input and keeps generated Cobalt source disposable. Pagefind searches the resulting HTML, not the contents of arbitrary downloads.

To revise content, build into a new output directory, such as `target/local-build-v2`. The CLI requires an absent or empty output root. A later Cobalt/Pagefind failure can leave partial output; inspect it and remove only a task-owned directory. Do not overwrite or adopt an unrelated directory, and do not use blanket cleanup commands. Serve the containing `site/` directory as the preview root; for prefixed builds, serving the inner prefix directory would drop the URL prefix.

## Respect output ownership

The native Akibiyori CLI requires its build output to be absent or an empty real directory. Make recipes use ownership markers for their managed outputs. The demo-site script also refuses a symlink or an output that it cannot prove it owns; inspect such a path rather than deleting or adopting it. If a build fails partway, it leaves the partial output for inspection and does not write a success marker. Remove only a confirmed task-owned output, then rerun.

`make clean-example` removes only the existing marked example outputs. It does not promise to clean arbitrary directories or user data. The documentation/demo hub uses its own marked output; do not point it at a source checkout or a directory containing unrelated files.

## Read changes as source changes

Before updating an example, check its pinned source, contract route counts, license and notices. Keep the original input and any separately identified correction distinct. A rendered summary or generated sample is a bounded reading aid, not proof of full OpenAPI/JSON Schema validity. See [public fixture provenance](../public-fixtures/) and [credits](../credits/).
