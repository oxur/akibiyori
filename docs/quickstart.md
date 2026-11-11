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

The outputs are `target/demo-site/root/site` and `target/demo-site/prefix/site`. The prefix tree includes the `akibiyori-docs/` directory and serves routes such as `/akibiyori-docs/docs/` and `/akibiyori-docs/demos/`; serve the containing `site` directory, not that inner directory. The [demo hub](../../demos/) explains the separate Observatory, Scalar Galaxy, Kubernetes API Extensions and Swagger Petstore inputs. Each keeps its own contract, routes and Pagefind index. The composed Theme picker offers sixteen profiles in this order: Fuma Dark and Light, Slate Night and Day, six House profiles, four Forest profiles, Botanist Day and Night. It starts on Maple & Mist Light, and the saved Theme choice is shared by root members and separately by prefix members. To assemble another valid manifest, set `SHOWCASE_MANIFEST=/path/to/manifest.json` before the Make target; the script verifies all five fixed owner IDs before replacing a previous owned output. These commands build local static examples and make no network request to an example API.

The standalone `make docs` target builds Akibiyori Docs alone with its existing Fuma Dark default. The standalone `make botanist-example botanist-example-prefix` targets keep the Observatory's two-profile Botanist Night/Day Deploy pair and System mode; the composed showcase uses its sixteen-profile catalog instead.

<h2 id="build-a-botanist-demo-from-an-external-copy">Build a Botanist Demo from an external copy</h2>

This walkthrough copies only the Observatory's authored content, contract directory and Botanist config to a fresh directory outside the checkout. From the repository root, run the real CLI with real Cobalt and Pagefind executables:

```sh
botanist_site=$(mktemp -d)
cp -R examples/observatory/content examples/observatory/contracts "$botanist_site/"
cp examples/observatory/config-theme-botanist.json "$botanist_site/config.json"

cargo run --locked -- build \
  --config "$botanist_site/config.json" \
  --output "$botanist_site/root-build" \
  --theme-mode demo \
  --cobalt cobalt --pagefind pagefind

cargo run --locked -- build \
  --config "$botanist_site/config.json" \
  --output "$botanist_site/prefix-build" \
  --base-path /botanist-preview \
  --theme-mode demo \
  --cobalt cobalt --pagefind pagefind
```

The first output serves from `$botanist_site/root-build/site` at `/`; the second serves from `$botanist_site/prefix-build/site` at `/botanist-preview/`. Serve each containing `site` directory as the web root. The root build includes `/docs/reading-an-observation/`, `/reference/` and `/search/`; the prefix build includes `/botanist-preview/docs/reading-an-observation/`, `/botanist-preview/reference/` and `/botanist-preview/search/`. Both output directories are distinct and new for this example. The built-in Botanist family supplies its profile values, stylesheet and existing font dependencies, so the copied directory needs no theme, CSS or font files.

The Theme control in this Demo offers Botanist Day and Botanist Night, and initially shows Night because the config's mapped Deploy Dark profile is Night. The operating-system preference does not choose the initial Demo profile. Without `--theme-mode demo`, the copied config keeps its original two-profile Deploy pair and `mode: system`, which maps Dark to Night and Light to Day. This CLI override does not change either standalone Make target or the composed showcase.
