# Quick start

## Build a site as a consumer

Install the local candidate as described in [Install](../install/), and make sure `akibiyori`, Cobalt 0.20.4 and Pagefind 1.5.2 resolve from your shell. Copy the complete `examples/starter-site/` directory from a checkout or extracted package to a fresh `weather-docs` directory **outside the Akibiyori checkout**:

```sh
cp -R /path/to/akibiyori/examples/starter-site /path/to/weather-docs
cd /path/to/weather-docs
akibiyori --version
akibiyori build --config config.json --output target/local-build
cp -R assets target/local-build/site/assets
```

The starter contains `config.json`, `contracts/weather.json`, `content/index.md`, `content/guide.md`, `content/reference.md`, and `assets/`. The configuration reads contract and Markdown source paths relative to itself. Its authored routes are `/`, `/guide/reading-a-reading/`, and the required `/reference/` overview; routes are public URLs, not source filenames. The home page uses a relative guide link and shows the station image. The guide links back to the API operation and model and has a stable `#sample-readings` heading anchor.

Akibiyori does not have an asset-copy setting. After the build, copy the starter's author-owned `assets/` directory to the **absent** `target/local-build/site/assets/` destination. That adds the image and CSV without replacing generated styles, scripts or fonts. In the finished tree, `site/assets/station.svg` is available at `/assets/station.svg`; from the guide route, `../../assets/station.svg` and `../../assets/sample-readings.csv` resolve to those files.

Preview from the site root:

```sh
python3 -m http.server 8000 --bind 127.0.0.1 --directory target/local-build/site
```

Open <http://127.0.0.1:8000/> and follow the guide's image and CSV download links. Stop the server with Ctrl+C before starting another preview.

### Build below a URL prefix

For a site hosted below `/weather-docs/`, make a separate fresh build and stage the same assets at the prefixed physical location:

```sh
akibiyori build --config config.json --output target/prefix-build --base-path /weather-docs
cp -R assets target/prefix-build/site/weather-docs/assets
python3 -m http.server 8000 --bind 127.0.0.1 --directory target/prefix-build/site
```

Open <http://127.0.0.1:8000/weather-docs/>. Keep `site/` as the server root: `site/weather-docs/index.html` maps to `/weather-docs/`, and `site/weather-docs/assets/station.svg` maps to `/weather-docs/assets/station.svg`. Deploy the appropriate completed `site/` subtree for your host's mount point; DNS and hosting configuration are outside this example. When you finish previewing, press Ctrl+C in that terminal before continuing to the manual or rebuild commands below.

### Generate Cobalt source directly

`generate` writes source only; it does not run Cobalt or Pagefind. To stage assets before the real Cobalt copy step, from the starter directory run:

```sh
akibiyori generate --config config.json --output target/manual-build/source
cp -R assets target/manual-build/source/assets
(cd target/manual-build/source && cobalt build --config _cobalt.yml --destination ../site)
pagefind --site target/manual-build/site
```

Cobalt mirrors non-page source files into its destination. Pagefind indexes the built HTML and its labels; this does not claim to index CSV or other file contents. Generated source is disposable: edit the original files under `content/` and `assets/`, then generate into a new output. To rebuild after changing content, use a fresh path such as `target/local-build-v2`; the CLI accepts an absent or empty output directory, and a later native failure may leave partial output. Inspect and remove only a directory you can prove this task owns; never use a blanket cleanup command.

### Rebuild after editing a page

After stopping any active preview server, return to the external `weather-docs` directory. Append an ordinary paragraph to the original guide, then build and stage assets into a new output. The prior `target/local-build` stays available for comparison:

```sh
printf '\n\nThis guide now includes a fresh local field note.\n' >> content/guide.md
akibiyori build --config config.json --output target/local-build-v2
cp -R assets target/local-build-v2/site/assets
python3 -m http.server 8000 --bind 127.0.0.1 --directory target/local-build-v2/site
```

Open <http://127.0.0.1:8000/> to preview the new output, then stop the server with Ctrl+C. The new build leaves the previous output unchanged. Always edit the original source and choose an absent or intentionally empty output; partial native failures need inspection rather than blanket deletion.

Use one visible H1 per page. This starter has no authored frontmatter; if adding frontmatter, use one complete leading `---` block. Akibiyori strips it and emits its own complete Cobalt metadata with `templated: false`. Use route-relative links rather than source filenames or localhost URLs so navigation keeps working under the prefix. The authored Markdown is trusted site source; do not place untrusted input in it. See [Author Markdown](../authoring/) and [Configure a site](../configuration/) for the complete rules.

## Contributor builds

## Build Akibiyori's project docs

After installing the [build prerequisites](../install/), build these docs from the repository root:

```sh
make docs
```

Make finds `cobalt` and `pagefind` on `PATH` by default. The generated Cobalt project is in `target/project-docs-build/source`; the finished site is in `target/project-docs-build/site`. Preview with Python 3:

```sh
python3 -m http.server 8000 --bind 127.0.0.1 --directory target/project-docs-build/site
```

Open <http://127.0.0.1:8000/docs/>. Leave the server running while you browse, then press Ctrl+C in that terminal to stop it before running the next commands. Python 3 is only needed for preview.

For a build that will be served below a URL path such as `/akibiyori-docs/`, run:

```sh
make docs-prefix
```

The leading path is the URL prefix, so `/docs/` becomes `/akibiyori-docs/docs/`. This target writes to the separate `target/project-docs-prefix-build/` directory. Its generated source is in `source/`; the finished site is in `site/`. Preview from that containing site directory:

```sh
python3 -m http.server 8000 --bind 127.0.0.1 --directory target/project-docs-prefix-build/site
```

Open <http://127.0.0.1:8000/akibiyori-docs/docs/>. For example, `site/akibiyori-docs/docs/index.html` maps to that URL when the containing `site/` directory is the server root. See the [project Docs overview](../) and [Build and maintain](../maintenance/) for the full output mapping and explanation. The [installation guide](../install/) lists native build prerequisites. Nondefault executable paths are covered in [Build and maintain](../maintenance/).

## Build the Observatory tutorial

The Observatory tutorial demonstrates Akibiyori's generated reader using a small example contract:

```sh
make example
make example-prefix
```

The root output is `target/example/observatory-build/site`; the prefix output is `target/example/observatory-prefix-build/site` and uses `/akibiyori-test`. In the prefix output, `site/akibiyori-test/index.html` maps to `/akibiyori-test/` when `site/` is the server root. The recipe builds the actual Akibiyori CLI, then runs Cobalt and Pagefind; it makes no request to the example API.

The site introduces the [Observatory tutorial API](../../reference/) and its [two operations](../../reference/operations/). To follow a complete schema path, read the [walkthrough](../reading-an-observation/), open the generated [ObservationEnvelope](../../reference/models/catalog/ObservationEnvelope/) model, and follow the links to the selected observation variant and `Calibration.calibration_epoch`. The page-local [search view](../../search/) uses the generated content's own index.

## Build the four-demo showcase

The complete local showcase combines project Docs with four separately built examples:

```sh
make demo-site
make demo-site-prefix
```

The outputs are `target/demo-site/root/site` and `target/demo-site/prefix/site`. The prefix tree includes `akibiyori-docs/`; for example, `site/akibiyori-docs/docs/index.html` maps to `/akibiyori-docs/docs/` when `site/` is the server root. The [project Docs overview](../) has the docs-only preview commands, and [Build and maintain](../maintenance/) explains the complete output mapping. The [demo hub](../../demos/) explains the separate Observatory, Scalar Galaxy, Kubernetes API Extensions and Swagger Petstore inputs. Each keeps its own contract, routes and Pagefind index.

The composed Theme picker offers eighteen profiles sorted alphabetically by displayed label. Pacific's labels correspond to the stable `pacific-day` and `pacific-night` IDs. It starts on Maple & Mist Light, and the saved Theme choice is shared by root members and separately by prefix members. The standalone `make docs` target builds Akibiyori Docs alone with its existing Fuma Dark default; standalone Botanist targets keep the Observatory's two-profile Botanist Night/Day Deploy pair and System mode.

To assemble another valid manifest, set `SHOWCASE_MANIFEST=/path/to/manifest.json` before the Make target; the script verifies all five fixed owner IDs before replacing a previous owned output. These commands build local static examples and make no network request to an example API.

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
