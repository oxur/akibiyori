# Akibiyori contributor instructions

## Project scope

Akibiyori is a reusable Rust package for generating Cobalt source from OpenAPI contracts and authored Markdown. Keep the library API usable without the CLI, keep `src/main.rs` focused on argument parsing and process exit mapping, and keep example contracts illustrative and independently authored.

## Public package boundary

Keep employer, customer and other organization-specific contracts, identifiers, trackers, planning records and local paths out of this public repository. Do not include private data or derived private corpus content in examples, tests, documentation, commit messages or generated public artifacts. Public examples should describe only their own illustrative domains.

## Git and authority

Preserve the inherited Git identity. Do not configure a remote, push, publish, or change branches unless separately authorized. Use neutral descriptive commit subjects. Do not stage unknown or unrelated files; inspect and stage only the files owned by the current task.

## Verification

Do not use mocks, fake CLIs, fake services, fake harnesses or second implementations of external or repository behavior. Use real unit/integration tests, static inspection, or real Make recipes, and state clearly when a claim is deferred. Use the actual supported Cobalt and Pagefind executables for native validation.

Run project Make targets for validation:

- `make check` for formatting, compilation, clippy, real native integration tests and documentation.
- `make example` for the root Observatory example.
- `make example-prefix` for the `/akibiyori-test` example.

Generated build artifacts belong under `target/` and must not be committed. A cleanup recipe may remove only outputs that its ownership marker proves it owns.
