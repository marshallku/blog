# @blog/styles

CSS processing and optimization for the blog using PostCSS.

## Usage

### Development

```bash
pnpm dev     # Watch mode with live reload
pnpm build   # Production build (minified)
pnpm lint    # Run stylelint
```

### Build Output

-   `dist/theme.css` - Minified, auto-prefixed CSS bundle

### Using in HTML

```html
<link rel="stylesheet" href="/css/theme.css" />
```

## Structure

```
src/
├── theme.css          # Main entry point (imports all partials)
├── base/
│   ├── reset.css      # CSS reset
│   ├── variables.css  # CSS custom properties
│   └── typography.css # Typography styles
├── components/
│   ├── header.css     # Header component
│   ├── footer.css     # Footer component
│   ├── navigation.css # Navigation component
│   ├── post.css       # Post and post list
│   ├── code.css       # Code blocks
│   └── pagination.css # Pagination
└── utilities/
    └── utilities.css  # Utility classes
```

## Features

-   **PostCSS** - Modern CSS processing
-   **Autoprefixer** - Auto vendor prefixes
-   **postcss-nested** - Sass-like nesting
-   **postcss-import** - Combine CSS files
-   **cssnano** - Production minification
-   **Stylelint** - CSS linting

## CSS Variables

All theme values use CSS custom properties for easy customization:

```css
:root {
    --color-primary: #0066cc;
    --color-bg: #ffffff;
    --spacing-md: 1rem;
    /* etc. */
}
```

Dark mode is automatic via `prefers-color-scheme`.

## Adding New Styles

1. Create a new CSS file in the appropriate folder
2. Import it in `theme.css`
3. Run `pnpm build`
4. Copy `dist/theme.css` to `static/css/`
