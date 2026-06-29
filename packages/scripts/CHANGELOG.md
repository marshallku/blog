# @blog/scripts

## 0.5.1

### Patch Changes

- Make the scroll-to-top button angular and complete the radius scale. The button drops
  its border and is now a sharp rounded-square (matching the overall vibe), with its
  scroll-progress drawn as a rounded `<rect>` tracing the outline (perimeter read from
  geometry). Adds `xl` (8px) and `full` (9999px) to the radius token scale and tokenizes
  the remaining hardcoded `border-radius` values across components (pills → `full`,
  8px → `xl`, 4px → `md`); intentional squares/circles stay literal.

## 0.5.0

### Minor Changes

- Card → post-cover shared-element page transition. SPA navigation now runs through the
  View Transitions API: clicking a post card's image/title morphs the thumbnail into the
  post's cover image (matching `view-transition-name`), with a plain cross-fade for all
  other navigation. Falls back to a plain swap when the API is unsupported or
  `prefers-reduced-motion` is set; tag-link clicks and cover-less destinations are
  excluded from the morph.

## 0.4.0

### Minor Changes

- Restrained hacker/developer cohesion pass + scroll-progress ring. Unifies the UI on
  one visual system: monospace as the accent typeface (meta, tags, nav, labels, code),
  a single brand-pink accent (links/tags/active/hover), sharp low radius, and hairline
  borders — across post, cards, home tabs, tag cloud, pagination, TOC, share, header.
  Replaces the top reading-progress bar with a bottom-right circular scroll-to-top
  button whose SVG ring stroke fills with whole-page scroll progress.

## 0.3.0

### Minor Changes

- Add view-counter ping that records a post view on load and SPA navigation (deduplicated server-side per visitor/day).

## 0.2.0

### Minor Changes

- Add post-detail reading features: reading time, table of contents with scrollspy, reading progress bar, and share buttons (X, LinkedIn, copy link).
