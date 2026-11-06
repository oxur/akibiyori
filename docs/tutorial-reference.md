# Observatory tutorial API

The Observatory catalog is a small, independently authored OpenAPI contract used to explain how Akibiyori turns a supplied document into readable pages. It is a tutorial API for this example only; it is not the Akibiyori library or CLI API.

## Follow one value through the schema

Start with [the read-observation operation](read-observation/). Its `200` response returns an `ObservationEnvelope`, whose `observation` field points to a `oneOf` containing a photometric or spectroscopic variant. Each variant composes shared fields with its own measurements. From the shared `Instrument`, continue to `Calibration.calibration_epoch`.

- Browse [all operations](operations/) or [all models](models/).
- Read the complete [observation walkthrough](../docs/reading-an-observation/).
- Open [the ObservationEnvelope model](models/catalog/ObservationEnvelope/) and follow its field links.

The contract has two operations and nine named models. Its names, values, and example behavior describe this local catalog; they make no claim about a running Observatory service. For the other independent inputs, use the [demo hub](../demos/).
