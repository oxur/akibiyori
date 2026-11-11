# Pacific theme family

Pacific is a two-profile theme for documentation readers: `pacific-day` uses Light mode and `pacific-night` uses Dark mode. It adapts a warm paper-and-oxide palette to the existing Akibiyori reader, keeping its API, navigation and code component owners. The [theme authoring guide](../../themes/) explains the catalog and Deploy pair; the [design overview](../) links the reader guidance.

## Palette and type roles

Both profiles use a quiet paper surface in Day and a deep brown reading surface in Night. A separate frame colour carries through the gutters around the reader. Method and syntax roles retain their separate meanings.

| Role | Pacific Day | Pacific Night |
| --- | --- | --- |
| Frame | `#EDE3D6` | `#0C0704` |
| Reading surface | `#F9F4EE` | `#1D140D` |
| Reading panel | `#FEFBF8` | `#291C14` |
| Reading text | `#221812` | `#E4DDD3` |
| Muted text | `#625347` | `#B9AC9F` |
| Action | `#8A3819` | `#E7AA5B` |
| Focus | `#8A3819` | `#F2B76A` |

Navigation, headings and controls use Red Hat Display with sans-serif fallbacks. Narrative paragraphs use bundled Newsreader at 18px with 1.65 leading; code and paths use IBM Plex Mono. Newsreader's variable optical size follows the rendered text size. System fallbacks stay in each complete stack.

The seven TTF files are embedded from the pinned Google Fonts source. The full SIL Open Font License notices are emitted under `themes/pacific/notices/` when Pacific is selected. Google Fonts and Haleiki input provenance remains in the public family source files. The family stylesheet adapts five pinned Haleiki mockups as a token source; the public source hashes and MIT notice are retained with the family. It does not copy those pages' layouts or prose. The font files add about 1.6 MB before serving compression. The palette is not a claim that every rendered state meets an accessibility target; check actual text, focus and control pairs in the target reader.

## Reader specimens

These labelled callouts use the existing authored-Markdown component. Their accent hues are visual cues; each meaning remains in its title and text.

<aside class="aki-callout" data-kind="regular"><p class="aki-callout-title">Reading surface</p><p>Pacific keeps the paper, frame and raised panel as distinct surfaces. Compare the <a href="../specimens/">shared specimens</a> with the current reader owners.</p></aside>

<aside class="aki-callout" data-kind="info"><p class="aki-callout-title">Two profiles</p><p>Choose <code>pacific-day</code> for Light or <code>pacific-night</code> for Dark.</p></aside>

<aside class="aki-callout" data-kind="caution"><p class="aki-callout-title">Pair schemes</p><p>Deploy needs one Dark and one Light profile. Keep Night in the Dark slot and Day in the Light slot.</p></aside>

<aside class="aki-callout" data-kind="warning"><p class="aki-callout-title">Local ownership</p><p>The frame, prose, controls and code retain separate owners. Changing a page alias does not recolour a nested component that resolves its own value.</p></aside>

<aside class="aki-callout" data-kind="critical"><p class="aki-callout-title">Invalid selection</p><p>A missing profile or a Light profile in the Dark slot fails generation before output changes.</p></aside>

<section class="aki-callout" data-kind="editorial" aria-labelledby="pacific-editorial-title"><h3 class="aki-callout-title" id="pacific-editorial-title">A bounded adaptation</h3><p>The source mockups informed token choices. Their site structure, taxonomy and trust indicators are not part of this reader theme.</p></section>

The existing role table keeps its column associations when the screen narrows:

| Profile | Scheme | Prose face | Code face |
| --- | --- | --- | --- |
| `pacific-day` | Light | Newsreader | IBM Plex Mono |
| `pacific-night` | Dark | Newsreader | IBM Plex Mono |

Prose links remain underlined, and inline code keeps its separate monospace face: read the [theme authoring guide](../../themes/) before selecting a family for a site.

```json
{
  "catalog": [{ "builtin": "pacific" }],
  "deploy": { "dark": "pacific-night", "light": "pacific-day" },
  "mode": "system"
}
```

The sample below is plain preformatted text without syntax-token classes:

<pre><code>data-aki-variant="pacific-night"; data-aki-mode="dark"</code></pre>

The fenced block below receives the reader's syntax highlighting:

```rust
fn selected_pair() -> (&'static str, &'static str) {
    ("pacific-night", "pacific-day")
}
```

## Build the standalone pair

The Observatory config is `examples/observatory/config-theme-pacific.json`. It selects Pacific Night and Day in Deploy with System as the initial visitor preference. Build both the root and `/pacific-theme` versions with the real Cobalt and Pagefind tools:

```sh
make pacific-example pacific-example-prefix
```

For a composed catalog, Pacific Day and Night follow Botanist Day and Night in the picker. The composed Demo still opens on Maple & Mist Light, and the existing Deploy pair remains Ember & Ink Dark with Maple & Mist Light. The `pacific` family is a named catalog selection; it is not a legacy `theme.source` manifest.
