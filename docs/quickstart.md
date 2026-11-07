# Quick start

## Build Akibiyori's project docs

From the repository root, build the first-party documentation with the real Cobalt and Pagefind executables:

```sh
COBALT=cobalt PAGEFIND=pagefind make docs
```

Generated Cobalt source is under `target/project-docs-build/source`, and the site is under `target/project-docs-build/site`. Serve the site directory as the root and visit `/docs/`. For a prefix build, run:

```sh
COBALT=cobalt PAGEFIND=pagefind make docs-prefix
```

Generated Cobalt source is under `target/project-docs-prefix-build/source`, and the site is under `target/project-docs-prefix-build/site`; serve the site directory as the root and visit `/akibiyori-docs/docs/`. Do not serve the inner `akibiyori-docs/` directory as the root. If Cobalt or Pagefind is not on `PATH`, assign `COBALT` or `PAGEFIND` to its executable path. See the [project Docs overview](../) and [installation guide](../install/) for the complete guide set and native prerequisites.

## Build the Observatory tutorial

The Observatory tutorial demonstrates Akibiyori's generated reader using a small example contract:

```sh
COBALT=cobalt PAGEFIND=pagefind make example
COBALT=cobalt PAGEFIND=pagefind make example-prefix
```

The root output is `target/example/observatory-build/site`; the prefix output is `target/example/observatory-prefix-build/site` and uses `/akibiyori-test`. Keep the containing site directory as the server root so the prefixed paths remain in place. The recipe builds the actual Akibiyori CLI, then runs Cobalt and Pagefind; it makes no request to the example API.

The site introduces the [Observatory tutorial API](../../reference/) and its [two operations](../../reference/operations/). To follow a complete schema path, read the [walkthrough](../reading-an-observation/), open the generated [ObservationEnvelope](../../reference/models/catalog/ObservationEnvelope/) model, and follow the links to the selected observation variant and `Calibration.calibration_epoch`. The page-local [search view](../../search/) uses the generated content's own index.

## Build the four-demo showcase

The complete local showcase combines project Docs with four separately built examples:

```sh
COBALT=cobalt PAGEFIND=pagefind make demo-site
COBALT=cobalt PAGEFIND=pagefind make demo-site-prefix
```

The outputs are `target/demo-site/root/site` and `target/demo-site/prefix/site`. The prefix tree includes the `akibiyori-docs/` directory and serves routes such as `/akibiyori-docs/docs/` and `/akibiyori-docs/demos/`; serve the containing `site` directory, not that inner directory. The [demo hub](../../demos/) explains the separate Observatory, Scalar Galaxy, Kubernetes API Extensions and Swagger Petstore inputs. Each keeps its own contract, routes and Pagefind index. These commands build local static examples and make no network request to an example API.
