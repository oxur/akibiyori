# Colour

Use colour to support a relationship already present in the content. A panel surface groups related material; text colour preserves the reading order; an action colour marks an affordance. A method label or required marker has a specific meaning and should remain distinct from general emphasis.

## Start with the shared roles

Akibiyori's complete supported set of 40 colour roles, their accepted values, owners, and consumers is documented in the [reader design system](../../design-system/). Theme selection and the manifest contract are defined in [theme authoring](../../themes/). Those pages are canonical; this page focuses on how to use the roles in authored guidance.

The following small sample references existing semantic roles. Its appearance follows the profile selected by the current site; it does not prescribe a palette.

| Example | Sample |
| --- | --- |
| Reading surface | <span style="display:inline-block;padding:.5rem 1rem;border:1px solid var(--aki-color-border);background:var(--aki-color-surface-page);color:var(--aki-color-text)">Page text</span> |
| Grouped content | <span style="display:inline-block;padding:.5rem 1rem;border:1px solid var(--aki-color-border);background:var(--aki-color-surface-panel);color:var(--aki-color-text-panel)">Panel text</span> |
| Action emphasis | <span style="display:inline-block;padding:.5rem 1rem;background:var(--aki-color-action);color:var(--aki-color-text-hover)">Action</span> |

## Check meaning and state together

When reviewing a foreground and background, inspect the actual resolved pair in the context where it appears. Check normal, hover, selected, disabled, and focus states separately when they exist. Transparency and overlays depend on what is behind them, so a source token name or hex value alone does not establish the rendered result.

Borders can separate important inputs and controls from surrounding content. Decorative rules can stay quiet when the structure is already clear. Keep focus visibly identifiable, and give status and method meaning words or semantic labels as well as colour. The role system describes available inputs; it is not a universal contrast guarantee. Record pairs or states that have not been checked instead of implying they pass.
