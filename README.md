# ssdocs - Static Site Generator

A blazing-fast, memory-efficient static site generator built in Rust for the marshallku blog.

## Quick Start

### 1. Configure your site

Create a `config.yaml` file in the project root:

```yaml
site:
  title: "My Blog"
  url: "https://example.com"
  author: "Your Name"

theme:
  name: "default"

build:
  content_dir: "content/posts"
  output_dir: "dist"
  posts_per_page: 10
```

If you don't create a config file, ssdocs will use sensible defaults.

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
cargo run -- build

# Incremental build (uses cache)
cargo run -- build --incremental

# Build specific post
cargo run -- build --post content/posts/dev/my-post.md
```

### 5. Development with watch mode

Watch mode automatically rebuilds when files change and serves your site:

```bash
# Start watch mode (default port 8080)
cargo run -- watch

# Use custom port
cargo run -- watch --port 3000
```

Then visit `http://localhost:8080` to view your site. Edit any file in `content/`, `themes/`, or `static/` and it will automatically rebuild!

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
ssdocs/
├── config.yaml            # Site configuration (optional)
├── src/                   # Rust source code
│   ├── main.rs           # CLI and build logic
│   ├── config.rs         # Configuration loading
│   ├── theme.rs          # Theme engine
│   ├── types.rs          # Core types (Post, Category, etc.)
│   ├── parser.rs         # Markdown + frontmatter parsing
│   ├── renderer.rs       # Markdown → HTML rendering
│   ├── generator.rs      # Template application
│   ├── indices.rs        # Index page generation
│   ├── category.rs       # Category discovery
│   ├── metadata.rs       # Metadata cache
│   └── cache.rs          # Build cache management
├── content/
│   └── posts/            # Your blog posts (by category)
│       ├── dev/
│       │   ├── .category.yaml  # Category metadata (optional)
│       │   └── *.md
│       ├── chat/
│       ├── gallery/
│       └── tutorials/
├── themes/               # Theme system
│   └── default/          # Default theme
│       ├── theme.yaml    # Theme metadata
│       ├── base.html     # Base layout
│       ├── post.html     # Post page
│       ├── index.html    # Homepage
│       ├── category.html # Category pages
│       ├── tag.html      # Tag pages
│       ├── tags.html     # Tags overview
│       └── components/   # Reusable components
├── static/               # Static assets (CSS, JS, images)
│   ├── css/
│   ├── js/
│   └── icons/
└── dist/                 # Build output (gitignored)
```

## Commands

### `ssg build`

Build all posts in `content/posts/`.

Options:

- `--incremental`, `-i` - Use cache to skip unchanged files
- `--post <path>`, `-p <path>` - Build only a specific post

### `ssg new`

Create a new blog post with pre-filled frontmatter.

```bash
ssg new <category> "<title>"
```

Example:

```bash
ssg new dev "Building a Rust SSG"
# Creates: content/posts/dev/building-a-rust-ssg.md

ssg new dev "한글 제목"
# Creates: content/posts/dev/한글-제목.md
# URL will be: /dev/%ED%95%9C%EA%B8%80-%EC%A0%9C%EB%AA%A9
```

**Note**: Filenames can contain Korean, Japanese, Chinese, emoji, or any Unicode characters. They are automatically percent-encoded for URLs.

### `ssg watch`

Watch for file changes and automatically rebuild with built-in dev server.

```bash
ssg watch [--port <port>]
```

Options:

- `--port <port>`, `-p <port>` - Port for dev server (default: 8080)

Watches:

- `content/` - Markdown posts
- `themes/` - Theme templates and metadata
- `static/` - CSS, JS, images

The dev server automatically serves your site while watching for changes.

## Configuration

### Site Configuration (config.yaml)

The `config.yaml` file controls your site settings, theme selection, and build options:

```yaml
site:
  title: "My Blog"
  url: "https://example.com"
  author: "Your Name"

theme:
  name: "default" # Theme to use
  variables: # Override theme variables
    primary_color: "#3498db"
    font_family: "Inter, sans-serif"

build:
  content_dir: "content/posts" # Where your posts are
  output_dir: "dist" # Where HTML is generated
  posts_per_page: 10 # Posts per page (pagination)
```

**All fields are optional** - ssdocs will use sensible defaults if `config.yaml` doesn't exist or fields are missing.

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

## Theme System

ssdocs uses a powerful theme system that lets you customize your site's appearance without touching core code.

### Using a Theme

Select a theme in `config.yaml`:

```yaml
theme:
  name: "default" # Use themes/default/
```

### Customizing Theme Variables

Override theme colors, fonts, and other settings:

```yaml
theme:
  name: "default"
  variables:
    primary_color: "#FF5733"
    accent_color: "#C70039"
    font_family: "'Fira Sans', sans-serif"
    max_width: "1200px"
```

### Creating a Custom Theme

1. **Create a theme directory:**

   ```bash
   mkdir -p themes/mytheme
   ```

2. **Create `theme.yaml`:**

   ```yaml
   name: "My Theme"
   version: "1.0.0"
   author: "Your Name"
   parent: "default" # Inherit from default theme

   variables:
     primary_color: "#FF5733"

   required_templates:
     - base.html
     - post.html
     - index.html
   ```

3. **Override templates (optional):**

   ```bash
   # Only create templates you want to customize
   cp themes/default/post.html themes/mytheme/post.html
   # Edit themes/mytheme/post.html
   ```

4. **Activate your theme:**
   ```yaml
   # config.yaml
   theme:
     name: "mytheme"
   ```

**Theme Inheritance**: Child themes automatically fall back to parent theme templates, so you only need to override what changes!

See [THEME_SYSTEM.md](./THEME_SYSTEM.md) for complete documentation and examples.

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

- `title` - Post title (displayed in browser, RSS feed)
- `date.posted` - Publication date (ISO 8601 format)
- `tags` - Array of tags (can be empty: `[]`)

**Optional fields**:

- `date.modified` - Last modified date
- `description` - Meta description for SEO
- `featured_image` - Cover image URL
- `draft` - If `true`, post is excluded from build

**Notes**:

- **Category** is not in frontmatter - it's extracted from directory path
  - `content/posts/dev/file.md` → category: `dev`
- **Tags** can contain non-ASCII characters (Korean, Japanese, etc.)
  - They are automatically percent-encoded for tag page URLs
- **Slug** is generated from filename and percent-encoded for URLs
  - Use `title` for display, not `slug`

### Backwards Compatibility

Simple date format is still supported:

```yaml
date: 2025-11-11T10:00:00Z # Converts to { posted: ..., modified: null }
```

## Non-ASCII Filename Support

ssdocs fully supports Korean, Japanese, Chinese, emoji, and other Unicode characters in filenames and tags:

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

- Filenames and tags are **percent-encoded** for URLs (RFC 3986)
- Browser sends encoded URLs, ssdocs decodes to find files
- No file renaming required - use your native language!
- Display uses `title` from frontmatter, not encoded slug
