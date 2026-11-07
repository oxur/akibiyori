# Akibiyori Docs

These pages explain how to build and maintain Akibiyori sites from a source checkout. Start with the [quick start](quickstart/) to build these project docs, then use the guide that matches your task.

## Build and learn

- [Install](install/) Rust, Cobalt and Pagefind from their supported sources.
- [Quick start](quickstart/) distinguishes this project documentation from the Observatory tutorial and the complete demo showcase.
- [Author Markdown](authoring/) explains authored pages, route-relative links and generated frontmatter.
- [Configure a site](configuration/) covers strict JSON configuration and the optional reference-reader settings.
- [Build and maintain](maintenance/) describes repeatable native builds and output ownership.

## Understand the reader

- [Theme authoring](themes/) documents the built-in and local theme contracts.
- [Reference reader](reference-reader/) describes supported contract reading, examples and limits.
- [Public fixture provenance and bounds](public-fixtures/) records the separate public inputs and their measured limits.
- [Third-party notices](credits/) identifies adapted material and retained licenses.

## Keep the examples distinct

The [demo hub](../demos/) assembles four separate public inputs. The Observatory contract is a tutorial for the generated reader; it is not an API reference for Akibiyori's Rust library or command line. The Scalar Galaxy API and its separately credited upstream Fumadocs prose are different destinations. Each demo keeps its own generated routes and search index, and none of the examples is contacted as a live service.

## Build these project docs

From the repository root, run the native documentation target:

```sh
COBALT=cobalt PAGEFIND=pagefind make docs
```

It writes generated Cobalt source under `target/project-docs-build/source` and the site under `target/project-docs-build/site`. Serve the site directory as the root and visit `/docs/`. For a mounted build, run:

```sh
COBALT=cobalt PAGEFIND=pagefind make docs-prefix
```

This writes generated Cobalt source under `target/project-docs-prefix-build/source` and the site under `target/project-docs-prefix-build/site`. Serve the site directory as the root and visit `/akibiyori-docs/docs/`; keep its `akibiyori-docs/` child in place. If either native tool is not on `PATH`, set `COBALT` and `PAGEFIND` to its executable path. The build uses the real Akibiyori CLI, Cobalt and Pagefind. Inspecting generated files does not require a server, but browser search needs the actual local HTTP and index context.
