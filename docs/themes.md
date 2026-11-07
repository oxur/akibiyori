# Themes

Akibiyori ships static Fumadocs Dark and Light reference modes. A site can keep the Dark default, select built-in Light, or use a local manifest with mode-specific overrides. Theme selection is a site configuration option; it does not change the reference layout, content, or navigation.

## Default

Omit `theme` to use the same selection as:

```json
{
  "theme": {
    "source": { "builtin": "fumadocs" },
    "mode": "dark"
  }
}
```

The omitted selection and explicit `dark` selection retain the existing built-in Dark output, including its empty theme override stylesheet. Built-in `light` emits the pinned Light inputs for both body and sidebar owners. `theme` is a strict object. An explicit `null`, unknown fields, or a source or mode not listed here is an error. Dark and Light are static build-time modes; System preference detection, runtime switching, remote themes, and a theme selector are not provided.

## Built-in Light

Select the built-in Light mode with this `theme` object in a site's configuration:

```json
{
  "theme": {
    "source": { "builtin": "fumadocs" },
    "mode": "light"
  }
}
```

This is the theme fragment, not a complete site config. The [runnable Planets Light configuration](../examples/planets-light.json) shows the complete file and preserves the authored API pages. The [native Light inputs](../src/theme.rs) and [Light role values](../theme/css/akibiyori-tokens.css) are maintained in source; the [design-system guide](../theme/DESIGN_SYSTEM.md#static-light-source-mapping) and [vendor provenance](../theme/vendor/PROVENANCE.md) describe their mappings and source limits. The root and `/planets-light` builds are `make planets-light-example` and `make planets-light-example-prefix` above.

The palette follows the selected source values; it is not a general contrast certification. Source-faithful amber selected states, muted roles, and some Latte small-text/state pairs fail or remain untested. Native Cobalt syntax classes differ from the source Shiki classes, and the option mapping is an explicit class adaptation; unsupported or plain language labels are not assigned a guessed grammar. In Light mode, highlighted article code outside API panels receives the article foreground role while its existing inner code-box background, border, and geometry remain. Parsed token classes retain their colors. Unhighlighted raw Source JSON remains ordinary text, and API examples retain their separate API roles.

## Local theme manifest

To use a local theme, point `source.file` at a JSON manifest relative to the site config file:

```json
{
  "theme": {
    "source": { "file": "themes/neutral-example/theme.json" },
    "mode": "dark"
  }
}
```

A version 1 manifest must be a `.json` file and looks like this:

```json
{
  "version": 1,
  "id": "neutral-example",
  "extends": "fumadocs",
  "modes": {
    "dark": {
      "colors": {
        "--aki-color-action": "#fafafa"
      }
    }
  },
  "stylesheet": "overrides.css",
  "assets": []
}
```

`version`, `id`, `extends`, and `modes.dark.colors` remain required. `modes.light` may be omitted; if present, it must be an object with a required `colors` object. An omitted Light override inherits the complete built-in Fumadocs Light palette, not Dark overrides. Explicit `null` is invalid. `extends` must be `fumadocs`. `stylesheet` and `assets` may be omitted; when present they must have the declared type and cannot be `null`. Unknown or duplicate fields, duplicate color keys, additional modes, and unsupported values are rejected. Theme IDs start with a lowercase ASCII letter and contain only lowercase letters, digits, and hyphens.

The `colors` object accepts only the 40 `--aki-color-*` roles documented in [the design system](../theme/DESIGN_SYSTEM.md#theme-colour-roles). Values must be hexadecimal colors in `#RGB`, `#RGBA`, `#RRGGBB`, or `#RRGGBBAA` form. The compiler also projects the twelve mapped Akibiyori roles to their corresponding Fumadocs role at the actual body and sidebar owners. Other roles remain Akibiyori-only. Only roles explicitly named in the manifest are overridden, so derived local tints continue to respond to their inputs unless the author explicitly overrides the derived role itself.

The optional stylesheet path is relative to the manifest and must end in `.css`. Its bytes are copied to `themes/<id>/styles.css`; the stylesheet is trusted author CSS and is not parsed, sanitized, fetched, or rewritten. The optional `assets` list contains objects with a `source` relative to the manifest and a public `target` beneath `themes/<id>/`:

```json
{
  "source": "author/geist-latin.woff2",
  "target": "fonts/author.woff2"
}
```

Asset sources must be regular files within the config directory tree, without symlink components, and use `.woff2`, `.woff`, `.ttf`, `.otf`, or `.txt`. Public target segments use portable ASCII letters, digits, hyphens, underscores, and dots; segments cannot begin with a dot or underscore. The `styles.css` target is reserved for the optional stylesheet. Duplicate, case-folding, file/directory, route, and generated-asset collisions fail before output files are written.

Asset bytes are copied unchanged. CSS `url(...)` references should point at the matching path in the theme asset namespace, for example `url("fonts/author.woff2")`. The generator does not inspect font validity or license terms, rewrite URLs, fetch external files, copy directories, or validate CSS syntax or contrast. Theme authors are responsible for the CSS and asset licenses they add.

## Generated files and prefixes

Generation writes the compiled role stylesheet at `css/akibiyori-theme.css`, theme metadata at `_data/akibiyori_theme.json`, and any optional author stylesheet/assets below `themes/<id>/`. The default and search layouts load the generated stylesheet after the Akibiyori adapter; the optional author stylesheet loads last, after Pagefind UI CSS on the search page. The HTML class and escaped `data-aki-mode` attribute both reflect the effective static mode, and the escaped `data-aki-theme` identifies the selected theme.

The `theme-example` pair builds the Observatory with a local Dark theme. The `planets-light-example` pair builds the same authored Planets API pages with built-in Light:

```sh
COBALT=/path/to/cobalt PAGEFIND=/path/to/pagefind make theme-example
COBALT=/path/to/cobalt PAGEFIND=/path/to/pagefind make theme-example-prefix
COBALT=/path/to/cobalt PAGEFIND=/path/to/pagefind make planets-light-example
COBALT=/path/to/cobalt PAGEFIND=/path/to/pagefind make planets-light-example-prefix
```

The Light prefix target is mounted at `/planets-light`. These targets write to separately owned directories below `target/example/`; their markers prevent cleanup of an unowned output. Use Cobalt 0.20.4 and Pagefind 1.5.2, the versions checked by the project. A deployment mounted below a URL prefix must preserve that prefix when serving the generated tree.

## Updating theme support

The bundled Fumadocs CSS and fonts are pinned third-party inputs with notices in the repository. Keep their source pins, copied bytes, and notices aligned when updating them. The role list and Fumadocs projections are explicit rather than a general CSS-variable API. System mode and runtime mode switching remain separate work. Remote sources, a theme UI, CSS sanitizer, or font validator are not part of the local Dark/Light theme format.
