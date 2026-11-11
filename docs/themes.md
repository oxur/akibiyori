# Theme authoring

A site uses Deploy presentation unless `theme.presentation` is set to `demo`. Deploy selects a Dark and Light profile and a default visitor mode; the native Mode control chooses between those profiles, whose family identities may differ. Demo exposes the complete ordered catalog through a native Theme control. If `theme.demo_default` is omitted, the mapped Deploy Dark profile supplies the initial Demo profile; when present, the value must be a non-null ID from the full catalog. The [canonical reader design system](../design-system/) maps role names, values, owners and source limits. Theme notices and third-party attribution are in [Credits](../credits/).

## Built-in themes and defaults

When `theme` is omitted, Akibiyori uses the Fumadocs built-in with a Dark default. The following explicit selection is equivalent:

```json
"theme": { "source": { "builtin": "fumadocs" }, "mode": "dark" }
```

The generic `neutral-example` compatibility theme is also available as a built-in. The file-based Observatory example is defined in source by `examples/observatory/config-theme.json`; it is not the same transport: built-in bytes are packaged with Akibiyori, while a file theme is read relative to its site configuration.

```json
"theme": { "source": { "builtin": "neutral-example" }, "mode": "system" }
```

The complete runnable Observatory config is the source file `examples/observatory/config-theme-builtin.json`; build it with `make theme-builtin-example` or `make theme-builtin-example-prefix`. The prefix recipe mounts the site at `/theme-builtin`. The existing external-file example remains `examples/observatory/config-theme.json`, built by `make theme-example` and `make theme-example-prefix`.

`mode` is `dark`, `light`, or `system`. It is the visitor's starting preference, not a third theme palette. In Deploy, System resolves initially to Dark and then follows the browser's current `prefers-color-scheme` value when the local adapter can read it. In Demo, `demo_default` sets the static profile and its Light/Dark class; Mode/System does not choose a Demo profile. Without JavaScript, the static selection remains usable and the enhanced control stays hidden.

## Named deploy profiles

The additive catalog API selects the two profiles used by a deployment. `catalog` and `deploy` are required together and are mutually exclusive with legacy `source`; `mode` remains the default visitor preference. The catalog may contain built-in Fumadocs, House, Forest, or Botanist families and local manifest families, while `deploy.dark` must name a Dark variant and `deploy.light` a Light variant:

```json
"theme": {
  "catalog": [
    { "builtin": "fumadocs" },
    { "file": "themes/neutral-profile/theme.json" }
  ],
  "deploy": { "dark": "neutral-night", "light": "fumadocs-light" },
  "mode": "system",
  "presentation": "demo",
  "demo_default": "neutral-night"
}
```

The required `deploy` pair remains valid in both presentations. Deploy packages only that pair and their dependencies. Demo packages all catalog profiles in catalog order, and their required assets. `presentation` accepts only `deploy` or `demo`; omitted means Deploy. When supplied, `demo_default` must be a non-null ID from the complete catalog. This validation still applies when a caller overrides the requested presentation to Deploy. Profile selectors include family and variant identity. A selected profile may name a trusted author stylesheet; the adapter keeps one active stylesheet and removes it when switching to a profile without one. Labels are display text, not IDs. Literal Liquid-looking text in a label or font name stays data and is not evaluated as template source.

The browser chooses a valid single `theme` query parameter first, then a valid stored Demo selection, then `demo_default` or, when omitted, the mapped Deploy Dark profile. A valid query choice is saved. Selecting a profile updates the current URL with `history.replaceState`, preserving other query values, the path and the fragment. The Demo preference key is `akibiyori:theme:v1:<base-path>` and is independent of the Deploy Mode key `akibiyori:mode:v1:<base-path>`. Deploy ignores Demo query and storage state. The optional CLI override is `--theme-mode deploy|demo` for both `akibiyori generate` and `akibiyori build`. Rust callers can compose options with existing showcase settings:

```rust
let options = GenerationOptions::default()
    .with_theme_mode(ThemePresentation::Demo)
    .with_showcase(ShowcaseOptions { manifest, site })
    .with_showcase_expected_sites(vec![
        "docs".to_owned(),
        "observatory".to_owned(),
        "planets".to_owned(),
        "kubernetes".to_owned(),
        "petstore".to_owned(),
    ]);
generate_with_options(&config, &source_output, "/docs", &options)?;
build_with_options(&build_options, &options)?;
```

Existing entry points remain available. The runnable standalone six-profile Observatory config is `examples/observatory/config-theme-demo.json`; build the root and `/theme-demo` variants with `make theme-demo-example` and `make theme-demo-example-prefix`. It intentionally keeps its custom Neutral Night and long “Neutral Day with an intentionally longer display name” profiles; those supported author-defined profiles are separate from the curated first-party picker.

## One theme for a composed showcase

A strict v1 showcase manifest can supply both a normal `theme` selection and a stable `theme_scope`. Omit both to retain the existing member-local theme behavior. The curated first-party picker has fifteen profiles in this order: Fuma Dark/Light, Slate Night/Day, the six House profiles, the four Forest profiles, and Botanist Day. Slate Night and Slate Day are display labels for the existing `neutral-example-dark` and `neutral-example-light` IDs; the neutral-example family, stylesheet path, and legacy configuration remain stable. The standalone custom-manifest examples remain supported separately. House provides Maple & Mist Light, Maple & Mist Dark, Golden Hour Light, Golden Hour Dark, Ember & Ink Light, and Ember & Ink Dark; the showcase opens Demo on Maple & Mist Light and keeps Ember & Ink Dark with Maple & Mist Light as its Deploy pair:

```json
{
  "theme_scope": "akibiyori-showcase",
  "theme": {
    "catalog": [
      { "builtin": "fumadocs" },
      { "builtin": "neutral-example" },
      { "builtin": "house" },
      { "builtin": "forest" },
      { "builtin": "botanist" }
    ],
    "deploy": { "dark": "ember-ink-dark", "light": "maple-mist-light" },
    "presentation": "demo",
    "demo_default": "maple-mist-light",
    "mode": "system"
  }
}
```

The first-party catalog includes the four-profile Forest family after House and Botanist after Forest. Forest adds `forest-light`, `forest-dusk`, `forest-storm`, and `forest-midnight`; Botanist adds the Light profile `botanist-day`. The existing House selection and initial Maple Light choice remain unchanged. A standalone mixed-family example is `examples/observatory/config-theme-botanist.json`, built with `make botanist-example` and `make botanist-example-prefix` at `/botanist-theme`. The file-family path is relative to the showcase manifest directory, not a member config. Catalog dependencies are validated and packaged by the same resolver used for a standalone site. Every member config must omit theme or declare only the implicit Deploy defaults; an explicit source/catalog, pair, nondefault presentation, Demo default, or nondefault visitor mode conflicts. `theme_scope` follows the lowercase ASCII theme-ID grammar: it starts with a lowercase letter and continues with lowercase letters, digits, or hyphens. The shared Demo key is `akibiyori:theme:v1:showcase:<scope>:<composition-root-or-/>`: all members in a root or prefix assembly share the choice, while different scopes and composition roots remain separate. The existing Mode key stays member/base-path scoped and Deploy ignores Demo state.

The selected House roles and their reader adaptations are described in the [House colour guide](../design/house/). The [Forest theme family](../design/forest/) documents its four profiles, Georgia heading role, and shared callouts. The [Botanist Day guide](../design/botanist/) documents its single Light profile, local inverse component owners, and mixed-family example.

Both `generate` and `build` accept `--showcase-expected-sites` for an exact comma-delimited ID set. `SHOWCASE_MANIFEST` selects the assembly manifest and defaults to `akibiyori-showcase.json`; the shell assembly passes the expected set to its CLI preflights and builds. The script runs real `generate` preflights for all five fixed owners into temporary output directories before replacing an owned assembly. A rejected central selection leaves an existing owned output and its marker intact.

A v1 manifest contains exactly one of the legacy `modes` object or a named `variants` list. `source` continues to accept only a legacy `modes` manifest; use `catalog` to select named variants. `extends` accepts `fumadocs` or `default`. Optional `defaults` supplies shared colors, font roles, and geometry; a profile can override each role. Omitted Light in a legacy modes manifest keeps the Fumadocs Light base and does not inherit Dark colors. Color roles retain their existing validation and owner behavior.

A profile's `fonts` map accepts `body`, `heading`, and `code`. Each explicitly supplied stack is complete: optional local `face`, ordered `families`, and a required `generic` (`sans-serif`, `serif`, `monospace`, or `system-ui`). It must include a local `face` or at least one literal family; a generic alone is invalid. The family list may be empty when a face is supplied. The shared/variant maps merge per role. If heading is absent after resolution, it follows the resolved body stack at each actual body/sidebar owner; an explicit heading stack remains independent. Bundled Geist and JetBrains Mono bytes are included only when a selected profile needs them; these ASCII family names are recognized case-insensitively in any role. No font is unconditionally preloaded.

Local `font_faces` records point to regular font files below the config directory. Repeated family IDs group style and weight records; a stack references the ID with `face`. Records use `normal` or `italic` style and an inclusive weight interval from 1 to 1000. Overlapping intervals for the same ID and style fail. When a selected profile references one face ID, all records for that ID are packaged under generated namespaced CSS families and safe filenames containing ID, style, and range. The generator copies the source bytes unchanged; it does not inspect glyph coverage or prove italic rendering. Keep the font license with the site.

Typed `geometry` supports `border_width`, the named component radii, and menu/sidebar/dialog shadows. Lengths use nonnegative `px` or `rem`; shadow x/y and spread may be signed. Shadow colors are optional hex values and otherwise use the existing owner-local overlay roles. Only declared values are emitted, so omitted typed geometry leaves shared defaults and trusted CSS behavior intact.

A compact local-face and geometry declaration looks like this:

```json
{
  "font_faces": [
    { "id": "my-sans", "source": "author/regular.woff2", "weight": [100, 400], "style": "normal" },
    { "id": "my-sans", "source": "author/bold.woff2", "weight": [401, 900], "style": "normal" },
    { "id": "my-sans", "source": "author/italic.woff2", "weight": [100, 900], "style": "italic" }
  ],
  "defaults": {
    "fonts": {
      "body": { "face": "my-sans", "families": ["Arial"], "generic": "sans-serif" }
    },
    "geometry": {
      "border_width": { "value": 1, "unit": "px" },
      "radii": { "copy-code": { "value": 0.5, "unit": "rem" } },
      "shadows": {
        "menu": {
          "x": { "value": 0, "unit": "px" },
          "y": { "value": 8, "unit": "px" },
          "blur": { "value": 24, "unit": "px" },
          "spread": { "value": 0, "unit": "px" },
          "color": "#0000003d"
        }
      }
    }
  }
}
```

The face `id` groups records for one CSS family; weight intervals are inclusive and may not overlap for the same style. `source` must be a local regular file with a supported font extension. A role can refer to that group through `face`. The only radius keys are `panel`, `navigation`, `selector-menu`, `endpoint`, `example-card`, `schema-card`, `small-control`, `schema-chip`, `copy-code`, and `callback-dialog`; shadow keys are `menu`, `sidebar`, and `dialog`. `border_width`, each radius length, and shadow `blur` are nonnegative; shadow `x`, `y`, and `spread` may be signed. Units are `px` or `rem`. When a typed radius or shadow is omitted, its existing shared CSS value remains in effect, including any trusted author override.


The runnable mixed-pair Observatory example is `examples/observatory/config-theme-pair.json`; build the root and `/theme-pair` variants with `make theme-pair-example` and `make theme-pair-example-prefix`.

## Local theme manifest

A file theme is selected with a path relative to the config file:

```json
"theme": { "source": { "file": "themes/my-example/theme.json" }, "mode": "system" }
```

A legacy source manifest uses the strict `modes` shape. Here is a complete small example with the generic theme's declared Dark and Light action values:

```json
{
  "version": 1,
  "id": "my-example",
  "extends": "fumadocs",
  "modes": {
    "dark": { "colors": { "--aki-color-action": "#fafafa" } },
    "light": { "colors": { "--aki-color-action": "#171717" } }
  },
  "stylesheet": "overrides.css",
  "assets": []
}
```

`version`, `id`, `extends`, and `modes.dark.colors` are required for a legacy source manifest. `extends` accepts `fumadocs` or `default`. `modes.light` may be omitted; omission inherits the built-in Fumadocs Light palette, not the author's Dark overrides. If present, Light must be an object with a required `colors` object. Explicit `null`, unknown or duplicate fields, duplicate color keys, additional modes and unsupported values fail. IDs begin with a lowercase ASCII letter and contain only lowercase ASCII letters, digits and hyphens.

`colors` accepts the 40 `--aki-color-*` roles in the [design system](../design-system/). Values are `#RGB`, `#RGBA`, `#RRGGBB`, or `#RRGGBBAA` hexadecimal. Twelve roles also project to their corresponding Fumadocs input at the existing body and sidebar owners; the other roles affect their Akibiyori consumers. The compiler overrides only listed roles, so local derived tints continue to follow their source inputs unless the author explicitly supplies that derived role.

The optional `stylesheet` path is relative to the manifest and must be a `.css` file. Its bytes are copied to `themes/<id>/styles.css`. CSS is trusted author input: it is not parsed, sanitized, fetched or rewritten. An optional `assets` list contains files relative to the manifest and public targets below `themes/<id>/`:

```json
"assets": [
  { "source": "author/geist-latin.woff2", "target": "fonts/geist-latin.woff2" },
  { "source": "author/geist-OFL.txt", "target": "licenses/geist-OFL.txt" }
]
```

Sources must be regular files under the config directory, without symlink components, and use `.woff2`, `.woff`, `.ttf`, `.otf`, or `.txt`. Public target path segments use portable ASCII letters, digits, hyphens, underscores and dots; a segment cannot begin with `.` or `_`. The `styles.css` target is reserved. Duplicate/case-folded targets, file-directory or generated-route collisions fail before output is written. Asset bytes are copied unchanged. The generator does not validate font contents, license terms or CSS syntax, fetch remote files, copy directories or rewrite `url(...)`; authors own those decisions and notices.

## External author walkthrough

This walkthrough starts with a fresh copy of the public Observatory example, edits only that copy, and runs the actual CLI and native site tools. From the repository root:

```sh
make check
AUTHOR_DIR="$(mktemp -d "${TMPDIR:-/tmp}/my-observatory.XXXXXX")"
cp -R examples/observatory/. "$AUTHOR_DIR/"
mkdir -p "$AUTHOR_DIR/themes/my-example/author"
cp theme/vendor/fonts/geist-latin.woff2 "$AUTHOR_DIR/themes/my-example/author/geist-latin.woff2"
cp theme/vendor/licenses/geist-OFL.txt "$AUTHOR_DIR/themes/my-example/author/geist-OFL.txt"
```

The copied font is an existing example asset; keep its OFL text with it. If you use another font, provide its own license and retain it with your site. Create `$AUTHOR_DIR/themes/my-example/theme.json`:

```sh
cat > "$AUTHOR_DIR/themes/my-example/theme.json" <<'JSON'
{
  "version": 1,
  "id": "my-example",
  "extends": "fumadocs",
  "modes": {
    "dark": { "colors": { "--aki-color-action": "#fafafa" } },
    "light": { "colors": { "--aki-color-action": "#171717" } }
  },
  "stylesheet": "overrides.css",
  "assets": [
    { "source": "author/geist-latin.woff2", "target": "fonts/geist-latin.woff2" },
    { "source": "author/geist-OFL.txt", "target": "licenses/geist-OFL.txt" }
  ]
}
JSON
```

Create `$AUTHOR_DIR/themes/my-example/overrides.css`:

```sh
cat > "$AUTHOR_DIR/themes/my-example/overrides.css" <<'CSS'
@font-face {
  font-family: "My Example Geist";
  src: url("fonts/geist-latin.woff2") format("woff2");
  font-style: normal;
  font-weight: 100 900;
  font-display: swap;
}

body.framework {
  --aki-font-body: "My Example Geist", Geist, sans-serif;
  font-family: var(--aki-font-body);
}
CSS
```

The URL is relative to the emitted stylesheet at `themes/my-example/styles.css`; the declared target is copied below that theme directory. The `--aki-font-body` override is trusted CSS and does not alter the typed role map. Set `font-family` on the actual `body.framework` consumer (and separately on any other intended owner); a descendant role override cannot change a font-family already computed on an ancestor. The shared adapter keeps heading and compatibility aliases owner-aware, but a custom stylesheet still controls its own selectors.

Create `$AUTHOR_DIR/site.json` with the copied contract and authored page paths:

```sh
cat > "$AUTHOR_DIR/site.json" <<'JSON'
{
  "version": 1,
  "title": "My Observatory",
  "contracts": [
    { "id": "catalog", "label": "Catalog contract", "path": "contracts/catalog.json" }
  ],
  "pages": [
    { "source": "content/index.md", "title": "My Observatory", "route": "/" },
    { "source": "content/reference.md", "title": "Catalog API reference", "route": "/reference/" },
    { "source": "content/docs/reading-an-observation.md", "title": "Reading an observation", "route": "/docs/reading-an-observation/" }
  ],
  "navigation": [
    { "label": "Learn", "href": "/docs/reading-an-observation/" },
    { "label": "Search", "href": "/search/" },
    { "label": "Reference", "href": "/reference/" }
  ],
  "theme": {
    "source": { "file": "themes/my-example/theme.json" },
    "mode": "system"
  }
}
JSON
```

The actual CLI build command is:

```sh
COBALT=/path/to/cobalt
PAGEFIND=/path/to/pagefind
CLI="$PWD/target/observatory-public/debug/akibiyori"
"$CLI" build --config "$AUTHOR_DIR/site.json" --output "$AUTHOR_DIR/root" \
  --cobalt "$COBALT" --pagefind "$PAGEFIND"
"$CLI" build --config "$AUTHOR_DIR/site.json" --output "$AUTHOR_DIR/prefix" \
  --base-path /author-example --cobalt "$COBALT" --pagefind "$PAGEFIND"
```

Use Cobalt 0.20.4 and Pagefind 1.5.2, the versions exercised by the project. Akibiyori does not download them. See the installation guide for the [Cobalt install instructions](../install/) and the native tools' official release information. To enable the API columns layout and sanitized Markdown descriptions for a modern OpenAPI input, add the existing site-level `reference` settings to the copied config; `layout: "columns"` and `descriptions: "markdown"` are separate opt-ins. The public [configuration guide](../configuration/) describes those settings.

The generated `/search/` is local to each output. A deployment mounted below a URL prefix must serve the prefixed tree at that same prefix. The project verifies actual root and `/author-example` outputs and bytes; it does not make a live API call or install a Rust fork, npm package or second renderer.

## Generated state, trust and limits

In Deploy, generation emits `css/akibiyori-theme.css` with only the mapped Dark and Light profile blocks. In Demo, it emits blocks for every explicit catalog profile, in catalog order. `_data/akibiyori_theme.json` records the selected pair and effective mode in Deploy; in Demo it also records the catalog profiles, initial `demo_default` profile, and its scheme. It carries a base-path-scoped Mode key and a Demo Theme key: standalone sites scope Theme by base path, while a composed showcase scopes it by `theme_scope` and composition root. The fixed local `js/akibiyori-mode.js` applies the selection. Selected family stylesheets and assets appear under `themes/<id>/`; generated local-face and bundled font CSS/bytes are included only for dependencies of the selected Deploy pair or Demo catalog. In Demo, a valid single `theme` query choice takes priority over a valid stored profile ID, followed by the author `demo_default`; Deploy uses the visitor Mode preference for its pair. A missing or invalid choice falls back to the presentation's author default. If storage cannot be read, the current rendered choice is kept; a failed write affects that page and is not claimed persistent.

The mode adapter runs after static stylesheets have been parsed and before `</head>`, updates the root profile and effective mode, and synchronizes the existing select on page-show and history navigation. It updates or removes the single active author stylesheet when the selected family changes. It schedules one coalesced deferred refresh after restored page state. It does not infer a browser cache mechanism or rebuild reader content. Static builds and tests do not prove dynamic operating-system transitions, exceptional storage behavior, every history path or absence of a no-flash interval.

The Fumadocs Light values and role-to-framework mapping are an explicit source projection, not a general contrast guarantee. Some amber selected, muted and syntax pairs fail or remain untested. Native Cobalt syntax classes do not always correspond to upstream Shiki token classes; unsupported/plain language labels are not assigned a guessed grammar. In Light, highlighted article code outside API panels uses the article foreground role; its existing inner code-box background, border and geometry remain. Parsed token classes retain their colors, raw source JSON remains ordinary text, and API examples keep their separate roles. Arbitrary author CSS is outside the semantic role contract.

## Maintenance

The role map lives in the [reader design system](../design-system/); source-only paths such as `src/theme.rs`, `theme/css/akibiyori-tokens.css`, and `theme/vendor/PROVENANCE.md` are checkout files, not generated download URLs. Keep source pins, copied bytes and applicable notices aligned when changing upstream material. The local adapter switches between the author-selected Dark and Light pair in Deploy; in Demo it can select any explicit catalog profile. The format validates supported font declarations and safe asset paths, but does not inspect font bytes for glyph coverage or prove axis/italic behavior. Remote theme sources, CSS sanitization, CSS syntax checks and license-term validation are outside the local theme format.
