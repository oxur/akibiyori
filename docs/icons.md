# Favicon and home-screen icon pack

Akibiyori can publish a consumer-owned icon pack and add its HTML head tags to
both documentation and search pages. This is optional and independent of
`brand_icon`, which controls the small image beside the site title.

## Generate the supplied leaf pack locally

Run `make icons` from the repository root with ImageMagick 7 (`magick`) on your
PATH. No online icon service or Node/npm package is used. The script reads
`assets/icons/leaves.png` and writes only these derived files:

| File | Pixels | Use |
| --- | --- | --- |
| `favicon.ico` | 16, 32, 48 | Multi-image browser/shortcut fallback |
| `favicon-16x16.png` | 16 × 16 | Small browser icon |
| `favicon-32x32.png` | 32 × 32 | Browser icon at higher density |
| `favicon-48x48.png` | 48 × 48 | Larger browser icon |
| `apple-touch-icon.png` | 180 × 180 | Apple home-screen/bookmark icon |
| `icon-192x192.png` | 192 × 192 | Web app manifest icon |
| `icon-512x512.png` | 512 × 512 | Larger web app manifest icon |
| `icon-maskable-512x512.png` | 512 × 512 | Adaptive/maskable home-screen icon |

The four original `leaves*.png` images are preserved. Browser and ordinary
manifest PNGs retain transparency. Apple and maskable icons have an opaque
`#111111` background. The maskable artwork fits in a centered 280px square,
wholly inside the 512px canvas's 40%-radius safe circle. Its corners remain
square so the operating system can supply its own mask. The rebuild tool is
only needed to change the pack; normal Rust/Cobalt builds copy the supplied
files without requiring ImageMagick.

## Enable a pack in a site

Copy the eight derived files into a directory beneath the consuming site's
configuration directory, for example `icons/`. Add this top-level setting to
its JSON config:

```json
{
  "icon_pack": {
    "directory": "icons",
    "theme_color": "#111111",
    "background_color": "#111111"
  }
}
```

This fragment belongs alongside the existing `version`, `title`, `contracts`
and `pages` fields. Both colors default to `#111111`; provided values must be
six-digit hexadecimal colors. If you regenerate icons on a different opaque
background, set the matching colors here too. The directory must be nonempty
and relative to the config, with no dot segments or symbolic links in any
input path. All eight files are required. PNG headers/dimensions and the ICO
directory's dimensions/bounds are checked before output writes; this is not a
full image decode or validation of a custom pack's maskable safe area.

Generation copies PNGs to `icons/`, places `favicon.ico` at the site's root,
and emits `site.webmanifest` using the site's own title. The manifest's `id`
and `scope` match the configured base path. Its `start_url` opens the authored
home page, or `/reference/` when no home page exists. Icon `src` URLs are
relative to the manifest so they also work below a project prefix.

The shared `theme/_includes/icons.liquid` emits these links and metadata:

```html
<link rel="icon" href="/favicon.ico" sizes="16x16 32x32 48x48" type="image/x-icon">
<link rel="icon" href="/icons/favicon-16x16.png" sizes="16x16" type="image/png">
<link rel="icon" href="/icons/favicon-32x32.png" sizes="32x32" type="image/png">
<link rel="icon" href="/icons/favicon-48x48.png" sizes="48x48" type="image/png">
<link rel="apple-touch-icon" href="/icons/apple-touch-icon.png" sizes="180x180">
<link rel="manifest" href="/site.webmanifest">
<meta name="theme-color" content="#111111">
<meta name="application-name" content="Your site title">
<meta name="apple-mobile-web-app-title" content="Your site title">
```

These examples show a root deployment. Akibiyori inserts the configured prefix
exactly once in generated head URLs, for example `/project/icons/favicon-32x32.png`.
No tags or public pack files are emitted when `icon_pack` is absent. Existing
`brand_icon` behavior remains separate.

The manifest uses `display: "browser"`. This provides site identity and icons
without changing normal browser navigation, adding a service worker or offline
cache, or claiming installability across platforms. Native app stores require
their own platform-specific asset catalogs. A multicolor favicon is also not
automatically a Safari monochrome pinned-tab mask.

## Sources and verification

- [Apple: specifying a webpage icon](https://developer.apple.com/library/archive/documentation/AppleApplications/Reference/SafariWebContent/ConfiguringWebApplications/ConfiguringWebApplications.html)
- [MDN: manifest icons](https://developer.mozilla.org/en-US/docs/Web/Progressive_web_apps/Manifest/Reference/icons)
- [MDN: maskable icons and safe zones](https://developer.mozilla.org/en-US/docs/Web/Progressive_web_apps/How_to/Define_app_icons)

`make check` exercises real Cobalt and Pagefind builds at the site root and
under `/autumn`, checks both reading and search heads, resolves the manifest's
asset paths, preserves the supplied bytes, and covers missing/malformed packs,
unsafe inputs and public output collisions. ImageMagick can inspect the actual
pack with `magick identify assets/icons/*.png assets/icons/favicon.ico`.
