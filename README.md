# blog - Static Site Generator

A blazing-fast, memory-efficient static site generator built in Rust for the marshallku blog.

This is a **pnpm monorepo** containing:

-   **Rust SSG** - Core static site generator (`src/`)
-   **@blog/icon** - Icon font generator (`packages/icon/`)
-   **@blog/scripts** - TypeScript browser scripts (`packages/scripts/`)
-   **@blog/styles** - CSS processing with PostCSS (`packages/styles/`)

## Quick Start

### 1. Configure your site

Create a `config.yaml` file in the project root:

```yaml
site:
    title: "My Blog"
    url: "https://example.com"
    author: "Your Name"

build:
    content_dir: "content/posts"
    output_dir: "dist"
    posts_per_page: 10
```

If you don't create a config file, blog will use sensible defaults.

### 2. Build the project

```bash
cargo build --release
```

### 3. Create your content structure

```bash
# Create category directories
mkdir -p content/posts/dev
mkdir -p content/posts/tutorials

# Categories are auto-discovered from directories
```

### 4. Build your site

```bash
# Full build
cargo run --release -- build

# Incremental build (uses cache)
cargo run --release -- build --incremental

# Build specific post
cargo run --release -- build --post content/posts/dev/my-post.md
```

### 5. Development with watch mode

Watch mode automatically rebuilds when files change and serves your site:

```bash
# Start watch mode (default port 8080)
cargo run --release -- watch

# Use custom port
cargo run --release -- watch --port 3000
```

Then visit `http://localhost:8080` to view your site. Edit any file in `content/`, `templates/`, or `static/` and it will automatically rebuild!

### View your site (without watch mode)

The generated files are in `dist/`. You can serve them with any static file server:

```bash
# Using Python
python3 -m http.server 8000 --directory dist

# Using a simple Rust server (if you have it installed)
miniserve dist
```

## Project Structure

```
blog/
├── package.json           # pnpm workspace root
├── pnpm-workspace.yaml    # Workspace configuration
├── tsconfig.base.json     # Shared TypeScript config
├── Cargo.toml             # Rust dependencies
├── config.yaml            # Site configuration (optional)
│
├── packages/              # Frontend tooling packages
│   ├── icon/              # @blog/icon - Icon font generator
│   │   ├── src/icons/     # SVG source files
│   │   └── dist/          # Generated fonts + CSS
│   ├── scripts/           # @blog/scripts - TypeScript
│   │   ├── src/           # TypeScript source
│   │   └── dist/          # Bundled JS
│   └── styles/            # @blog/styles - CSS processing
│       ├── src/           # CSS source (PostCSS)
│       └── dist/          # Minified CSS
│
├── src/                   # Rust source code
│   ├── main.rs           # CLI and build logic
│   ├── config.rs         # Configuration loading
│   ├── types.rs          # Core types (Post, Category, etc.)
│   ├── parser.rs         # Markdown + frontmatter parsing
│   ├── renderer.rs       # Markdown → HTML rendering
│   ├── generator.rs      # Template application
│   ├── indices.rs        # Index page generation
│   ├── category.rs       # Category discovery
│   ├── metadata.rs       # Metadata cache
│   ├── cache.rs          # Build cache management
│   ├── feeds.rs          # RSS feed generation
│   ├── search.rs         # Search index generation
│   └── parallel.rs       # Parallel build processing
├── content/
│   └── posts/            # Your blog posts (by category)
│       ├── dev/
│       │   ├── .category.yaml  # Category metadata (optional)
│       │   └── *.md
│       ├── chat/
│       ├── gallery/
│       └── tutorials/
├── templates/            # Tera HTML templates
│   ├── base.html         # Base layout
│   ├── post.html         # Post page
│   ├── index.html        # Homepage
│   ├── category.html     # Category pages
│   ├── tag.html          # Tag pages
│   ├── tags.html         # Tags overview
│   └── components/       # Reusable components
├── static/               # Static assets (CSS, JS, images)
│   ├── css/
│   ├── js/
│   └── icons/
└── dist/                 # Build output (gitignored)
```

## Commands

### `blog build`

Build all posts in `content/posts/` and generate static HTML files.

Options:

-   `--incremental`, `-i` - Use cache to skip unchanged files
-   `--post <path>`, `-p <path>` - Build only a specific post
-   `--parallel` - Enable parallel builds (default: true)

**Output**:

-   Static HTML files in `dist/`
-   RSS feeds (`dist/feed.xml`, per-category feeds)
-   Search index (`dist/search-index.json`)
-   Copied static assets

### `blog new`

Create a new blog post with pre-filled frontmatter.

```bash
blog new <category> "<title>"
```

Example:

```bash
blog new dev "Building a Rust SSG"
# Creates: content/posts/dev/building-a-rust-ssg.md

blog new dev "한글 제목"
# Creates: content/posts/dev/한글-제목.md
# URL will be: /dev/%ED%95%9C%EA%B8%80-%EC%A0%9C%EB%AA%A9
```

**Note**: Filenames can contain Korean, Japanese, Chinese, emoji, or any Unicode characters. They are automatically percent-encoded for URLs.

### `blog watch`

Watch for file changes and automatically rebuild with built-in dev server.

```bash
blog watch [--port <port>]
```

Options:

-   `--port <port>`, `-p <port>` - Port for dev server (default: 8080)

Watches:

-   `content/` - Markdown posts
-   `templates/` - HTML templates
-   `static/` - CSS, JS, images

The dev server automatically serves your site while watching for changes.

## Frontend Tooling

The monorepo includes TypeScript packages for icons, scripts, and CSS. These are built separately from the Rust SSG.

### Setup

```bash
# Install pnpm and node with mise
mise install

# Install dependencies
pnpm install

# Build all packages
pnpm build
```

### Package Commands

```bash
# Build all packages
pnpm build

# Watch mode for all packages
pnpm dev

# Clean all dist folders
pnpm clean

# TypeScript type checking
pnpm typecheck
```

### Individual Packages

**@blog/icon** - Icon font generator

```bash
cd packages/icon
pnpm build      # Generate icon fonts from SVGs
pnpm dev        # Watch mode
```

**@blog/scripts** - TypeScript browser scripts

```bash
cd packages/scripts
pnpm build      # Bundle TypeScript to minified JS
pnpm dev        # Watch mode
```

**@blog/styles** - CSS processing

```bash
cd packages/styles
pnpm build      # Process and minify CSS
pnpm dev        # Watch mode
pnpm lint       # Run stylelint
```

### Deploying Assets

Use the automated build script to build, generate manifest, and copy versioned assets:

```bash
# Build all packages, generate manifest.json, and copy to static/
pnpm build:assets
```

This creates versioned directories in `static/`:

```
static/
├── styles/0.1.0/theme.css
├── scripts/0.1.0/bundle.js
└── icon/0.1.0/icons.css, icons.woff, icons.woff2
```

### Version Management (Changesets)

Uses `@changesets/cli` for semantic versioning:

```bash
# Create a changeset for your changes
pnpm changeset

# Bump versions and update manifest.json
pnpm version

# Build and deploy new versions
pnpm build:assets
```

The Rust SSG reads `manifest.json` and passes asset paths to templates via `config.assets`.
Templates reference versioned paths like `{{ config.assets.styles.theme }}`.

### Adding New Scripts/Assets

To add new assets without modifying Rust code, update the package's `package.json`:

```json
{
    "blog": {
        "assets": {
            "bundle": "bundle.js",
            "search": "search.js",
            "gallery": "gallery.js"
        }
    }
}
```

Run `pnpm manifest` to regenerate paths. Templates access via `{{ config.assets.scripts.search }}`.

## Configuration

### Site Configuration (config.yaml)

The `config.yaml` file controls your site settings and build options:

```yaml
site:
    title: "My Blog"
    url: "https://example.com"
    author: "Your Name"

build:
    content_dir: "content/posts" # Where your posts are
    output_dir: "dist" # Where HTML is generated
    posts_per_page: 10 # Posts per page (pagination)
```

**All fields are optional** - blog will use sensible defaults if `config.yaml` doesn't exist or fields are missing.

### Category Configuration

Categories are automatically discovered from directory structure. Optionally customize them with `.category.yaml`:

```yaml
# content/posts/dev/.category.yaml
name: "Development"
description: "Technical articles about software development"
index: 0 # Sort order (lower = first)
hidden: false # Hide from navigation
icon: "code-blocks" # Optional icon identifier
color: "#66b3ff" # Optional color
```

See [CATEGORY_SYSTEM.md](./CATEGORY_SYSTEM.md) for complete documentation.

## Frontmatter Format

### Post Frontmatter

Posts require YAML frontmatter at the top of the markdown file:

```yaml
---
title: "My Post Title"
date:
    posted: 2025-11-11T10:00:00Z
    modified: 2025-11-12T15:30:00Z # optional
tags: [rust, webdev, 한글태그] # non-ASCII tags supported
description: "Optional meta description"
featured_image: "/images/cover.jpg" # optional
draft: false # optional, default: false
---
# Post content here
```

**Required fields**:

-   `title` - Post title (displayed in browser, RSS feed)
-   `date.posted` - Publication date (ISO 8601 format)
-   `tags` - Array of tags (can be empty: `[]`)

**Optional fields**:

-   `date.modified` - Last modified date
-   `description` - Meta description for SEO
-   `featured_image` - Cover image URL
-   `draft` - If `true`, post is excluded from build

**Notes**:

-   **Category** is not in frontmatter - it's extracted from directory path
    -   `content/posts/dev/file.md` → category: `dev`
-   **Tags** can contain non-ASCII characters (Korean, Japanese, etc.)
    -   They are automatically percent-encoded for tag page URLs
-   **Slug** is generated from filename and percent-encoded for URLs
    -   Use `title` for display, not `slug`

### Backwards Compatibility

Simple date format is still supported:

```yaml
date: 2025-11-11T10:00:00Z # Converts to { posted: ..., modified: null }
```

## Non-ASCII Filename Support

blog fully supports Korean, Japanese, Chinese, emoji, and other Unicode characters in filenames and tags:

```bash
# Korean filename
content/posts/dev/소스코드-검사.md
→ URL: /dev/%EC%86%8C%EC%8A%A4%EC%BD%94%EB%93%9C-%EA%B2%80%EC%82%AC

# Japanese filename
content/posts/tutorials/日本語.md
→ URL: /tutorials/%E6%97%A5%E6%9C%AC%E8%AA%9E

# Tag with Korean
tags: [rust, 한글태그]
→ Tag page: /tag/%ED%95%9C%EA%B8%80%ED%83%9C%EA%B7%B8
```

**How it works**:

-   Filenames and tags are **percent-encoded** for URLs (RFC 3986)
-   Browser sends encoded URLs, blog decodes to find files
-   No file renaming required - use your native language!
-   Display uses `title` from frontmatter, not encoded slug
