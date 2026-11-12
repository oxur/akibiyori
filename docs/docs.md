# Akibiyori Docs

These pages explain how to build and maintain Akibiyori sites from a source checkout. Start with the [quick start](quickstart/) to build these project docs, then use the guide that matches your task.

## Build and learn

- [Install](install/) Rust, Cobalt and Pagefind from their supported sources.
- [Quick start](quickstart/) distinguishes this project documentation from the Observatory tutorial and the complete demo showcase.
- [Author Markdown](authoring/) explains authored pages, route-relative links and generated frontmatter.
- [Configure a site](configuration/) covers strict JSON configuration and the optional reference-reader settings.
- [Build and maintain](maintenance/) describes repeatable native builds and output ownership.

## Design the reader

- [Design guidelines](design/) introduces the current reading principles, colour and state roles, typography, layout, and shared specimens.

## Understand the reader

- [Theme authoring](themes/) documents the built-in and local theme contracts.
- [Reference reader](reference-reader/) describes supported contract reading, examples and limits.
- [Public fixture provenance and bounds](public-fixtures/) records the separate public inputs and their measured limits.
- [Third-party notices](credits/) identifies adapted material and retained licenses.

## Keep the examples distinct

The [demo hub](../demos/) assembles four separate public inputs. The Observatory contract is a tutorial for the generated reader; it is not an API reference for Akibiyori's Rust library or command line. The Scalar Galaxy API and its separately credited upstream Fumadocs prose are different destinations. Each demo keeps its own generated routes and search index, and none of the examples is contacted as a live service.

## Build these project docs

After [installing the build tools](install/), run this from the repository root:

```sh
make docs
```

This creates `target/project-docs-build/`. Its `source/` directory holds the Cobalt project Akibiyori generates; `site/` holds the finished website to serve or publish. Make finds `cobalt` and `pagefind` on `PATH` by default. See [Build and maintain](maintenance/) for the full pipeline and optional custom tool locations.

To preview with Python 3, run:

```sh
python3 -m http.server 8000 --bind 127.0.0.1 --directory target/project-docs-build/site
```

Open <http://127.0.0.1:8000/docs/>. Leave the server running while browsing. Press Ctrl+C in that terminal to stop it before running the next commands. Python 3 is only needed for preview, not for the Make build.

For a build that will be served below a URL path such as `/akibiyori-docs/`, run:

```sh
make docs-prefix
```

That leading path is the URL prefix: `/docs/` becomes `/akibiyori-docs/docs/`. This target uses a separate build directory, so it can coexist with `make docs`. Preview it with:

```sh
python3 -m http.server 8000 --bind 127.0.0.1 --directory target/project-docs-prefix-build/site
```

Open <http://127.0.0.1:8000/akibiyori-docs/docs/>. The server root maps `site/akibiyori-docs/docs/index.html` to that URL. See [Build and maintain](maintenance/) for the directory tree and why the containing site directory is the right server root.
