# Reader design system

This stylesheet set names the appearance roles used by the static Dark and Light Akibiyori reader modes. It is a maintainer map for the existing shell and the site-configured manifest described in [`docs/themes.md`](../docs/themes.md). It does not add a browser theme selector, System mode, or a new visual scale.

## Ownership and cascade

| File or layer | Responsibility |
| --- | --- |
| `theme/vendor/fumadocs-reference.css` | Pinned compiled Fumadocs source CSS. Keep it as a credited vendor artifact; do not edit it to change local reader roles. |
| `theme/css/akibiyori-fonts.css` and the local WOFF2 files | Licensed font faces and their local source-compatible fallbacks. Preserve the files and notices together. |
| `theme/css/akibiyori-tokens.css` | Local purpose names and their current source values. This is the normal place to change a reader role while preserving its meaning. |
| `theme/css/akibiyori.css` | The authored adapter selectors and component contexts that consume those roles. Preserve selector specificity, source order, geometry, and interaction behavior when changing a role. |
| `theme/_layouts/default.liquid` and `search.liquid` | Load fonts, pinned source CSS, local tokens, then the adapter and generated theme stylesheet. Optional trusted author CSS is last; Search UI CSS remains before it on the search layout. |

The layouts use the effective `html.dark` or `html.light` class with `body.framework`. Fumadocs supplies the `--color-fd-*` roles; the framework body supplies a yellow primary value, while the sidebar sets its own muted, secondary, and muted-foreground inputs. The same local color-role declarations are scoped to both `body.framework` and `#nd-sidebar`. Descendants and native callback dialogs opened through the body inherit the body-resolved values; the sidebar resolves its aliases from its own effective Fumadocs inputs. Derived values such as the secondary tint are computed at each owner too, so they follow the sidebar’s local secondary surface. Keep the Fumadocs roles as inputs; do not point them back to local aliases, which could create a cycle or freeze a value from the wrong scope.

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

In Dark, the `--aki-color-code-article-*` family preserves the article sample colors: foreground `#cdd6f4`, strings `#a6e3a1`, punctuation/comments `#9399b2`, keywords `#cba6f7`, names `#89b4fa`, constants `#f5c2e7`, and operators `#89dceb`. The separate `--aki-color-code-api-*` family preserves the API and callback values: foreground `#ffffff`, punctuation `#a0a0a0`, keys/numbers/options `#ffc799`, and string values `#99ffe4`; Light uses the observed githubLight values for those roles. Keep these scopes distinct even where a value happens to match another role. In Light mode, only the foreground of highlighted article code outside API panels follows the article foreground role; its existing inner code-box background, border, and geometry remain. Parsed token classes keep their specific colors, while unhighlighted raw Source JSON remains ordinary text rather than receiving syntax-token colors.

## Type roles

| Role family | Current values and consumers |
| --- | --- |
| `--aki-font-body`, `--aki-font-code` | Existing Geist and JetBrains Mono family stacks, including their current fallbacks |
| `--aki-type-nav-body-small`, `--aki-type-caption`, `--aki-type-code-field`, `--aki-type-micro` | `.875rem`, `.75rem`, `.8125rem`, `.6875rem` for navigation/body-small, captions, code/fields, and micro labels |
| `--aki-type-section-subheading`, `--aki-type-brand-label`, `--aki-type-status-feedback`, `--aki-type-icon-initial` | `.9375rem`, `.9375rem`, `.8rem`, `.7rem`; the two `.9375rem` uses remain separate roles |
| `--aki-type-page-title`, `--aki-type-section-title`, `--aki-type-callback-section` | `1.75em`, `1.5em`, `1.25em`; retain the existing em relationship to the containing context |
| `--aki-weight-regular`, `--aki-weight-book`, `--aki-weight-medium`, `--aki-weight-semibold` | Existing weights `400`, `450`, `500`, `600` |

Leading roles retain their existing contexts and values: sidebar prose `1.4`; navigation headings, webhook badges and callback description/chip text `1rem`; the Copy control `1rem`; navigation/TOC/callback rows `1.25rem`; operation title and code `1.5`; operation section heading `1.333333`; callback section heading `1.6`. The token file gives those contexts separate names where their purposes differ. The Copy control leading sizes the button only; source code text keeps its own context. `font: inherit` remains in the adapter before any existing family or size overrides; do not reorder or remove those overrides.

## Layout and component geometry roles

These roles name existing literal values at the selectors that already consume them. Values and units are intentionally not normalized. They do not own the framework's runtime layout inputs or create a second geometry system.

| Role | Value and existing consumer |
| --- | --- |
| `--aki-layout-panel-inset` | `.5rem`; `.reference-panel` margin. |
| `--aki-layout-panel-header-inset` | `.5rem`; `.panel-header` top and left. |
| `--aki-layout-panel-block-reduction` | `1rem`; literal subtraction in `.reference-panel` height, whose `--fd-layout-height` input remains live. |
| `--aki-layout-content-gutter` | `24px`; `.reference-scroll-row.operation-scroll-row` inline padding and the first `max()` operands for reference/search scroll rows. |
| `--aki-layout-composition-width` | `1064px`; subtraction in `.reference-scroll-row` centering calculation only. |
| `--aki-layout-article-width` | `760px`; `.reference-article` flex basis and `--fd-page-width` fallback; search flex basis/centering and narrow article maximum. |
| `--aki-layout-operation-width` | `1200px`; `.operation-article` maximum width. |
| `--aki-layout-toc-width` | `304px`; `.reference-toc` width and flex basis. |
| `--aki-layout-example-width` | `400px`; second desktop `.operation-columns` grid track. |
| `--aki-layout-scroll-start`, `--aki-layout-scroll-end` | `3.5rem`, `2rem`; reference and operation scroll-row block padding. |
| `--aki-layout-article-start`, `--aki-layout-article-end` | `1.5rem`, `2rem`; the existing reference and operation article padding declarations, including the narrow operation-article bottom override. Their separate declarations and cascade order remain. |
| `--aki-layout-content-gap`, `--aki-layout-operation-gap` | `1rem`, `24px`; article column gap and narrow operation/callback column gaps; desktop operation-column gap remains separate. |
| `--aki-layout-narrow-gutter`, `--aki-layout-narrow-end` | `1rem`, `3rem`; narrow reference scroll-row padding. The operation-row `1rem` override remains distinct. |
| `--aki-sidebar-collapse-gutter` | `8px`; desktop collapsed `--fd-sidebar-col` and `--fd-sidebar-width` assignments. Narrow collapsed values still override them. |
| `--aki-sidebar-mobile-max` | `19rem`; second operand of the existing narrow `min(85vw, …)` sidebar-width expressions. The `85vw` expression remains live. |
| `--aki-sidebar-header-block` | `3.5rem`; later `.sidebar-header` height override. Its earlier `--fd-header-height` rule remains unchanged. |
| `--aki-sidebar-header-inline-start`, `--aki-sidebar-header-inline-end` | `22px`, `12px`; later `.sidebar-header` inline padding, retaining pixels. |
| `--aki-sidebar-control-block` | `2.5rem`; section summary, search and section-menu row minimum heights. |
| `--aki-sidebar-nav-block` | `2.25rem`; ordinary and generated navigation row minimum heights. |
| `--aki-sidebar-row-gap`, `--aki-sidebar-row-block`, `--aki-sidebar-row-inline` | `.75rem`, `.5rem`, `.625rem`; effective sidebar controls/navigation gap and row/heading padding. |
| `--aki-sidebar-controls-inline` | `.75rem`; effective sidebar controls and viewport inline padding. The earlier overridden shorthand declarations remain unchanged. |
| `--aki-sidebar-group-separation`, `--aki-sidebar-generated-separation` | `1.25rem`, `1.5rem`; ordinary sidebar group separation and generated navigation group margin, respectively. |
| `--aki-sidebar-popover-width-reduction` | `1.5rem`; literal subtraction from live `--fd-sidebar-width` in `.sidebar-section-menu`. |
| `--aki-control-icon-button`, `--aki-control-icon-padding`, `--aki-control-icon-glyph` | `2rem`, `.375rem`, `1.125rem`; collapse/reopen and callback-close dimensions, padding and SVG dimensions. |
| `--aki-brand-icon-size` | `20px`; brand image width, height and flex basis. |
| `--aki-endpoint-block`, `--aki-endpoint-gap`, `--aki-endpoint-padding-block`, `--aki-endpoint-padding-inline` | `44px`, `12px`, `6px`, `14px`; `.operation-endpoint` minimum height, gap and padding. |
| `--aki-schema-header-block`, `--aki-schema-control-block`, `--aki-schema-filter-inline` | `36px`, `28px`, `160px`; schema/auth/media headers, input/select controls, filter widths and bases, including nested and narrow overrides. |
| `--aki-schema-body-block`, `--aki-schema-body-inline` | `.625rem`, `.75rem`; property-card block padding and existing schema/auth/header/body inline padding. |
| `--aki-schema-summary-gap`, `--aki-schema-property-spacing`, `--aki-schema-chip-gap` | `.5rem`, `.25rem`, `.375rem`; property summaries and description groups; property-description/chip top spacing and body paragraph margins; chip-list gap. Existing `.45rem` and `.5rem` exceptions stay distinct. |
| `--aki-field-permalink-size`, `--aki-field-permalink-padding` | `22px`, `4px`; field permalink dimensions and padding. The `-4px` block margin remains signed and local. |
| `--aki-example-header-block`, `--aki-example-variant-max`, `--aki-example-language-inline` | `40px`, `9rem`, `5rem`; example header height, variant-control maximum, and language-control width/flex basis. |
| `--aki-example-copy-size`, `--aki-example-copy-glyph` | `28px`, `14px`; example copy control and glyph dimensions. |
| `--aki-example-code-max-block`, `--aki-example-code-padding-block`, `--aki-example-code-padding-inline` | `600px`, `.875rem`, `1rem`; example `pre` maximum height and code padding. |
| `--aki-example-rail-inset`, `--aki-example-rail-block-reduction` | `1rem`, `2rem`; sticky example-rail top and literal reduction in its maximum-height calculation. `--fd-layout-height` remains live. |
| `--aki-toc-padding-start`, `--aki-toc-padding-inline-start`, `--aki-toc-padding-end`, `--aki-toc-block-reduction` | `2rem`, `4rem`, `1rem`, `1rem`; `.reference-toc` padding and only the literal operand in its live layout-height calculation. JS-owned `--toc-active-top` and `--toc-active-height` remain untouched. |
| `--aki-search-field-max-inline`, `--aki-search-placeholder-min-block` | `42rem`, `8rem`; Pagefind search input `min()` and `.search-ui` minimum height. Pagefind's native bundle is unchanged. |
| `--aki-copy-code-inset`, `--aki-copy-code-padding-block`, `--aki-copy-code-padding-inline` | `.5rem`, `.3rem`, `.5rem`; generic `.copy-code` inset and padding. Together with `--aki-leading-code-copy` and `--aki-border-default`, these roles reserve the owned PRE toolbar band so source text stays below the control. |
| `--aki-callback-dialog-max-inline`, `--aki-callback-dialog-max-block`, `--aki-callback-dialog-viewport-reduction` | `64rem`, `52rem`, `3rem`; native callback-dialog `min()` bounds and viewport-calculation operands; viewport units remain live. |
| `--aki-callback-dialog-header-block`, `--aki-callback-dialog-header-inline-start`, `--aki-callback-dialog-header-inline-end` | `3rem`, `1.25rem`, `.5rem`; callback dialog header flex basis and padding. The narrow end padding uses the existing end role; its deliberate `1rem` start override remains separate. |
| `--aki-callback-body-inset`, `--aki-callback-endpoint-block`, `--aki-callback-content-inset`, `--aki-callback-section-separation` | `1.25rem`, `2.75rem`, `1rem`, `2.5rem`; callback dialog body, endpoint, inline fallback content and later callback section spacing. Earlier first-section zero override remains. |
| `--aki-callback-delivery-block`, `--aki-callback-delivery-inline`, `--aki-callback-request-header-block` | `.75rem`, `1rem`, `2.25rem`; delivery context padding and callback request subheader height, including the media-controls minimum. These retain their own purposes and units. |
| `--aki-border-default` | `1px`; authored solid borders only. Zero-border resets are unchanged. |
| `--aki-focus-outline-width`, `--aki-focus-outline-offset` | `2px`, `2px`; existing positive focus outlines/offsets. The callback trigger's negative `-2px` offset stays literal and distinct. |
| `--aki-radius-panel`, `--aki-radius-navigation`, `--aki-radius-selector-menu` | `1rem`, `.5rem`, `.75rem`; reference panel, navigation/link/TOC/skip-link controls including the narrow sidebar toggle, and sidebar menus/search/callback list. |
| `--aki-radius-endpoint`, `--aki-radius-example-card`, `--aki-radius-schema-card` | `12px`, `12px`, `.625rem`; operation endpoint, example card, and schema/auth/media cards plus origin popover. Equal `12px` values remain distinct roles. |
| `--aki-radius-small-control`, `--aki-radius-schema-chip`, `--aki-radius-pill`, `--aki-radius-copy-code`, `--aki-radius-callback-dialog` | `.375rem`, `.3rem`, `999px`, `.4rem`, `1rem`; inputs/selects/icon buttons, schema chips, auth/origin badges, ordinary code-copy button and callback dialog. The narrow dialog's zero radius remains an override. |

## Retained geometry and source exceptions

The variables above are plain literal aliases with no registration or runtime API. Framework-owned `--fd-sidebar-col`, `--fd-sidebar-width`, `--fd-layout-height`, `--fd-header-height`, and `--fd-page-width`, plus JS-owned `--toc-active-top` and `--toc-active-height`, stay at their existing owners. Consumers retain their `calc()`, `min()`, `max()`, fallback and viewport relationships; only mapped literal operands changed. Narrow breakpoint overrides continue to win.

Earlier defaults intentionally overridden later remain in the stylesheet in their original order. Other unmapped microspacing stays local: breadcrumb and paragraph margins, heading spacing, badge padding, sidebar microspacing, callback trigger/endpoint details, icon16px versus `1rem` contexts, skip-link position, dialog shadow dimensions, focus negative offsets, decoration, motion, z-index, state opacity, percentages, intrinsic sizes, and grid/flex/order relationships. Media/container thresholds and the native component behavior are unchanged. This map names the required high-value shared literals; it does not claim that every local number is a public theme control.

## Code-copy frame

`--aki-leading-code-copy` is `1rem` for the existing generic Copy button. The adapter applies `pre:has(> .copy-code)` only when a PRE directly contains the button, giving that PRE its own positioning/scroll frame and reserving the control band from the existing inset, padding and border roles. This requires a browser with the CSS `:has()` selector. The unchanged reader script excludes dedicated `[data-code-panel]` examples; generic source disclosures, including those within API pages, use the band. Callback content moves into a body-level dialog, so the PRE owns its frame even outside `.reference-article`. This intentionally changes generic PRE geometry and establishes a new baseline; it is not the previous StageA PRE layout.

## Change and credit boundaries

When changing an appearance value, trace its alias owner in `akibiyori-tokens.css`, its actual adapter consumers, and any inherited or portaled consumer before editing. Preserve the current dark computed appearance, units, fallbacks, cascade, focus behavior, wrapping, geometry, and component-specific code roles. A token name or source diff alone is not evidence of visual equivalence; use the native generator and the recorded same-engine comparison protocol for changes that affect rendered values.

The compiled Fumadocs CSS, fonts, icons, and adapted snippets retain their source pins and license notices in the repository's provenance and third-party notice files. Keep those notices aligned if an upstream asset or source projection changes. Dark and Light are build-time modes. System preference selection, runtime switching, a browser theme selector, or a broader token API requires separate design and verification.

## Theme colour roles

The optional version-1 local theme manifest accepts exactly these 40 Akibiyori colour roles. Each declaration is applied at both `body.framework` and `#nd-sidebar` under the escaped theme identity and effective mode attributes. Only the twelve mapped roles also set their Fumadocs input; the other roles affect their Akibiyori consumers only. Unspecified roles keep the inherited or locally derived value.

| Akibiyori role | Fumadocs input |
| --- | --- |
| `--aki-color-text` | `--color-fd-foreground` |
| `--aki-color-text-muted` | `--color-fd-muted-foreground` |
| `--aki-color-surface-page` | `--color-fd-background` |
| `--aki-color-surface-panel` | `--color-fd-card` |
| `--aki-color-text-panel` | `--color-fd-card-foreground` |
| `--aki-color-surface-raised` | `--color-fd-popover` |
| `--aki-color-surface-secondary` | `--color-fd-secondary` |
| `--aki-color-border` | `--color-fd-border` |
| `--aki-color-focus-ring` | `--color-fd-ring` |
| `--aki-color-action` | `--color-fd-primary` |
| `--aki-color-hover` | `--color-fd-accent` |
| `--aki-color-text-hover` | `--color-fd-accent-foreground` |
| `--aki-color-method-get` | — |
| `--aki-color-method-post` | — |
| `--aki-color-method-put` | — |
| `--aki-color-method-patch` | — |
| `--aki-color-method-delete` | — |
| `--aki-color-property-required` | — |
| `--aki-color-action-tint` | — |
| `--aki-color-hover-tint` | — |
| `--aki-color-secondary-tint` | — |
| `--aki-color-toc-active-tint` | — |
| `--aki-color-overlay-menu-shadow` | — |
| `--aki-color-overlay-sidebar-shadow` | — |
| `--aki-color-overlay-dialog-shadow` | — |
| `--aki-color-overlay-dialog-backdrop` | — |
| `--aki-color-code-article-foreground` | — |
| `--aki-color-code-article-string` | — |
| `--aki-color-code-article-punctuation` | — |
| `--aki-color-code-article-keyword` | — |
| `--aki-color-code-article-name` | — |
| `--aki-color-code-article-constant` | — |
| `--aki-color-code-article-operator` | — |
| `--aki-color-code-api-foreground` | — |
| `--aki-color-code-api-punctuation` | — |
| `--aki-color-code-api-key-number-option` | — |
| `--aki-color-code-api-string-value` | — |
| `--aki-color-surface-frame` | — |
| `--aki-color-code-api-command-option` | — |
| `--aki-color-code-api-delimiter` | — |

Every authored color value uses a validated short or full hexadecimal form. In Dark, the three added roles alias the existing panel, API key/number-option, and punctuation roles at each owner. In Light, frame aliases the Light background, while API command/options and section delimiters use the curated Light roles. Dark author overrides therefore continue through the new aliases; Light overrides apply only to the selected Light mode. The manifest compiler does not rewrite the pinned vendor CSS or flatten sidebar-local defaults. Owner-local computed values such as color mixes remain attached to their original input unless the manifest explicitly names that derived role. Author stylesheet bytes are intentionally outside this role map; they are trusted CSS supplied by the site owner and copied unchanged.


## Static Light source mapping

Light applies the observed Fumadocs values for background, foreground, muted, popover, card, border, secondary, accent, ring, and framework primary inputs at both local owners. The opaque sidebar frame uses the Light background while cards retain the distinct card surface. Article syntax roles use the pinned Latte values; API examples use the pinned githubLight values. API commas and colons remain red punctuation; the more specific section-delimiter role is blue. Actual shell command/options use the purple command-option role. This native highlighter mapping is an explicit class adaptation: for example, native `-X POST` may receive the option color even though the upstream Shiki sample leaves that text foreground-colored. It does not claim exact grammar or Shiki-token parity.

The method-label roles use existing Tailwind 600 colors in Light and preserve the 400 colors in Dark. Required markers use the declared red 600 source-family adaptation. These choices do not establish a general WCAG contrast guarantee. The amber selected state and some small-text syntax pairs fail or remain untested; preserve the recorded pair-by-pair limits rather than describing every Light state as passing. See the [built-in Light author guidance](../docs/themes.md#built-in-light), the [native Fumadocs Light inputs](../src/theme.rs), the [local role values](css/akibiyori-tokens.css), and [vendor provenance](vendor/PROVENANCE.md).

The native Pagefind search input uses its own stylesheet, including a low-opacity placeholder. In Light, the adapter gives the placeholder the existing muted-text role at full opacity; Dark keeps the native widget styling. The opaque muted value over the Light background is approximately 4.349:1, below a 4.5:1 small-text target, so this local readability adjustment is not a general contrast pass.
