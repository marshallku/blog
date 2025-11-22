# @blog/icon

Icon font generator for the blog. Converts SVG icons to web fonts.

## Usage

### Adding Icons

1. Add SVG files to `src/icons/`
2. Run `pnpm build`
3. Copy `dist/icons.css` and font files to `static/`

### SVG Requirements

- Single color (fill will be controlled via CSS)
- Clean paths (no transforms, no embedded styles)
- Recommended size: 24x24 or similar square viewBox

### Build Output

After building, the `dist/` folder contains:

- `icons.woff` / `icons.woff2` - Font files
- `icons.css` - CSS with font-face and icon classes
- `icons.json` - Icon metadata
- `icons.ts` - TypeScript types

### Using Icons in HTML

```html
<link rel="stylesheet" href="/css/icons.css" />
<i class="icon icon-arrow-right"></i>
```

## Scripts

- `pnpm build` - Build all outputs
- `pnpm dev` - Watch mode for development
- `pnpm clean` - Remove dist folder
