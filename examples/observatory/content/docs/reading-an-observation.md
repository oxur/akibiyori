# Reading an observation

This walkthrough follows the catalog's documented response from an observation identifier to a timestamp on its calibration record. All names and values belong to this illustrative example.

## Start with the operation and response envelope

The **GET** operation [read-observation](../../reference/read-observation/) accepts a required UUID `observation_id`. Its `200` response links to [ObservationEnvelope](../../reference/models/catalog/ObservationEnvelope/), which contains the `observation` value and a UUID request identifier.

## Read the observation alternatives

The envelope's `observation` field links to [Observation](../../reference/models/catalog/Observation/). Its `oneOf` lists [PhotometricObservation](../../reference/models/catalog/PhotometricObservation/) and [SpectroscopicObservation](../../reference/models/catalog/SpectroscopicObservation/); exactly one alternative applies. Each variant uses `allOf` to compose the shared [ObservationBase](../../reference/models/catalog/ObservationBase/) fields with variant measurements.

For a photometric entry, `band` is one of `u`, `g`, or `r`, and `exposure_seconds` has a minimum of zero. The shared base includes the observation identifier, observation time, and a link to [Instrument](../../reference/models/catalog/Instrument/).

## Follow the calibration timestamp

The instrument links to [Calibration](../../reference/models/catalog/Calibration/), whose `calibration_epoch` is a date-time field. This gives a concrete field path from the response envelope through a selected observation variant and its shared instrument to the calibration timestamp.

## Other schema shapes in this catalog

[ScalarValue](../../reference/models/catalog/ScalarValue/) uses `anyOf` for four titled primitive alternatives. [ObservationGroup](../../reference/models/catalog/ObservationGroup/) contains a recursive child list and a string-valued labels map. Their local links and field constraints remain visible on their generated model pages.
