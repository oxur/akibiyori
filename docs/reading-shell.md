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
accepted icon names are `rocket`, `circle-help`, and `book-open`; omitting an
icon uses the label's initial. Routes are prefixed once when a base path is
configured. With no `sidebar_sections`, the selector is omitted.

The native disclosure lists these destinations. Its label and icon follow the
longest matching route prefix; selecting an item navigates to that page and
does not filter the sidebar groups. The desktop rail collapse control hides
the rail from keyboard and assistive technology navigation, leaves a narrow
reopen gutter, and returns focus to the paired control on expansion. On mobile,
the separate Menu button opens the navigation drawer.

Search remains Pagefind's generated interface. Its result list scrolls inside
the reading panel, and its text, surface, border, primary, tag, and font roles
use the existing dark reference theme tokens. The sidebar's Search link is
available without a keyboard shortcut.
