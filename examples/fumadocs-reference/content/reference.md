<p class="specimen-credit">This route pairs the Fumadocs-derived reading shell with Akibiyori’s independently authored Observatory Catalog contract. It is not a copy of Fumadocs’ Scalar API definition.</p>

# Catalog API reference

Explore the generated reference for the Observatory Catalog contract. Generated operation and model pages retain their source pointers, readable field summaries, and complete raw JSON excerpts.

## A small configuration example

The sidebar can be configured with grouped routes:

```json
{
  "sidebar_groups": [
    {
      "title": "Documentation",
      "links": [{ "label": "Quick Start", "href": "/" }]
    }
  ]
}
```

<div class="cards-grid">
  <a data-card href="/reference/operations/"><h3 class="not-prose">Operations</h3><p>Browse the available HTTP operations.</p></a>
  <a data-card href="/reference/models/"><h3 class="not-prose">Models</h3><p>Browse the contract’s named schemas.</p></a>
</div>

## Reading a reference page

<div class="fd-steps">
  <section class="fd-step"><h3>Start with the summary</h3><p>Read the method, path, operation summary, and required parameter names.</p></section>
  <section class="fd-step"><h3>Follow model references</h3><p>Use field anchors and schema links to move between the operation and its nested models.</p></section>
  <section class="fd-step"><h3>Inspect the original JSON</h3><p>Open the source details when you need to compare a concise display with the preserved contract input.</p></section>
</div>

| Page kind | What it shows | Source remains available |
| --- | --- | --- |
| Operation | Method, path, parameters, request and response details | Yes |
| Model | Named fields, constraints, and references | Yes |

<details>
  <summary>What is deliberately not inferred?</summary>
  <p>The generated pages do not execute a service, issue network requests, or treat an OpenAPI example as a live API. Raw JSON and source pointers remain the evidence when the concise view omits a feature.</p>
</details>

#### External source page

This external link has the same path as the local overview route and must not be marked as the current page: <a href="https://fumadocs.dev/reference/">Fumadocs reference</a>.
