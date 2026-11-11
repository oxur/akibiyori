# Forest theme family

Forest is a four-profile reader theme for documentation sites. The included Observatory configuration is a runnable example, not a limit on where the family can be used. Each profile keeps the shared page geometry, Geist body text, and JetBrains Mono code; Forest headings use Georgia with Times New Roman and the serif generic fallback.

Forest Light uses a sage tint for the sidebar and surrounding frame, with a separate pale reading surface. Navigation method labels use darker local colours for readability while the API content keeps its existing palette.

<dl>
  <dt><strong>Forest Light</strong> (<code>forest-light</code>, Light)</dt>
  <dd>A clear, pale reading surface.</dd>
  <dt><strong>Forest Dusk</strong> (<code>forest-dusk</code>, Dark)</dt>
  <dd>A warm, muted dark surface.</dd>
  <dt><strong>Forest Storm</strong> (<code>forest-storm</code>, Dark)</dt>
  <dd>A neutral green-gray dark surface.</dd>
  <dt><strong>Forest Midnight</strong> (<code>forest-midnight</code>, Dark)</dt>
  <dd>A cool blue-gray dark surface.</dd>
</dl>

The central showcase opens on Maple & Mist Light. Its Deploy pair remains Ember & Ink Dark and Maple & Mist Light, with System mode selecting the light profile when the operating system requests it. Forest is an additional family: standalone authors can choose any valid dark and light variants, including a mixed-family pair. The runnable standalone example selects Forest Storm (Dark) and Forest Light for Deploy/System.

## Shared callouts

The six callout kinds name different reading roles. Regular adds a related detail; info supplies context; caution identifies a condition to check; warning explains a consequence; critical describes a consequential failure; editorial groups explanatory material without indicating severity or success. Each specimen includes a text label so meaning does not depend on colour.

The component is for authored Markdown pages and uses the shared reader adapter with ordinary authored HTML. It does not add callout support to API descriptions, which retain their existing sanitizer rules. Its default foreground and accent follow the local text role. Forest supplies 18 family aliases alongside the unchanged 40-role manifest API. Alias values are resolved independently at the article body and sidebar owners. The default surface is a restrained mix of its mapped role and the panel surface. Links remain underlined and opaque in hover states; inline code follows its immediate text colour, while block code retains its own syntax panel.

For example, an info callout can put a short explanation beside a generated field link, while an editorial callout can group a naming note and inline token. Use the visible label to name the kind and keep the text useful without its background. The [shared specimens](../specimens/) show all six authored structures together.

<aside class="aki-callout" data-kind="info"><p class="aki-callout-title">Context</p><p>Forest changes the heading role and adds semantic callout aliases. The <a href="../../themes/">theme authoring guide</a> describes the catalog selection contract.</p></aside>

<aside class="aki-callout" data-kind="caution"><p class="aki-callout-title">Keep the label</p><p>Describe the condition in words as well as using the callout's accent.</p></aside>

<section class="aki-callout" data-kind="editorial" aria-labelledby="forest-editorial-title"><h3 class="aki-callout-title" id="forest-editorial-title">Editorial grouping</h3><p>Group explanatory notes without implying that the subject is urgent or successful.</p></section>

## Build the standalone example

From the repository root, with the supported Cobalt and Pagefind executables available:

```sh
make forest-example forest-example-prefix
```

The prefix build is mounted at `/forest-theme`. Both recipes use the existing owned-output markers. The theme is selected through `theme.catalog` and `theme.deploy`; a Forest catalog family is not a legacy `theme.source` manifest.

See [shared specimens](../specimens/), [typography](../typography/), the [design overview](../), and [theme authoring](../../themes/) for the component, type role, and selection mechanics.
