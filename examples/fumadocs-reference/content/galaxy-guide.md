# Read the Galaxy contract

This guide follows relationships in the corrected Scalar Galaxy reading copy. It is a static view of the pinned contract, not a live planetary service.

## Follow one planet lookup

Open [`getPlanet`](../../reference/getPlanet/), which describes `GET /planets/{planetId}`. Its required `planetId` path parameter is an integer with `int64` format. The `200` response refers to [Planet](../../reference/models/scalar/Planet/), whose required fields are `id` and `name`.

## Follow a collection response

The [`getAllData`](../../reference/getAllData/) operation describes `GET /planets`. Its `200` response uses the generic [PaginatedResource](../../reference/models/scalar/PaginatedResource/) schema. In the corrected reading copy, the response binds the generic `itemType` to [Planet](../../reference/models/scalar/Planet/) using `$dynamicAnchor` and `$dynamicRef`. This binding appears for both `application/json` and `application/xml`; the displayed schema is a structural reading aid, not an XML encoding.

## Read the source boundaries

The original YAML remains unchanged. The separate corrected copy changes four response-reference values so the generic resource and its `Planet` item resolve for both media entries. The repository's `planets-provenance.md` records those changes. The reader shows the source media labels and schema relationships; it does not call a service or validate a returned payload.
