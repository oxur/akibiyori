<div class="p-3 border rounded-xl bg-fd-card text-fd-card-foreground my-4! text-sm not-prose">
  <p class="font-medium">Using an AI agent?</p>
  <p class="mt-1 mb-4 text-fd-muted-foreground">Copy the setup instructions into your agent (Claude Code, Cursor, Codex, etc.), it will follow the recommended way to set up a docs site.</p>
  <a class="inline-flex items-center justify-center rounded-md border bg-fd-secondary px-3 py-1.5 text-xs font-medium" href="https://github.com/fuma-nama/fumadocs/blob/0cd7bd4a0b7a1fa911e61bbce1242695b1c9e9ef/apps/docs/components/agent-instructions.md" rel="noopener noreferrer">View source instructions</a>
</div>

## Introduction

Fumadocs <span class="text-fd-muted-foreground text-sm">(Foo-ma docs)</span> is a **documentation framework**, designed to be fast, flexible,
and composes seamlessly into your React framework. It consists of multiple layers:

<div class="grid grid-cols-2 gap-3 @container">
  <a data-card="true" class="block rounded-xl border bg-fd-card p-4 text-fd-card-foreground transition-colors @max-lg:col-span-full hover:bg-fd-accent/80" href="https://fumadocs.dev/docs/headless"><div class="not-prose mb-2 w-fit shadow-md rounded-lg border bg-fd-muted p-1.5 text-fd-muted-foreground [&amp;_svg]:size-4"><svg aria-hidden="true" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="lucide lucide-cpu text-purple-300"><path d="M12 20v2"></path><path d="M12 2v2"></path><path d="M17 20v2"></path><path d="M17 2v2"></path><path d="M2 12h2"></path><path d="M2 17h2"></path><path d="M2 7h2"></path><path d="M20 12h2"></path><path d="M20 17h2"></path><path d="M20 7h2"></path><path d="M7 20v2"></path><path d="M7 2v2"></path><rect x="4" y="4" width="16" height="16" rx="2"></rect><rect x="8" y="8" width="8" height="8" rx="1"></rect></svg></div><h3 class="not-prose mb-1 text-sm font-medium">Fumadocs Core</h3><div class="text-sm text-fd-muted-foreground prose-no-margin empty:hidden"><p>Handles most of the logic, including document search, content source adapters, and Markdown extensions.</p></div></a>
  <a data-card="true" class="block rounded-xl border bg-fd-card p-4 text-fd-card-foreground transition-colors @max-lg:col-span-full hover:bg-fd-accent/80" href="https://fumadocs.dev/docs/ui"><div class="not-prose mb-2 w-fit shadow-md rounded-lg border bg-fd-muted p-1.5 text-fd-muted-foreground [&amp;_svg]:size-4"><svg aria-hidden="true" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="lucide lucide-panels-top-left text-blue-300"><rect width="18" height="18" x="3" y="3" rx="2"></rect><path d="M3 9h18"></path><path d="M9 21V9"></path></svg></div><h3 class="not-prose mb-1 text-sm font-medium">Fumadocs UI</h3><div class="text-sm text-fd-muted-foreground prose-no-margin empty:hidden"><p>The default theme of Fumadocs offers a beautiful look for documentation sites and interactive components.</p></div></a>
  <div data-card="true" class="block rounded-xl border bg-fd-card p-4 text-fd-card-foreground transition-colors @max-lg:col-span-full"><div class="not-prose mb-2 w-fit shadow-md rounded-lg border bg-fd-muted p-1.5 text-fd-muted-foreground [&amp;_svg]:size-4"><svg aria-hidden="true" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="lucide lucide-database"><ellipse cx="12" cy="5" rx="9" ry="3"></ellipse><path d="M3 5V19A9 3 0 0 0 21 19V5"></path><path d="M3 12A9 3 0 0 0 21 12"></path></svg></div><h3 class="not-prose mb-1 text-sm font-medium">Content Source</h3><div class="text-sm text-fd-muted-foreground prose-no-margin empty:hidden"><p>The source of your content, can be a CMS or local data layers like <a href="https://fumadocs.dev/docs/mdx">Fumadocs MDX</a> (the official content source).</p></div></div>
  <a data-card="true" class="block rounded-xl border bg-fd-card p-4 text-fd-card-foreground transition-colors @max-lg:col-span-full hover:bg-fd-accent/80" href="https://fumadocs.dev/docs/cli"><div class="not-prose mb-2 w-fit shadow-md rounded-lg border bg-fd-muted p-1.5 text-fd-muted-foreground [&amp;_svg]:size-4"><svg aria-hidden="true" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="lucide lucide-terminal"><path d="M12 19h8"></path><path d="m4 17 6-6-6-6"></path></svg></div><h3 class="not-prose mb-1 text-sm font-medium">Fumadocs CLI</h3><div class="text-sm text-fd-muted-foreground prose-no-margin empty:hidden"><p>A command line tool to install UI components and automate things, useful for customizing layouts.</p></div></a>
</div>

<aside class="flex gap-2 my-4 rounded-xl border bg-fd-card p-3 ps-1 text-sm text-fd-card-foreground" style="--callout-color:var(--color-fd-info, var(--color-fd-muted))"><div role="none" class="w-0.5 bg-(--callout-color)/50 rounded-sm"></div><svg aria-hidden="true" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="lucide lucide-info size-5 -me-0.5 fill-(--callout-color) text-fd-card"><circle cx="12" cy="12" r="10"></circle><path d="M12 16v-4"></path><path d="M12 8h.01"></path></svg><div class="flex flex-col gap-2 min-w-0 flex-1"><p class="font-medium my-0!">Want to learn more?</p><div class="text-fd-muted-foreground prose-no-margin empty:hidden"><p>Read our in-depth <a href="/docs/what-is-fumadocs/">What is Fumadocs</a> introduction.</p></div></div></aside>

### Terminology

**Markdown/MDX:** Markdown is a markup language for creating formatted text. Fumadocs natively supports Markdown and MDX (superset of Markdown).

**[Bun](https://bun.sh):** A JavaScript runtime, it will be used in our guides for running scripts. If you prefer Node.js, try [unrun](https://gugustinette.github.io/unrun/guide/getting-started.html#cli) instead.

Some basic knowledge of React.js would be useful for further customizations.

<aside class="flex gap-2 my-4 rounded-xl border bg-fd-card p-3 ps-1 text-sm text-fd-card-foreground" style="--callout-color:var(--color-fd-info, var(--color-fd-muted))"><div role="none" class="w-0.5 bg-(--callout-color)/50 rounded-sm"></div><svg aria-hidden="true" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="lucide lucide-info size-5 -me-0.5 fill-(--callout-color) text-fd-card"><circle cx="12" cy="12" r="10"></circle><path d="M12 16v-4"></path><path d="M12 8h.01"></path></svg><div class="flex flex-col gap-2 min-w-0 flex-1"><p class="font-medium my-0!">Try Fumapress with simpler DX</p><div class="text-fd-muted-foreground prose-no-margin empty:hidden"><p><a href="https://press.fumadocs.dev">Fumapress</a> is a site generator that manages more features on top of Fumadocs.</p><ul><li>Automatic routing, <code>llms.txt</code>, MCP and more basic features.</li><li>Based on Vite (<a href="https://waku.gg">Waku</a>).</li></ul><p>It enforces a React framework of its own, and offers a better DX in return.</p></div></div></aside>

## Automatic Installation

A minimum version of Node.js 22 required.

```npm
npm create fumadocs-app
```

It will ask you the built-in template to use:

- **Framework**: Next.js, Waku, React Router, Tanstack Start.
- **Content source**: Fumadocs MDX, or content collections.

A new fumadocs app should be initialized, with following features configured by default:

- [LLM integration](https://fumadocs.dev/docs/integrations/llms).
- [Dynamic metadata image](https://fumadocs.dev/docs/integrations/og).

<aside class="flex gap-2 my-4 rounded-xl border bg-fd-card p-3 ps-1 text-sm text-fd-card-foreground" style="--callout-color:var(--color-fd-info, var(--color-fd-muted))"><div role="none" class="w-0.5 bg-(--callout-color)/50 rounded-sm"></div><svg aria-hidden="true" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="lucide lucide-info size-5 -me-0.5 fill-(--callout-color) text-fd-card"><circle cx="12" cy="12" r="10"></circle><path d="M12 16v-4"></path><path d="M12 8h.01"></path></svg><div class="flex flex-col gap-2 min-w-0 flex-1"><p class="font-medium my-0!">From Existing Codebase?</p><div class="text-fd-muted-foreground prose-no-margin empty:hidden"><p>You can follow the <a href="https://fumadocs.dev/docs/manual-installation">Manual Installation</a> guide to get started.</p></div></div></aside>

### Enjoy!

Create your first MDX file in the docs folder.

```mdx title="content/docs/index.mdx"
---
title: Hello World
---

## Yo what's up
```

Run the app in development mode and see http://localhost:3000/docs.

```npm
npm run dev
```

## FAQ

Some common questions you may encounter.

<details>
  <summary>Getting error with missing APIs or bugs?</summary>

Make sure to upgrade Fumadocs when you've encountered any problems or trying out new features:

```bash
pnpm update -i -r --latest
```
</details>

<details>
  <summary>How to change the base route of docs?</summary>

Routing is handled by your React framework, you need to change the routing structure first.

For example, in Next.js, rename the route (`/docs/*` -> `/info/*`):

```text
app/docs/layout.tsx
app/info/layout.tsx
```

Or rename from `/docs/*` to `/*` using a route group:

```text
app/(docs)/layout.tsx
```

Finally, update the base URL of pages in `source.ts`:

```ts
import { loader } from 'fumadocs-core/source';

export const source = loader({
  baseUrl: '/info', // to the new value [!code highlight]
});
```
</details>

<details>
  <summary>How to implement multi-docs?</summary>

We recommend using [Layout Tabs](https://fumadocs.dev/docs/ui/layouts/docs#layout-tabs).
</details>

### For Vite

<details>
  <summary>Getting error with React contexts?</summary>

There's some weird pre-bundling problems for Vite: [#3910](https://github.com/vitejs/vite/issues/3910).
Make sure to exclude Fumadocs from pre-bundling and add it to `noExternal`:

```ts
import { defineConfig } from 'vite';

export default defineConfig({
  resolve: {
    // add other Fumadocs deps as needed
    noExternal: ['fumadocs-core', 'fumadocs-ui', 'fumadocs-openapi', '@fumadocs/base-ui'],
  },
});
```
</details>

### For Next.js

<details>
  <summary>Known Node.js issues</summary>

If you have similar Turbopack errors like:

```
JSON file not found: FileSystemPath { ... path: "lib/locales/sources/file.json.json" }
```

It is usually because `require(esm)` is not supported ([#3312](https://github.com/fuma-nama/fumadocs/issues/3312)), please upgrade to Node.js 22+.

On Node.js 23.1, if you encountered this error in production build:

```
Could not find a production build in the '.next' directory.
```

It is a known problem, see [#1021](https://github.com/fuma-nama/fumadocs/issues/1021), you can upgrade your Node.js version to 23.6.0+.
</details>

<details>
  <summary>It uses Dynamic Route, will it be poor in performance?</summary>

Next.js turns dynamic route into static routes when `generateStaticParams` is configured.
Hence, it is as fast as static pages.

You can [enable Static Exports](https://fumadocs.dev/docs/deploying/static) on Next.js to get a static build output.
</details>

<details>
  <summary>How to create a page in /docs without docs layout?</summary>

Same as managing layouts in Next.js App Router, remove the original MDX file from content directory (`/content/docs`).
This ensures duplicated pages will not cause errors.

Now, You can add the page to another route group, which isn't a descendant of docs layout.

For example, to replace `/docs/test`:

```text
(home)/docs/test/page.tsx
docs/layout.tsx
docs/[[...slug]]/page.tsx
```

For `/docs`, you need to change the catch-all route to be non-optional:

```text
(home)/docs/page.tsx
docs/layout.tsx
docs/[...slug]/page.tsx
```
</details>

## Learn More

New to here? Don't worry, we are welcome for your questions.

If you find anything confusing, please give your feedback on [Github Discussion](https://github.com/fuma-nama/fumadocs/discussions)!

### Writing Content

For authoring docs, make sure to read:

<div class="grid grid-cols-2 gap-3 @container">
  <a data-card="true" class="block rounded-xl border bg-fd-card p-4 text-fd-card-foreground transition-colors @max-lg:col-span-full hover:bg-fd-accent/80" href="https://fumadocs.dev/docs/markdown"><h3 class="not-prose mb-1 text-sm font-medium">Markdown</h3><div class="text-sm text-fd-muted-foreground prose-no-margin empty:hidden"><p>Fumadocs has some additional features for authoring content.</p></div></a>
  <a data-card="true" class="block rounded-xl border bg-fd-card p-4 text-fd-card-foreground transition-colors @max-lg:col-span-full hover:bg-fd-accent/80" href="https://fumadocs.dev/docs/navigation"><h3 class="not-prose mb-1 text-sm font-medium">Navigation</h3><div class="text-sm text-fd-muted-foreground prose-no-margin empty:hidden"><p>Learn how to customize navigation structure.</p></div></a>
  <a data-card="true" class="block rounded-xl border bg-fd-card p-4 text-fd-card-foreground transition-colors @max-lg:col-span-full hover:bg-fd-accent/80" href="https://fumadocs.dev/docs/page-conventions"><h3 class="not-prose mb-1 text-sm font-medium">Page Slugs &amp; Page Tree</h3><div class="text-sm text-fd-muted-foreground prose-no-margin empty:hidden"><p>Learn how to organise content.</p></div></a>
  <a data-card="true" class="block rounded-xl border bg-fd-card p-4 text-fd-card-foreground transition-colors @max-lg:col-span-full hover:bg-fd-accent/80" href="https://fumadocs.dev/docs/ui/components"><h3 class="not-prose mb-1 text-sm font-medium">Components</h3><div class="text-sm text-fd-muted-foreground prose-no-margin empty:hidden"><p>See all available components to enhance your docs</p></div></a>
</div>

### Special Needs

<div class="grid grid-cols-2 gap-3 @container">
  <a data-card="true" class="block rounded-xl border bg-fd-card p-4 text-fd-card-foreground transition-colors @max-lg:col-span-full hover:bg-fd-accent/80" href="https://fumadocs.dev/docs/deploying/static"><h3 class="not-prose mb-1 text-sm font-medium">Configure Static Export</h3><div class="text-sm text-fd-muted-foreground prose-no-margin empty:hidden"><p>Learn how to enable static export on your docs.</p></div></a>
  <a data-card="true" class="block rounded-xl border bg-fd-card p-4 text-fd-card-foreground transition-colors @max-lg:col-span-full hover:bg-fd-accent/80"><h3 class="not-prose mb-1 text-sm font-medium">Internationalization</h3><div class="text-sm text-fd-muted-foreground prose-no-margin empty:hidden"><p>Learn how to enable i18n</p></div></a>
  <a data-card="true" class="block rounded-xl border bg-fd-card p-4 text-fd-card-foreground transition-colors @max-lg:col-span-full hover:bg-fd-accent/80" href="https://fumadocs.dev/docs/ui/theme"><h3 class="not-prose mb-1 text-sm font-medium">Color Themes</h3><div class="text-sm text-fd-muted-foreground prose-no-margin empty:hidden"><p>Add themes to Fumadocs UI</p></div></a>
  <a data-card="true" class="block rounded-xl border bg-fd-card p-4 text-fd-card-foreground transition-colors @max-lg:col-span-full hover:bg-fd-accent/80" href="https://fumadocs.dev/docs/ui/layouts"><h3 class="not-prose mb-1 text-sm font-medium">Layouts</h3><div class="text-sm text-fd-muted-foreground prose-no-margin empty:hidden"><p>Customize the layouts in Fumadocs UI</p></div></a>
</div>
