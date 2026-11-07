# Configure a site

A config is strict versioned JSON. Contract, page, and asset paths are relative to the config file and cannot use `..` segments. A site needs a contract plus an authored `/reference/` page. An authored `/` home page is optional but gives readers a useful entry point:

```json
{
  "version": 1,
  "title": "My API guide",
  "contracts": [
    {"id": "catalog", "label": "Catalog tutorial", "path": "contracts/catalog.json"}
  ],
  "pages": [
    {"source": "content/index.md", "title": "My API guide", "route": "/"},
    {"source": "content/reference.md", "title": "Catalog tutorial", "route": "/reference/"}
  ],
  "navigation": [
    {"label": "Overview", "href": "/reference/"},
    {"label": "Search", "href": "/search/"}
  ]
}
```

All internal navigation destinations must match an authored or generated route. Routes are case-preserving and end in `/` except `/`. Unknown config fields and unsupported versions fail rather than being ignored.

## Optional reader modes

`reference` is a site-level reader setting shared by the configured contracts. This site leaves it out so the reader uses the existing `source`, `plain`, and `link` defaults. A site can opt into columns, sanitized description Markdown, or HTTPS-only Markdown images:

```json
{
  "reference": {
    "layout": "columns",
    "descriptions": "markdown",
    "images": "https"
  }
}
```

The [reference reader guide](../reference-reader/) explains the resulting operation layout, sanitizer and example boundaries. Set `descriptions` to `markdown` explicitly when sanitized Markdown descriptions are wanted; choosing columns alone does not enable them. These options affect contract descriptions, not authored page Markdown.

Use `document_uri` as a logical identifier for local reference resolution; it does not permit network fetching. Contract JSON and supported JSON-compatible YAML preserve source order. Unsupported YAML features and unresolved or unsafe references fail before output. The [Planets input guide](../planets-input/) describes the resource and dynamic-scope behavior with a real OpenAPI 3.1 example.



## Optional showcase context

A site normally builds with only its own local navigation. The first-party demo hub opts into a shared Showcase context by passing a version-1 manifest and the current site ID together. The manifest lives at the repository root in `akibiyori-showcase.json`; each `config` path is resolved from the manifest's directory, while `mount` and `entry_route` describe that site's public URL. The consumer manifest names the Docs route, the OpenAPI category route, and four demo entries. Each route is included only when it is authored by the site named as its owner. Before writing output, the generator loads every named config through the ordinary strict config reader and checks that the selected current site's canonical config file and input root match the config being built.

For example, building the Planets reference as part of the hub supplies both options:

```sh
akibiyori build \
  --config examples/planets.json \
  --output target/planets \
  --base-path /demos/planets \
  --showcase-manifest akibiyori-showcase.json \
  --showcase-site planets
```

The pair is optional on both `generate` and `build`; omitting both preserves the ordinary single-site build, and supplying only one is an error. Rust callers use `ShowcaseOptions { manifest, site }` with `generate_with_showcase` or `build_with_showcase`. Existing `generate`, `build`, and `BuildOptions` callers do not need to change.

Each site's normal navigation, local Section links, Search URL, and Mode preference remain scoped to that site's base path. Showcase links use the composition root plus the named owner's mount and route, so the same manifest works at the domain root and below a prefix. The generated public projection contains site IDs, labels, URLs, and current flags; filesystem config paths remain generator inputs and are not emitted into the site.

## Theme selection

Theme identity and the visitor's default mode are configured for the whole site. Omit `theme` to retain the Fumadocs Dark default, or select the bundled `fumadocs` or `neutral-example` identity and a `dark`, `light`, or `system` default. Local manifests use the same strict version-1 role and asset contract. See the [theme authoring guide](../themes/) and its [canonical reader design-system map](../design-system/).
