# Reading shell configuration

The reference example shows the optional sidebar section selector. Add
`sidebar_sections` to a site configuration to provide its internal destinations:

```json
{
  "sidebar_sections": [
    {"label": "Framework", "href": "/", "icon": "book-open"},
    {"label": "Catalog API", "href": "/reference/", "icon": "book-open"}
  ]
}
```

Each route must be unique and resolve to an authored or generated page. The
accepted icon names are `rocket`, `circle-help`, and `book-open`; a section
without an icon uses the label's initial. Routes are prefixed once when a base
path is configured. With no `sidebar_sections`, the selector is omitted.
The reference specimen's three authored internal links use route-relative hrefs
so the same content resolves at the root and beneath a configured prefix.

An optional `brand_icon` is a trusted, consumer-owned PNG path relative to the
configuration directory. It must remain inside that directory, cannot traverse
symlinks, and is checked for the PNG signature before output is written. When
present, the exact input bytes are emitted at the fixed `icons/brand.png` path
and shown at 20×20 CSS pixels beside the site title. Without it, no brand image
file or image slot is generated.

The Search link opens a dedicated full-page search view; it is not the modal
search dialog in the upstream framework. The native `<details>` disclosure
lists section destinations even without JavaScript. Its current label/icon
enhancement follows the longest matching route prefix; selecting an item
navigates to that page and does not filter the sidebar groups. Without the
reading script the generic “Sections” label remains, while native links and
disclosure still work. The desktop rail collapse/reopen controls, active
section update, generated TOC, and clipboard controls require the reading
script. On mobile, the separate Menu button opens the navigation drawer when
the reading script is loaded.

Search remains Pagefind's generated interface. Its results require the local
Pagefind assets and search bootstrap script. The result list scrolls inside the
reading panel, and its text, surface, border, primary, tag, and font roles use
the existing dark reference theme tokens. The sidebar's Search link remains
available without JavaScript or a keyboard shortcut.
