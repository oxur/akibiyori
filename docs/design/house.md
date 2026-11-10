# House colour themes

House is Akibiyori's six-profile colour family in the existing reader. The central showcase exposes every profile in Demo, opens on Maple & Mist Light, and keeps the system-mode Deploy pair at Ember & Ink Dark with Maple & Mist Light. House retains the existing Geist body and heading faces, JetBrains Mono code face, and shared reader geometry.

| Profile ID | Display name | Scheme |
| --- | --- | --- |
| `maple-mist-light` | Maple & Mist Light | Light |
| `maple-mist-dark` | Maple & Mist Dark | Dark |
| `golden-hour-light` | Golden Hour Light | Light |
| `golden-hour-dark` | Golden Hour Dark | Dark |
| `ember-ink-light` | Ember & Ink Light | Light |
| `ember-ink-dark` | Ember & Ink Dark | Dark |

## How the colours meet the reader

Maple & Mist uses warm page and panel surfaces with dark text; its Dark profile uses deep brown surfaces with pale text. Golden Hour pairs sunlit cream and olive accents in Light with a dark umber surface and gold action in Dark. Ember & Ink uses pale rose surfaces and dark text in Light, then deep warm surfaces and pale text in Dark. Code examples remain on the reader's existing panel, with syntax colours selected for each surface. Essential boundaries use a stronger neutral border so controls and panels remain easier to distinguish.

The manifest supplies 32 existing roles per profile. Eight additional derived and overlay roles continue to resolve at their existing body and sidebar owners. The [reader design system](../../design-system/) documents role ownership and the inherited Fumadocs limits; the [colour guide](../colour/) and [shared specimens](../specimens/) show the actual shared patterns. These choices are specific to this host and are not a general accessibility guarantee.

## Use House in an independent site

An author can select any valid Dark and Light combination in a standalone Deploy family. For example, Maple & Mist Dark with Golden Hour Light uses the existing catalog contract:

```json
"theme": {
  "catalog": [{ "builtin": "fumadocs" }, { "builtin": "house" }],
  "deploy": { "dark": "maple-mist-dark", "light": "golden-hour-light" },
  "mode": "system"
}
```

The runnable standalone default pair uses `examples/observatory/config-theme-house.json`; from the repository root, build its root and `/house-theme` versions with `make house-example` and `make house-example-prefix`. This explicit example is separate from Akibiyori's composed showcase: the standalone `make docs` configuration intentionally omits a theme and keeps its Fumadocs default, while the central showcase owns its members' House selection.

The [theme authoring guide](../../themes/) explains the named catalog and Deploy contract. Profile values are kept in the bundled House manifest rather than duplicated in this guide.
