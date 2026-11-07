# Themes

Akibiyori currently ships the Fumadocs dark reference theme. A site can keep that default or select a local dark-theme manifest. Theme selection is a site configuration option; it does not change the reference layout, content, or navigation.

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

The built-in theme emits an empty theme override stylesheet and leaves the pinned Fumadocs and Akibiyori role values in place. `theme` is a strict object. An explicit `null`, unknown fields, or a source or mode not listed here is an error. This release supports `dark` only; it does not provide light mode, system mode, remote themes, or a theme selector in the generated site.

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

`version`, `id`, `extends`, and `modes.dark.colors` are required. `extends` must be `fumadocs`. `stylesheet` and `assets` may be omitted; when present they must have the declared type and cannot be `null`. Unknown or duplicate fields, duplicate color keys, additional modes, and unsupported values are rejected. Theme IDs start with a lowercase ASCII letter and contain only lowercase letters, digits, and hyphens.

The `colors` object accepts only the 37 existing `--aki-color-*` roles documented in [the design system](../theme/DESIGN_SYSTEM.md#theme-colour-roles). Values must be hexadecimal colors in `#RGB`, `#RGBA`, `#RRGGBB`, or `#RRGGBBAA` form. The compiler also projects the twelve mapped Akibiyori roles to their corresponding Fumadocs role at the actual body and sidebar owners. Other roles remain Akibiyori-only. Only roles explicitly named in the manifest are overridden, so derived local tints continue to respond to their inputs unless the author explicitly overrides the derived role itself.

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

Generation writes the compiled role stylesheet at `css/akibiyori-theme.css`, theme metadata at `_data/akibiyori_theme.json`, and any optional author stylesheet/assets below `themes/<id>/`. The default and search layouts load the generated stylesheet after the Akibiyori adapter; the optional author stylesheet loads last, after Pagefind UI CSS on the search page. The HTML keeps the dark class and receives escaped `data-aki-theme` and `data-aki-mode` attributes.

The root and prefix example targets build the same Observatory pages and API with the included `neutral-example` theme:

```sh
COBALT=/path/to/cobalt PAGEFIND=/path/to/pagefind make theme-example
COBALT=/path/to/cobalt PAGEFIND=/path/to/pagefind make theme-example-prefix
```

Use Cobalt 0.20.4 and Pagefind 1.5.2, the versions checked by the project. These targets write to owned directories below `target/example/`; their markers prevent cleanup of an unowned output. A deployment mounted below a URL prefix must preserve that prefix when serving the generated tree.

## Updating theme support

The bundled Fumadocs CSS and fonts are pinned third-party inputs with notices in the repository. Keep their source pins, copied bytes, and notices aligned when updating them. The role list and Fumadocs projections are explicit rather than a general CSS-variable API. Adding another mode, palette generator, remote source, theme UI, CSS sanitizer, or font validator needs separate design and verification; none is implied by the current local dark-theme format.
