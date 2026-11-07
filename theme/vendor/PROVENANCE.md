# Reference theme provenance

The reference fixture is based on the public Fumadocs source at commit `0cd7bd4a0b7a1fa911e61bbce1242695b1c9e9ef`. This package keeps a small native-Cobalt shell and the selected compiled reading stylesheet; it does not include the Fumadocs application, React code, Next.js runtime, or upstream scripts.

## Compiled reading CSS

The deployed source file is `3xiw_38akfqyl.css` from the Fumadocs docs deployment. Its SHA256 is `fa1588367414ad9c098ba5524159b0a0229db477d83db6b66c8104b6934521bf` (229108 bytes). Public `fumadocs-reference.css` is 227077 bytes, SHA256 `479f5f76e838eed60ff1e2277803399fd2c0c778e4122b04c8daebf14de25e69`.

The public projection removes exactly the 2031-byte interval `[218316,220347)` between the unique `.fumari-marquee{` and `.dark{--color-landing-foreground:` boundaries. The excluded interval contains the independent sponsor marquee's `.fumari-marquee*` / `.fumari-sponsors-*` selectors, its two `fumari-marquee` keyframes, and reduced-motion rules. Every byte before and after that interval is unchanged. The sponsor interface is not part of the selected docs reading UI. The source stylesheet contains no remaining `@import` or `url()` dependencies.

The local Cobalt shell follows the selected production DOM landmarks and source declarations. Its sidebar includes the source-sized section selector, search row, scroll viewport, grouped links, and an 8px desktop collapsed gutter; the selector changes its label and icon to the longest matching internal route prefix and jumps to that section's page.

- `#fd-spacious-layout`: sidebar/header/right grid with a 268px desktop rail and a 760px page measure;
- `#nd-page-panel`: 8px desktop inset, rounded dark page surface and border;
- `#nd-page`: one article body, 760px maximum prose measure;
- `#nd-toc`: right contents rail, 304px wide in the matched desktop shell including its 64px source left inset; it is hidden when the reading container is too narrow.

The Dark shell uses the source `html.dark` token set. The native static Light shell uses observed Fumadocs Light owner inputs on `body.framework` and `#nd-sidebar`; its opaque sidebar frame uses the background role while cards keep their separate card role. The article prose, card (`[data-card]`), callout, steps (`.fd-steps` / `.fd-step`), code, and content table structures map the selected Fumadocs source components to ordinary Markdown and trusted authored HTML. `.not-prose` headings are excluded from the generated contents list, following the source MDX semantics.

## Syntax color roles

The selected source code samples were queried from the delivered Quick Start HTML. Its Dark Shiki span values include foreground `#cdd6f4`, blue `#89b4fa`, green `#a6e3a1`, gray `#9399b2`, mauve `#cba6f7`, pink `#f5c2e7`, and cyan `#89dceb`. Actual native Cobalt 0.20.4 JSON and Bash outputs use `highlighter-syntect` spans. The local adapter maps observed `c-source`/`c-meta` to the foreground role, `c-string` to green, `c-punctuation` and `c-comment` to gray, `c-keyword`/`c-storage` to mauve, `c-variable`/`c-function`/`c-type` to blue, constants/numbers to pink, and operators to cyan. Source code remains unchanged; unsupported language labels are not assigned a guessed grammar.

The static Light palette uses the observed Fumadocs background, foreground, muted, popover, card, border, secondary, accent, ring and framework-primary values recorded by the native theme contract. Root neutral primary `#171717` is source provenance; the framework accent role is `#cc8b00`. The local opaque sidebar uses background `#f5f5f5`, while card surfaces remain `#f1f1f1`. Article syntax values follow the observed Latte palette; API values follow githubLight. The native syntax-class adaptation gives shell commands/options a separate purple role and section delimiters a blue role, while generic commas and colons stay red punctuation. This differs in bounded cases from upstream Shiki tokenization and is not a claim of exact grammar parity.

The initial palette preserves the declared source values rather than claiming every small-text/state pair passes contrast. Amber selected text/tint and some Latte roles have failed or untested pairs; no blanket accessibility or contrast guarantee is made. The official prose currently served by Fumadocs is Astro-based; the retained older Node specimen is not treated as the live deployment. Matching CSS assets do not establish live deployment identity.

The main source tree is [Fumadocs](https://github.com/fuma-nama/fumadocs/tree/0cd7bd4a0b7a1fa911e61bbce1242695b1c9e9ef), especially `apps/docs/app/global.css`, the base-ui `preset.css` and `lib/{base,shiki}.css`, and `packages/base-ui/src/layouts/spacious` / `components/{card,callout,steps}.tsx`. `THIRD_PARTY_NOTICES.md` points to the retained license texts.

## Fonts

The exact source font stylesheet is `reference-fonts-source.css` (3640 bytes, SHA256 `3b2a6f1a7960ab41a2ce494311a127030e615d158adb8ae260872be5ff20b8fa`). The served `theme/css/akibiyori-fonts.css` is a clearly labeled Latin-only projection. It keeps the pinned source fallback metric overrides and references the included local fonts only; regional subset URLs that are not included are omitted.

- Geist Latin WOFF2: 29288 bytes, SHA256 `9b6f5ff45b278c744b5f379a2c4ecbaf858a842b8eaf82ac8d21b699ca16c608`.
- JetBrains Mono Latin WOFF2: 40480 bytes, SHA256 `1e06740a02a443fb7f3eeda8fcaa685a0f6c620e3f01e6666e847295469ce3ad`.

Both fonts' full SIL Open Font License texts are retained in `licenses/`.

## Icons

The shell and specimen use selected inline Lucide SVGs for search, copy, table-of-contents, cards, callouts, section selection, navigation groups, and rail collapse/reopen, adapted from the pinned Fumadocs DOM. The navigation icons are `rocket`, `circle-help`, and `book-open`; the controls use `chevrons-up-down` and `panel-left`. The standalone local search asset `icons/search.svg` is 275 bytes, SHA256 `283d371c2e433817bb9c0c8310caa6c77fa4177c0f4f1168d9c83b97af7389dc`. The icon source pin and full dual-license text are in `THIRD_PARTY_NOTICES.md` and `licenses/lucide-dual-license.txt`.

## Deliberate omissions

The optional imagezoom and KaTeX stylesheet chunks, sponsor interface, standalone icon asset files beyond the local search icon, all upstream runtime/application JavaScript, remote fonts, and non-Latin font subsets are not shipped. Selected Lucide SVG markup is inline in the shell and specimen. No build or browser request depends on the omitted assets. The selected CSS still contains dormant selectors emitted by the shared upstream stylesheet; the fixture content does not instantiate those unrelated components.
