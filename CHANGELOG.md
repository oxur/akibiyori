# Changelog

## 0.1.0-rc1 — Pre-release (2026-11-13)

### Added

- A reusable Rust library and `akibiyori` command-line tool for generating Cobalt source and building static sites from supported OpenAPI contracts and authored Markdown.
- A shared documentation and API-reference reader with generated operation/model pages, local search, progressive examples and navigation, and named theme demo/deploy pairs.
- Public Observatory, Scalar Galaxy, Kubernetes API Extensions and Swagger Petstore examples, plus a complete neutral consumer starter with an OpenAPI contract, Markdown and author-owned image/CSV assets.
- A pinned macOS and Linux CI matrix with native root/prefix site builds, repository checks, package-consumer verification and a separate RustSec audit.
- A consumer-facing dependency and support guide covering host limits, native tools, browser notices and the maintainer refresh process.
- Compile-time emission of first-party and third-party license notices into each generated Cobalt source tree under `licenses/akibiyori/`.

### Updated

- Updated the pinned Ammonia dependency to 4.2.5 for its upstream SVG/MathML mutation-XSS fix.
- Retained the Pagefind 1.5.2 and selected browser-component license texts with public source provenance.

### Support limits

The reader is not a full OpenAPI validator and does not implement every schema dialect or extension. Example contracts remain bounded reading examples, not proof of complete conformance. Cobalt 0.20.4 and Pagefind 1.5.2 are separately installed native tools; author-owned assets require explicit staging. The configured 0.1.0-rc1 CI targets macOS and Linux; Windows is outside the tested host matrix.

### Release status

This pre-release does not stand in for stable 0.1.0. Check crates.io for this exact version before using registry installation; this changelog does not assert registry availability or hosted CI completion. Stable release steps remain separate.
