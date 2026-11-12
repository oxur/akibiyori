# Layout

Layout should help readers keep related information together and find the next useful step. Start with the content order: a short explanation can precede an example, while a reference page may put an operation and its details beside one another when the reader's task calls for it.

## Keep the existing shell in view

The [reader design system](../../design-system/) records current layout roles and the selectors that consume them. The article, navigation, table of contents, operation columns, and narrow behavior belong to the existing reader. This guidance does not claim that a proposed character measure or spacing value is implemented as a universal rule.

Use headings, proximity, lists, and tables to make relationships legible before adding visual separation. A border is useful when it distinguishes an important input or groups content whose relationship is otherwise unclear. Do not use decorative containers that make every paragraph compete for attention.

Review the same content in the generated root and mounted pages. At a narrow width, check that the reading sequence remains understandable, controls remain available, and long links or identifiers do not hide essential information. A width number is only a candidate until the actual content, type, and available reading area have been examined.

## Documentation navigation

Keep authored documentation groups as plain headings and lists that stay visible. Match their heading weight, size, color, line height and inline padding to the OpenAPI navigation, while keeping the smaller gap below a documentation heading and the established space between groups. Indent the links within each list. OpenAPI tag groups remain accordions; shared visual hierarchy does not change their distinct interaction behavior.

## Code fences and examples

Standalone article and callback code fences use one padded, softly rounded PRE frame, with the code itself left transparent and unpadded. When Copy appears, reserve half the former control band above the code and keep the control in a separate intrinsic column; long code scrolls within its own column. Dedicated source disclosures share the frame while keeping their established padding and caption/Copy layout. API examples remain flush inside their card, with padding on the code element; inline code keeps its own compact treatment. Tune shared fence geometry through the reader design system roles, and review the generated root, callback dialog, and source disclosure together.
