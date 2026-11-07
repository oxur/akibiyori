# Themes

Akibiyori ships Fumadocs Dark and Light palettes. A site author selects the theme identity and a default visitor mode; visitors can choose System, Light, or Dark in the reader shell. Theme identity remains site configuration and is not a runtime picker.

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

The omitted selection and explicit `dark` selection retain the existing Dark rendering and produce identical generated output. The compiled stylesheet also contains the inactive Light block so a visitor can switch modes locally. Built-in `light` selects the pinned Light inputs for both body and sidebar owners. `theme` is a strict object. An explicit `null`, unknown fields, or a source or mode not listed here is an error. The default mode may be `dark`, `light`, or `system`; a local manifest still defines only `modes.dark` and optional `modes.light`, never a `modes.system` palette. System uses the browser's current color preference and falls back to Dark when that preference cannot be read.

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

## Visitor mode and preference

The selected author default is emitted as `default_mode`; `mode` is the initial effective palette. For a System default, the native HTML starts in Dark, and the early-loaded local adapter selects Light only when the browser reports a Light color preference. Visitors can change the native Mode select to System, Light, or Dark. Without JavaScript the select remains hidden and the generated default remains usable.

A visitor choice is stored under `akibiyori:mode:v1:<base-path>`. The root site uses `akibiyori:mode:v1:/`; a site mounted at `/manual` uses `akibiyori:mode:v1:/manual`. The key comes from the configured native base path, not the current page URL or theme identity, so pages in one mount share a choice and separate mounts do not. A valid stored choice overrides the author default. Missing or invalid stored values use that default. If browser storage cannot be read, the page keeps its current choice; a failed write affects only that page and is not reported as persistent.

When a page is shown or history navigation changes the active document, the adapter rereads the stored value and synchronizes the existing select and root mode attributes. It also schedules one coalesced zero-delay refresh after the browser restores page state, so a restored native select is checked against the saved preference. This does not rebuild reader content, bind duplicate handlers, or claim that the browser used its back-forward cache. System reacts to the browser's current `prefers-color-scheme` result. In Light mode, the new Mode caption and its visible keyboard-focus outline use the existing text role; the rest of the Light palette and the control's geometry remain unchanged. Static pages and native tests do not prove dynamic operating-system changes, blocked/corrupt storage, every history-restoration path, or a no-flash guarantee; those require the actual browser capability and state to be observed.

The runnable [Planets System configuration](../examples/planets-system.json) changes only the theme preference from the original Planets config. Build it at root or under `/planets-system` with `make planets-system-example` and `make planets-system-example-prefix`.

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

Generation writes the compiled Dark-then-Light role stylesheet at `css/akibiyori-theme.css`, metadata at `_data/akibiyori_theme.json`, the fixed mode adapter at `js/akibiyori-mode.js`, and any optional author stylesheet/assets below `themes/<id>/`. The default and search layouts load the parser-blocking local mode adapter before their first stylesheet; the optional author stylesheet loads last, after Pagefind UI CSS on the search page. The HTML class and escaped `data-aki-mode` attribute reflect the effective Light or Dark palette, `data-aki-default-mode` records the author's default, `data-aki-mode-key` carries the escaped native preference key, and `data-aki-theme` identifies the selected theme.

The `theme-example` pair builds the Observatory with a local Dark theme. The `planets-light-example` and `planets-system-example` pairs build the same authored Planets API pages with Light and System defaults:

```sh
COBALT=/path/to/cobalt PAGEFIND=/path/to/pagefind make theme-example
COBALT=/path/to/cobalt PAGEFIND=/path/to/pagefind make theme-example-prefix
COBALT=/path/to/cobalt PAGEFIND=/path/to/pagefind make planets-light-example
COBALT=/path/to/cobalt PAGEFIND=/path/to/pagefind make planets-light-example-prefix
COBALT=/path/to/cobalt PAGEFIND=/path/to/pagefind make planets-system-example
COBALT=/path/to/cobalt PAGEFIND=/path/to/pagefind make planets-system-example-prefix
```

The Light and System prefix targets are mounted at `/planets-light` and `/planets-system`. These targets write to separately owned directories below `target/example/`; their markers prevent cleanup of an unowned output. Use Cobalt 0.20.4 and Pagefind 1.5.2, the versions checked by the project. A deployment mounted below a URL prefix must preserve that prefix when serving the generated tree.

## Updating theme support

The bundled Fumadocs CSS and fonts are pinned third-party inputs with notices in the repository. Keep their source pins, copied bytes, and notices aligned when updating them. The role list and Fumadocs projections are explicit rather than a general CSS-variable API. The local adapter owns only visitor mode preference and effective palette; theme identity remains author-selected. Remote sources, a CSS sanitizer, or font validator are not part of the local theme format.
