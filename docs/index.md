# Akibiyori

Akibiyori generates Cobalt source and a static site from supplied OpenAPI contracts and authored Markdown. It reads the contract structure to produce operation and model pages; it does not validate payloads or contact the described service.

## Start with a working site

- [Install Akibiyori and its native tools](docs/install/).
- Follow the [quick start](docs/quickstart/) to build a real local site.
- Learn how to [author Markdown](docs/authoring/) and [configure a site](docs/configuration/).

## Choose a reading example

The [demo hub](demos/) keeps four public inputs separate: the Observatory tutorial catalog, Scalar Galaxy, Kubernetes API Extensions, and Swagger Petstore. The [Observatory walkthrough](docs/reading-an-observation/) follows a response through a variant to the `calibration_epoch` field.

The [Observatory tutorial API](reference/) is an example contract used to demonstrate the reader. It is not Akibiyori's library or command-line interface. [Search](search/) uses each generated Pagefind index within its own site.

## Understand the boundaries

Read the [reference reader guide](docs/reference-reader/) for supported OpenAPI and JSON Schema facts, examples, media and resource limits. The [Planets input guide](docs/planets-input/) and [public fixture notes](docs/public-fixtures/) record source provenance and the exact reader limits for those examples. [Maintenance](docs/maintenance/) covers output ownership and repeatable builds.
