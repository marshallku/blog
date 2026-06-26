# @blog/styles

## 0.4.0

### Minor Changes

- Self-host the Pretendard Variable font. Adds the woff2 to the build/asset pipeline,
  declares `@font-face` (variable weight `45 920`, `font-display: swap`) in the
  critical bundle, and preloads it in `base.html` so the existing Pretendard font
  stack resolves to the bundled font instead of falling back to system fonts.

## 0.3.0

### Minor Changes

- Add post-detail reading features: reading time, table of contents with scrollspy, reading progress bar, and share buttons (X, LinkedIn, copy link).
