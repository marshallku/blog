# @blog/styles

## 0.5.0

### Minor Changes

- Restrained hacker/developer cohesion pass + scroll-progress ring. Unifies the UI on
  one visual system: monospace as the accent typeface (meta, tags, nav, labels, code),
  a single brand-pink accent (links/tags/active/hover), sharp low radius, and hairline
  borders — across post, cards, home tabs, tag cloud, pagination, TOC, share, header.
  Replaces the top reading-progress bar with a bottom-right circular scroll-to-top
  button whose SVG ring stroke fills with whole-page scroll progress.

## 0.4.0

### Minor Changes

- Self-host the Pretendard Variable font. Adds the woff2 to the build/asset pipeline,
  declares `@font-face` (variable weight `45 920`, `font-display: swap`) in the
  critical bundle, and preloads it in `base.html` so the existing Pretendard font
  stack resolves to the bundled font instead of falling back to system fonts.

## 0.3.0

### Minor Changes

- Add post-detail reading features: reading time, table of contents with scrollspy, reading progress bar, and share buttons (X, LinkedIn, copy link).
