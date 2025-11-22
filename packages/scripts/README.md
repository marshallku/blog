# @blog/scripts

TypeScript scripts for the blog. Bundled with tsup for browser usage.

## Usage

### Development

```bash
pnpm dev     # Watch mode
pnpm build   # Production build
```

### Build Output

- `dist/bundle.js` - Minified browser bundle
- `dist/index.js` - ESM module for imports
- `dist/index.d.ts` - TypeScript declarations

### Using in HTML

```html
<script type="module" src="/js/bundle.js"></script>
```

### Features

The bundle includes:

- **Scroll to top** - Shows button when scrolled down
- **Lazy images** - Loads images with `data-src` on scroll
- **Code copy** - Adds copy button to code blocks

### Adding New Features

1. Add code to `src/bundle.ts` or create new modules
2. Import and use in the initialization function
3. Run `pnpm build`
4. Copy `dist/bundle.js` to `static/js/`

### Utilities

The package exports utility functions:

```typescript
import { debounce, throttle, isInViewport } from "@blog/scripts";
```
