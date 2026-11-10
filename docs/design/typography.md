# Typography

Typography helps readers recognize where they are and what kind of information follows. Use real heading levels in order, and give each section a heading that describes its content. Do not choose a heading level only because of its default size.

## Use the current type roles

The current body and heading stack uses Geist with its existing fallbacks; code uses JetBrains Mono with its fallbacks. Profiles may provide supported alternatives. Forest changes only the heading role to Georgia, Times New Roman, serif; body and code remain Geist and JetBrains Mono. The [reader design system](../../design-system/) documents the actual type roles and ownership, and [theme authoring](../../themes/) explains how a profile supplies complete font stacks and local faces.

Let body text carry explanations, headings show the outline, and inline code distinguish exact names such as `observation_id`. Use code blocks for structured examples, not long prose. Keep identifiers intact and allow them to wrap where a narrow reading area needs it. Font availability, glyph coverage, fallback appearance, and user enlargement depend on the selected assets and environment; this guide does not claim those have been verified for every profile.

Review a page with its actual examples and headings. Check that readers can scan the outline, read a long identifier without losing its context, and distinguish prose from code. Keep markup meaningful if a custom face is unavailable or text is enlarged.
