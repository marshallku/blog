# @blog/styles

## 0.8.4

### Patch Changes

- Darken the comment composer background to a panel below the window (was a light tint that read brighter than the mockup).

## 0.8.3

### Patch Changes

- Wrap long code-block lines instead of scrolling horizontally — a horizontal scroll carried the absolutely-positioned terminal header bar off-screen and clipped it.

## 0.8.2

### Patch Changes

- Make the comment composer textarea multi-line by default and auto-grow with content (capped, then scrolls), instead of a fixed single-line height that clipped wrapped text on mobile.

## 0.8.1

### Patch Changes

- Restyle the comment thread (and guestbook) as a terminal/chat treatment: glass avatars, pink nicknames, OP bubbles right-aligned with a pink badge + tint, dividers between top-level comments, and a terminal-window composer with `~$`/`@` inputs and a `send ↵` button.

## 0.8.0

### Minor Changes

- Add the About page terminal/résumé design system (`about.css` route bundle) and re-point `--font-sans` to a Pretendard-led stack.

## 0.7.0

### Minor Changes

- Refine the hacker/terminal design pass on post detail, cards, and home, and adapt the light/sepia palettes to match.

    - Post detail: comment-style last-modified line, smaller mono meta row, terminal-window TOC (`tree ./article` with `▸/·/└` markers), and prev/next cards retoned to the token-based hacker look.
    - Cards: filename label truncates with ellipsis; footer shows reading time (`N분`), backed by a new `reading_time` field on `PostMetadata`.
    - Home: hero description typewriter effect + live status-bar clock, profile restructured (image + `[ icon ]` socials), and a flex filter toolbar that wraps on mobile.
    - Light and sepia palettes re-pointed to cohesive terminal tones (decoupled hairline/tint tokens); code-block and hero header bars tokenized so they read on every theme.

## 0.6.0

### Minor Changes

- Make the scroll-to-top button angular and complete the radius scale. The button drops
  its border and is now a sharp rounded-square (matching the overall vibe), with its
  scroll-progress drawn as a rounded `<rect>` tracing the outline (perimeter read from
  geometry). Adds `xl` (8px) and `full` (9999px) to the radius token scale and tokenizes
  the remaining hardcoded `border-radius` values across components (pills → `full`,
  8px → `xl`, 4px → `md`); intentional squares/circles stay literal.

## 0.5.1

### Patch Changes

- Card → post-cover shared-element page transition. SPA navigation now runs through the
  View Transitions API: clicking a post card's image/title morphs the thumbnail into the
  post's cover image (matching `view-transition-name`), with a plain cross-fade for all
  other navigation. Falls back to a plain swap when the API is unsupported or
  `prefers-reduced-motion` is set; tag-link clicks and cover-less destinations are
  excluded from the morph.

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
