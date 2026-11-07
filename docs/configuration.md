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


## Theme selection

Theme identity and the visitor's default mode are configured for the whole site. Omit `theme` to retain the Fumadocs Dark default, or select the bundled `fumadocs` or `neutral-example` identity and a `dark`, `light`, or `system` default. Local manifests use the same strict version-1 role and asset contract. See the [theme authoring guide](../themes/) and its [canonical reader design-system map](../design-system/).