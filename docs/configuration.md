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



## Reference sidebar presentation

The generated reference sidebar remains a flat list by default. Set `reference_sidebar.presentation` to `contract-accordion` to group navigation by configured contract, then by each contract's source-ordered raw tags:

```json
{
  "reference_sidebar": {
    "overview_label": "API overview",
    "untagged_label": "Untagged operations",
    "presentation": "contract-accordion"
  }
}
```

Each nonempty contract receives a heading from its `contracts[].label`, so identical tag display names in separate contracts remain distinct. Tag headings use native, initially collapsed disclosure controls; readers can open several groups at once. When a generated operation link is the current page, its containing tag group opens on page setup. Disclosure state is not persisted. The flat presentation remains the default when the `presentation` field is omitted, and `flat` selects that same output explicitly. This setting changes only generated API-reference navigation; it does not change operation order or reference routes.

The first-party Planets, Kubernetes, Petstore, and Observatory demos opt into `contract-accordion` in their own configs, so their existing reference navigation uses this presentation across each site's configured Theme profiles. Observatory has no source tags: its two operations appear together under the configured `untagged_label`. This is an example adoption, not a library-wide default. The strict upstream-comparison config at `examples/fumadocs-reference.json` omits `reference_sidebar` and keeps its authored manual navigation.

## Optional showcase context

A site normally builds with only its own local navigation and theme. The first-party demo hub opts into a shared Showcase context by passing a version-1 manifest and the current site ID together. The manifest lives at the repository root in `akibiyori-showcase.json`; each `config` path is resolved from the manifest's directory, while `mount` and `entry_route` describe that site's public URL. The consumer manifest names the Docs route, the OpenAPI category route, and four demo entries. Each route is included only when it is authored by the site named as its owner. Before writing output, the generator loads every named config through the ordinary strict config reader and checks that the selected current site's canonical config file and input root match the config being built.

For example, building the Planets reference as part of the hub supplies both options:

```sh
akibiyori build \
  --config examples/planets.json \
  --output target/planets \
  --base-path /demos/planets \
  --showcase-manifest akibiyori-showcase.json \
  --showcase-site planets \
  --showcase-expected-sites docs,observatory,planets,kubernetes,petstore
```

The pair is optional on both `generate` and `build`; omitting both preserves the ordinary single-site build, and supplying only one is an error. Rust callers use `ShowcaseOptions { manifest, site }` with `generate_with_showcase` or `build_with_showcase`. Existing `generate`, `build`, and `BuildOptions` callers do not need to change.

`--showcase-expected-sites` is an optional comma-delimited exact set for a caller that owns a fixed assembly. Empty IDs, duplicate IDs, missing or extra manifest members, or using the constraint without showcase context fail before output writes. General showcase generation can omit the constraint. Rust callers can compose it with `GenerationOptions::with_showcase_expected_sites(Vec<String>)`.

Each site's normal navigation, local Section links, Search URL, and Mode preference remain scoped to that site's base path. Search follows the brand in the sidebar. The generated Showcase context remains shared across the assembly, but its navigation appears only on categorized demo owners; central Docs keeps its authored Docs and Demos groups. On demo owners, Showcase follows the first authored docs group and precedes generated API links; its heading is Showcase and its Switch demo disclosure links to the four demos. The generated public projection retains site IDs, labels, URLs, current flags, and the current label as hidden context data; filesystem config paths remain generator inputs and are not emitted into the site. Showcase links use the composition root plus the named owner's mount and route, so the same manifest works at the domain root and below a prefix.

Demo theme choices are displayed by label in case-insensitive natural alphabetical order. This presentation does not change profile IDs, the selected default, or theme data order. The manifest may also declare both `theme` and `theme_scope`, or neither. When present, the strict normal theme selection is resolved from the showcase manifest directory and shared by every member. A member with an explicit nondefault local theme conflicts with that selection. The stable scope ID starts with a lowercase ASCII letter and contains only lowercase ASCII letters, digits, and hyphens; it becomes part of the Demo preference key together with the derived composition root. Root and prefix members therefore share a Demo choice, while another scope or composition root stays independent. The first-party hub shares its configured catalog and Demo default across every member. Deploy presentation ignores Demo theme query and storage data and continues to use each page's ordinary Mode preference.

## Theme selection

Theme presentation is configured for the whole site. Deploy is the default: omit `theme` to retain the Fumadocs Dark default, or select a Dark/Light pair from the bundled `fumadocs`, `neutral-example`, or local version-1 catalog. Set `theme.presentation` to `demo` to expose every catalog profile and set `theme.demo_default` to a valid catalog profile ID. The visitor `mode` remains the Deploy preference; Demo's initial profile comes from `demo_default`. Both `generate` and `build` accept `--theme-mode deploy|demo` to override the presentation. Local manifests use the same strict role and asset contract. See the [theme authoring guide](../themes/) and its [canonical reader design-system map](../design-system/).
