# Botanist Day

Botanist is a named theme family with one Light profile: `botanist-day`. It pairs a deep forest-green frame with a quiet paper reading surface. Its standalone Observatory example chooses Forest Dusk for Dark and Botanist Day for Light, so authors can see that a Deploy pair may cross family boundaries. The [theme authoring guide](../../themes/) describes catalog selection; the [design overview](../) links the rest of this reader's design guidance.

## Where the colours belong

The article body keeps the pale paper surface and its dark text. The surrounding frame carries the green role. Existing navigation and component owners that need a dark surface receive their own inverse role values: the sidebar, reference top bar, example cards, article code panels, callback code panels and callback request subheader. The adapter resolves aliases at each owner so a value from the paper body does not leak into a dark panel. It preserves the shared reader geometry and the existing API, callback and code component boundaries.

The profile uses Georgia with Times New Roman and the serif generic as its heading stack. Body text remains on the existing Geist stack; code remains JetBrains Mono. Method colours are softened only in the sidebar navigation, while method and required-property roles in API content keep their declared values. Focus and state still have text and structural cues; colour alone does not carry meaning.

| Role area | Botanist Day | Where it is used |
| --- | --- | --- |
| Frame | `#233C2E` | Reader frame around the paper article |
| Article page | `#F7F5ED` | Main reading surface |
| Article text | `#2D4031` | Body copy on paper |
| Action | `#80551C` | The existing action role |
| Inverse surface | `#233C2E` | Dark code and control component owners |
| Inverse text | `#EDF0DF` | Text within those dark owners |

The values describe this profile's role map, not a claim that every rendered pair or state has the same contrast result. The [shared specimens](../specimens/) show the common components together; [typography](../typography/) describes the reading and code roles.

## Shared callouts

These six kinds use the existing authored-Markdown callout component. Keep the label and the explanation in words so readers do not need to infer meaning from the accent.

<aside class="aki-callout" data-kind="regular"><p class="aki-callout-title">Related detail</p><p>The frame role and the paper surface have different owners. Read the <a href="../specimens/">shared specimens</a> alongside their markup.</p></aside>

<aside class="aki-callout" data-kind="info"><p class="aki-callout-title">Profile</p><p><code>botanist-day</code> is the family's single Light profile; its title uses <strong>Georgia</strong>.</p></aside>

<aside class="aki-callout" data-kind="caution"><p class="aki-callout-title">Check the pair</p><p>Deploy needs one Dark and one Light profile. Botanist Day cannot fill the Dark slot because its declared scheme is Light.</p></aside>

<aside class="aki-callout" data-kind="warning"><p class="aki-callout-title">Owner scope</p><p>Keep inverse roles local to their existing components. A parent value may already be computed before a nested component inherits it.</p></aside>

<aside class="aki-callout" data-kind="critical"><p class="aki-callout-title">Selection error</p><p>A profile ID that is missing from the catalog or placed in the wrong scheme slot fails generation before output is written.</p></aside>

<section class="aki-callout" data-kind="editorial" aria-labelledby="botanist-editorial-title"><h3 class="aki-callout-title" id="botanist-editorial-title">A supported boundary</h3><p>This callout component is for authored Markdown pages. API description HTML keeps its existing sanitizer and allowlist; it does not gain callout support.</p></section>

## Select the standalone pair

The standalone example keeps the mixed-family pair explicit:

```json
{
  "catalog": [
    { "builtin": "forest" },
    { "builtin": "botanist" }
  ],
  "deploy": { "dark": "forest-dusk", "light": "botanist-day" },
  "mode": "light"
}
```

Run the root and `/botanist-theme` builds with real Cobalt and Pagefind installed:

```sh
make botanist-example botanist-example-prefix
```

The generated site retains the current article and code component behavior. For comparison, the [Forest guide](../forest/) describes its four profiles and Georgia heading role. This page's fenced JSON exercises syntax-highlighted code; ordinary inline code stays part of the surrounding paper text. The unhighlighted preformatted sample below has no language or syntax-token classes:

<pre><code>data-aki-variant="botanist-day"; data-aki-mode="light"</code></pre>

<details>
  <summary>Why the dark components keep their own role block</summary>
  <p>CSS custom properties resolve where they are declared. Re-declaring the mapped roles at the existing dark owner lets each component use its intended surface without changing its geometry or moving the paper article into a dark palette.</p>
</details>
