<p class="specimen-credit">This reading specimen is based on the Scalar Galaxy OpenAPI 3.1.1 contract from the pinned Fumadocs Scalar example. The original YAML remains unchanged. The generated reader uses a separate copy with four documented reference-value corrections.</p>

# Scalar Galaxy API reference

This reference presents the Scalar Galaxy contract's operations and named schemas. The source input is YAML; schema source sections display the parsed JSON representation, so they do not preserve YAML comments, quoting, or formatting. The original fixture and the four-reference reading derivative remain separately documented in the source repository.

Browse the [operations](operations/) or [models](models/) in this contract. It contains ten HTTP path operations, one newPlanet webhook, and fifteen named models. The webhook delivery destination is configured by its caller; this reference does not assign a URL or invoke a service.

The operation and model pages retain source locations, concise schema facts, and raw JSON details. A resolved schema reference links to its named model when one is available. The corrected reading copy changes only the four response references needed to resolve the generic `PaginatedResource` and bind its `itemType` to `Planet` for both JSON and XML.
