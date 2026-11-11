# Botanist theme family

Botanist is a named theme family with two profiles: `botanist-day` (Light) and `botanist-night` (Dark). The standalone Observatory example pairs them in Deploy mode and uses `system` as its initial mode. Authors can also choose a mixed-family pair such as Forest Dusk for Dark and Botanist Day for Light. The [theme authoring guide](../../themes/) describes catalog selection; the [design overview](../) links the rest of this reader's design guidance.

## Where the colours belong

Botanist Day keeps the article on a pale paper surface. Botanist Night uses a dark reading surface and pale text. Both profiles use the deep-green frame, navigation and inverse code/control owners; their shared family stylesheet keeps those values local to existing components. This preserves the reader geometry and its API, callback and code component boundaries.

Both profiles use Georgia with Times New Roman and the serif generic as their heading stack. Body text remains on the existing Geist stack; code remains JetBrains Mono. Sidebar navigation and inverse example-card delivery headers use pale method colours; ordinary reading-plane endpoints, schema and property content keep their declared roles in each mode. Focus and state still have text and structural cues; colour alone does not carry meaning.

| Role | Botanist Day | Botanist Night |
| --- | --- | --- |
| Frame | `#233C2E` | `#233C2E` |
| Reading surface | `#F7F5ED` | `#17201B` |
| Reading panel | `#FCFAF3` | `#1E2A23` |
| Reading text | `#2D4031` | `#DDE3D7` |
| Action | `#80551C` | `#E5CE90` |
| Focus | `#775216` | `#F0D385` |

The values describe these profiles' role maps, not a claim that every rendered pair or state has the same contrast result. The [shared specimens](../specimens/) show the common components together; [typography](../typography/) describes the reading and code roles.

## Shared callouts

These six kinds use the existing authored-Markdown callout component. Keep the label and the explanation in words so readers do not need to infer meaning from the accent.

<aside class="aki-callout" data-kind="regular"><p class="aki-callout-title">Related detail</p><p>The green frame and each reading surface have different owners. Read the <a href="../specimens/">shared specimens</a> alongside their markup.</p></aside>

<aside class="aki-callout" data-kind="info"><p class="aki-callout-title">Profile</p><p><code>botanist-day</code> and <code>botanist-night</code> share the existing <strong>Georgia</strong> heading role.</p></aside>

<aside class="aki-callout" data-kind="caution"><p class="aki-callout-title">Check the pair</p><p>Deploy needs one Dark and one Light profile. Use <code>botanist-night</code> for Dark and <code>botanist-day</code> for Light.</p></aside>

<aside class="aki-callout" data-kind="warning"><p class="aki-callout-title">Owner scope</p><p>Keep inverse roles local to their existing components. A parent value may already be computed before a nested component inherits it.</p></aside>

<aside class="aki-callout" data-kind="critical"><p class="aki-callout-title">Selection error</p><p>A profile ID that is missing from the catalog or placed in the wrong scheme slot fails generation before output is written.</p></aside>

<section class="aki-callout" data-kind="editorial" aria-labelledby="botanist-editorial-title"><h3 class="aki-callout-title" id="botanist-editorial-title">A supported boundary</h3><p>This callout component is for authored Markdown pages. API description HTML keeps its existing sanitizer and allowlist; it does not gain callout support.</p></section>

## Select the standalone pair

The standalone example uses the Botanist family for both sides of the Deploy pair:

```json
{
  "catalog": [{ "builtin": "botanist" }],
  "deploy": { "dark": "botanist-night", "light": "botanist-day" },
  "mode": "system"
}
```

Run the root and `/botanist-theme` builds with real Cobalt and Pagefind installed:

```sh
make botanist-example botanist-example-prefix
```

The generated site retains the current article and code component behavior. The [Forest guide](../forest/) describes its four profiles and Georgia heading role; a mixed-family Forest/Botanist pair remains supported when that is useful. This page's fenced JSON exercises syntax-highlighted code; ordinary inline code stays part of the current reading surface. The unhighlighted preformatted sample below has no language or syntax-token classes:

<pre><code>data-aki-variant="botanist-night"; data-aki-mode="dark"</code></pre>

<details>
  <summary>Why the dark components keep their own role block</summary>
  <p>CSS custom properties resolve where they are declared. Re-declaring the mapped roles at the existing dark owner lets each component use its intended surface without changing component geometry or moving content between its existing owners.</p>
</details>
