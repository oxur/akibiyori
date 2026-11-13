# Weather Station starter

This complete neutral starter demonstrates one OpenAPI operation and model, three authored routes, a Botanist Dark/Light Deploy pair, and two small author-owned assets.

## Build this site

Install Akibiyori locally and install the supported Cobalt 0.20.4 and Pagefind 1.5.2 tools. Confirm the commands resolve from your shell:

```sh
akibiyori --version
cobalt --version
pagefind --version
```

Copy this complete directory to a fresh location outside the Akibiyori checkout, then run the installed CLI and stage this starter's author-owned files:

```sh
cp -R /path/to/akibiyori/examples/starter-site /path/to/weather-docs
cd /path/to/weather-docs
akibiyori build --config config.json --output target/local-build
cp -R assets target/local-build/site/assets
python3 -m http.server 8000 --bind 127.0.0.1 --directory target/local-build/site
```

Open <http://127.0.0.1:8000/> and follow the guide, image and CSV download links. Stop the server with Ctrl+C before running another command. Build outputs must be new or empty; to edit the site, change files under `content/` or `assets/` and choose a new output such as `target/local-build-v2`. Keep this original starter as the source of truth. If you have the complete distribution, `docs/quickstart.md` has separate prefix and direct-Cobalt examples.

The image and CSV are illustrative static files. Nothing here contacts a station or represents live readings.
