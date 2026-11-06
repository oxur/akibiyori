# Quick start

From a checkout with Rust, Cobalt and Pagefind installed, build the Observatory example:

```sh
COBALT=cobalt PAGEFIND=pagefind make example
```

If the tools are not on `PATH`, pass their executable paths:

```sh
COBALT=/path/to/cobalt PAGEFIND=/path/to/pagefind make example
```

The Make recipe compiles and runs Akibiyori, then invokes real Cobalt and Pagefind. It writes generated Cobalt source and the served site below `target/example/observatory-build/`. Open `target/example/observatory-build/site/index.html` in a local static file server or inspect the generated files directly. `make example-prefix` repeats the same input under `/akibiyori-test`.

The site introduces the [Observatory tutorial API](../../reference/) and its [two operations](../../reference/operations/). To follow a complete schema path, read the [walkthrough](../reading-an-observation/), open the generated [ObservationEnvelope](../../reference/models/catalog/ObservationEnvelope/) model, and follow the links to the selected observation variant and `Calibration.calibration_epoch`. The page-local [search view](../../search/) is generated from the served content.

For the four-demo assembly, run `make demo-site` or `make demo-site-prefix`; the [demo hub](../../demos/) explains the separate Observatory, Scalar Galaxy, Kubernetes API Extensions, and Swagger Petstore inputs. These commands build local static examples and make no network request to an example API.
