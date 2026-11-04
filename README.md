# Akibiyori

Akibiyori is a Rust library and command-line tool for generating Cobalt documentation from supplied OpenAPI JSON contracts and authored Markdown.

The generation entry point is `akibiyori::generate(config_path, output, base_path)`.
Use `akibiyori generate --config site.json --output cobalt-source` with a caller-owned JSON configuration. Titles, contracts, authored pages and navigation are supplied by that configuration.

Use `make check` for the Rust validation available at this revision. Native rendering uses Cobalt 0.20.4. Capabilities vary by revision; consult the CLI help for the commands available at that revision. No historical validation result is asserted by this file.
