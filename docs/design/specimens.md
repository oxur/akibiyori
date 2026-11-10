# Shared specimens

These examples show authored Markdown and existing reader patterns together. They are illustrative page content, not generated contract facts. For the complete generated schema, filters, examples, and code behavior, open the [Observatory tutorial API](../../../reference/) and its [read observation operation](../../../reference/read-observation/) or [ObservationEnvelope model](../../../reference/models/catalog/ObservationEnvelope/).

## Editorial structures

### Explain the task first

An observation record can be read as a sequence: identify the observation, inspect its measurement, then follow its instrument to a calibration record. Keeping that sequence in the prose helps a reader understand why each field matters.

- Start with the reader's question.
- Name the field or action that answers it.
- Link to the generated reference when the complete contract is needed.

| Step | Reader looks for |
| --- | --- |
| Identify | `observation_id` |
| Measure | `band` and `exposure_seconds` |
| Calibrate | `calibration_epoch` |

> An example is most useful when it shows both what to look at and where its limits are.

## Semantic callouts

These authored examples use the shared component. Each one has a visible kind label; colour reinforces the distinction without carrying it alone.

<aside class="aki-callout" data-kind="regular"><p class="aki-callout-title">Related detail</p><p>The <strong>observation</strong> keeps its original identifier, <code>observation_id</code>, when you compare readings. See the <a href="../../../reference/read-observation/">read operation</a> for the generated contract.</p></aside>

<aside class="aki-callout" data-kind="info"><p class="aki-callout-title">Context</p><p>Each observation includes an <strong>instrument</strong> object. Its <code>name</code> identifies the device alongside the reading; see the <a href="../../../reference/models/catalog/Instrument/">Instrument model</a>.</p></aside>

<aside class="aki-callout" data-kind="caution"><p class="aki-callout-title">Check the interval</p><p>A timestamp without its <strong>timezone</strong> does not identify one moment. Keep the offset with values such as <code>2031-04-12T20:15:00Z</code>; the <a href="../../../reference/read-observation/">read operation</a> shows the source shape.</p></aside>

<aside class="aki-callout" data-kind="warning"><p class="aki-callout-title">Reader warning</p><p>Changing the units without updating the description can make an otherwise valid <strong>exposure</strong> value misleading. The field <code>exposure_seconds</code> needs its unit. Read the <a href="../../../reference/models/catalog/ObservationEnvelope/">model description</a> with it.</p></aside>

<aside class="aki-callout" data-kind="critical"><p class="aki-callout-title">Critical consequence</p><p>If the required <strong>calibration object</strong> is missing from an instrument, the example is incomplete. Check <code>instrument.calibration</code> in the <a href="../../../reference/models/catalog/Instrument/">Instrument model</a>.</p></aside>

<section class="aki-callout" data-kind="editorial" aria-labelledby="callout-editorial-title"><h3 class="aki-callout-title" id="callout-editorial-title">Editorial note</h3><p>This grouping adds a short explanation beside related material. It does not report severity or success. See the <a href="../forest/"><strong>Forest family</strong></a> guide for the <code>editorial</code> role.</p></section>

## Code examples

The following snippets use ordinary fenced code blocks, so the existing highlighter and copy control can act on them.

```json
{
  "observation_id": "sample-42",
  "instrument": "North Ridge",
  "observed_at": "2031-04-12T20:15:00Z"
}
```

From the repository root, with Cobalt and Pagefind available on `PATH`, this builds the Observatory tutorial with a mounted path:

```sh
cargo run --locked -- build \
  --config examples/observatory/config.json \
  --output target/observatory-design-example \
  --base-path /observatory
```

## A native disclosure

<details>
  <summary>Why this sample is illustrative</summary>
  <p>The values above demonstrate page structure. Consult the linked generated Observatory reference for its actual schema and examples.</p>
</details>

## Existing reference shapes

The operation endpoint and method classes below are shared reader classes. This manual specimen does not create an operation page or make a network request.

<div class="operation-endpoint">
  <span class="method text-green-400">GET</span>
  <code>/observations/{observation_id}</code>
</div>

The schema and property classes likewise demonstrate the existing presentation roles. The required label remains text as well as a class-controlled state.

<div class="schema-card">
  <div class="schema-card-header"><code>ObservationExample</code><span>object</span></div>
  <div class="fields">
    <div class="property-card">
      <div class="property-summary"><code class="property-name">observation_id</code><span class="property-type">string</span><span class="property-requirement required">required</span></div>
      <div class="property-description"><p>Identifies the illustrative observation.</p></div>
    </div>
  </div>
</div>
