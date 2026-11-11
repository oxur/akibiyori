# Observatory Catalog example

This independently authored, illustrative catalog demonstrates a small OpenAPI contract with two operations, nine named models and three Markdown pages. Its routes, titles and values are for local documentation examples only; they make no claim about a live service.

The contract shows a response envelope, alternative observation models, shared fields, nested references, scalar constraints and recursion. The guide follows the read operation to an observation variant and a nested calibration timestamp.

Build the included Botanist Deploy pair at the root and `/botanist-theme` prefix with the real native tools:

```sh
make botanist-example botanist-example-prefix
```

These targets use [the Botanist Observatory config](config-theme-botanist.json), which pairs Night for Dark with Day for Light in System mode. For an external copied-input Demo with just those two Theme choices, follow the [Quick Start walkthrough](../../docs/quickstart.md#build-a-botanist-demo-from-an-external-copy) and the [Botanist family guide](../../docs/design/botanist.md). The existing family is embedded in Akibiyori; the copied example does not need a theme file.
