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

## Code examples

The following snippets use ordinary fenced code blocks, so the existing highlighter and copy control can act on them.

```json
{
  "observation_id": "sample-42",
  "instrument": "North Ridge",
  "observed_at": "2031-04-12T20:15:00Z"
}
```

```sh
akibiyori build --base-path /observatory
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
