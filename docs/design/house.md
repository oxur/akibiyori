# House colour themes

Akibiyori's first House release brings two selected colour profiles into the existing reader: **Maple & Mist Light** and **Ember & Ink Dark**. The central showcase exposes both in Demo, opens on Maple & Mist Light, and uses them as the system-mode Deploy pair. The family keeps the existing Geist body and heading faces, JetBrains Mono code face, and shared reader geometry.

## How the colours meet the reader

Maple & Mist uses warm page and panel surfaces with dark text. Code examples remain on the reader's existing light panel, with dark syntax colours chosen to stay distinct on that surface. Essential boundaries use a stronger neutral border so controls and panels remain easier to distinguish. Ember & Ink uses deep warm surfaces, pale text, and the selected light syntax colours.

The manifest supplies 32 existing roles per profile. Eight additional derived and overlay roles continue to resolve at their existing body and sidebar owners. The [reader design system](../../design-system/) documents role ownership and the inherited Fumadocs limits; the [colour guide](../colour/) and [shared specimens](../specimens/) show the actual shared patterns. These choices are specific to this host and are not a general accessibility guarantee.

## Use House in an independent site

An author can select the pair as a standalone Deploy family using `examples/observatory/config-theme-house.json`. From the repository root, build its root and `/house-theme` versions with `make house-example` and `make house-example-prefix`. This explicit example is separate from Akibiyori's composed showcase: the standalone `make docs` configuration intentionally omits a theme and keeps its Fumadocs default, while the central showcase owns its members' House selection.

The [theme authoring guide](../../themes/) explains the named catalog and Deploy contract. Current House delivery contains these two profiles; the other selected House variants are a later increment.
