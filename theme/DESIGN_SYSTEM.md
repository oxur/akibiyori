# Reader design system

This stylesheet set names the appearance roles used by the current dark Akibiyori reader. It is a maintainer map for the existing shell; it does not add a user-selectable theme, a public configuration API, or a new visual scale.

## Ownership and cascade

| File or layer | Responsibility |
| --- | --- |
| `theme/vendor/fumadocs-reference.css` | Pinned compiled Fumadocs source CSS. Keep it as a credited vendor artifact; do not edit it to change local reader roles. |
| `theme/css/akibiyori-fonts.css` and the local WOFF2 files | Licensed font faces and their local source-compatible fallbacks. Preserve the files and notices together. |
| `theme/css/akibiyori-tokens.css` | Local purpose names and their current source values. This is the normal place to change a reader role while preserving its meaning. |
| `theme/css/akibiyori.css` | The authored adapter selectors and component contexts that consume those roles. Preserve selector specificity, source order, geometry, and interaction behavior when changing a role. |
| `theme/_layouts/default.liquid` and `search.liquid` | Load fonts, pinned source CSS, local tokens, then the adapter. Search UI CSS remains after the adapter on the search layout. |

The current layouts use `html.dark` and `body.framework`. Fumadocs supplies the `--color-fd-*` roles; the framework body supplies a yellow primary value. Local color aliases are therefore defined on `body.framework`, so descendants and native callback dialogs opened through the body inherit the same computed values. The vendor role names remain inputs; do not point them back to local aliases, which could create a cycle or freeze a different primary value.

The existing `--font-sans` and `--font-mono` variables remain compatibility facades for shared framework utilities. They alias `--aki-font-body` and `--aki-font-code`, respectively. Local adapter declarations use the purpose-named font roles directly. Layout dimensions, Pagefind variables, and TOC state properties retain their existing interfaces; this token set does not rename or own them.

## Color roles

| Local role | Current source value | Main consumers |
| --- | --- | --- |
| `--aki-color-text`, `--aki-color-text-muted` | Fumadocs foreground and muted foreground | Page text, navigation, labels, status and explanatory text |
| `--aki-color-surface-page`, `--aki-color-surface-panel`, `--aki-color-text-panel` | Fumadocs background, card and card foreground | Main reading surface, cards, body and sidebar |
| `--aki-color-surface-raised`, `--aki-color-surface-secondary`, `--aki-color-border` | Fumadocs popover (falling back to card), secondary and border | Menus, selectors, cards and separators |
| `--aki-color-focus-ring` | Fumadocs ring | Visible keyboard focus |
| `--aki-color-action`, `--aki-color-hover`, `--aki-color-text-hover` | Fumadocs primary, accent and accent foreground | Current links, focus/action affordances and hover states |
| `--aki-color-method-get/post/put/patch/delete` | Existing Tailwind green/blue/yellow/orange/red 400 roles | HTTP method labels; each method keeps its existing distinct color |
| `--aki-color-property-required` | Existing Tailwind red 400 role | Required property marker; remains distinct from the delete-method role |
| `--aki-color-action-tint`, `--aki-color-hover-tint`, `--aki-color-secondary-tint`, `--aki-color-toc-active-tint` | Existing `color-mix` inputs and percentages | Current navigation, controls, search and TOC highlights |
| `--aki-color-overlay-menu-shadow`, `--aki-color-overlay-sidebar-shadow`, `--aki-color-overlay-dialog-shadow`, `--aki-color-overlay-dialog-backdrop` | Existing black overlays at 24%, 30%, 35% and 40% | Menus, mobile rail, dialog shadow and dialog backdrop |

The `--aki-color-code-article-*` family preserves the article sample colors: foreground `#cdd6f4`, strings `#a6e3a1`, punctuation/comments `#9399b2`, keywords `#cba6f7`, names `#89b4fa`, constants `#f5c2e7`, and operators `#89dceb`. The separate `--aki-color-code-api-*` family preserves API and callback examples: foreground `#ffffff`, punctuation `#a0a0a0`, keys/numbers/options `#ffc799`, and string values `#99ffe4`. Keep these scopes distinct even where a value happens to match another role.

## Type roles

| Role family | Current values and consumers |
| --- | --- |
| `--aki-font-body`, `--aki-font-code` | Existing Geist and JetBrains Mono family stacks, including their current fallbacks |
| `--aki-type-nav-body-small`, `--aki-type-caption`, `--aki-type-code-field`, `--aki-type-micro` | `.875rem`, `.75rem`, `.8125rem`, `.6875rem` for navigation/body-small, captions, code/fields, and micro labels |
| `--aki-type-section-subheading`, `--aki-type-brand-label`, `--aki-type-status-feedback`, `--aki-type-icon-initial` | `.9375rem`, `.9375rem`, `.8rem`, `.7rem`; the two `.9375rem` uses remain separate roles |
| `--aki-type-page-title`, `--aki-type-section-title`, `--aki-type-callback-section` | `1.75em`, `1.5em`, `1.25em`; retain the existing em relationship to the containing context |
| `--aki-weight-regular`, `--aki-weight-book`, `--aki-weight-medium`, `--aki-weight-semibold` | Existing weights `400`, `450`, `500`, `600` |

Leading roles retain their existing contexts and values: sidebar prose `1.4`; navigation headings, webhook badges and callback description/chip text `1rem`; navigation/TOC/callback rows `1.25rem`; operation title and code `1.5`; operation section heading `1.333333`; callback section heading `1.6`. The token file gives those contexts separate names where their purposes differ. `font: inherit` remains in the adapter before any existing family or size overrides; do not reorder or remove those overrides.

## Change and credit boundaries

When changing an appearance value, trace its alias owner in `akibiyori-tokens.css`, its actual adapter consumers, and any inherited or portaled consumer before editing. Preserve the current dark computed appearance, units, fallbacks, cascade, focus behavior, wrapping, geometry, and component-specific code roles. A token name or source diff alone is not evidence of visual equivalence; use the native generator and the recorded same-engine comparison protocol for changes that affect rendered values.

The compiled Fumadocs CSS, fonts, icons, and adapted snippets retain their source pins and license notices in the repository's provenance and third-party notice files. Keep those notices aligned if an upstream asset or source projection changes. The current reader is a dark-mode contract only; adding a light mode, user theme selection, or a broader token API requires a separate design and verification decision.
