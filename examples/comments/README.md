# Comment System Examples

This directory contains complete, working examples of different comment system integrations for ssdocs.

Each example is a standalone HTML snippet that you can copy into your `templates/post.html` by overriding the `{% block comments %}` block.

## Available Examples

### 1. **utterances.html** - GitHub Issues (Recommended for developers)
- ✅ Free forever, no ads, open source
- Uses GitHub Issues for comment storage
- Requires GitHub account to comment
- Best for: Technical blogs, open source projects

### 2. **giscus.html** - GitHub Discussions
- ✅ Free, more features than Utterances
- Uses GitHub Discussions instead of Issues
- Threaded discussions, emoji reactions
- Best for: Community-focused blogs

### 3. **disqus.html** - Classic Disqus
- ✅ No GitHub account needed
- ⚠️ Has ads on free tier, privacy concerns
- Familiar to many users
- Best for: General audience blogs

### 4. **custom-backend.html** - Custom REST API
- ✅ Complete control over features
- Example based on marshallku.com implementation
- Includes: Nested comments, password-based auth, Korean spam prevention
- Best for: Custom requirements, existing backend

## How to Use

### Step 1: Choose an example
Pick the example that best fits your needs. If you're not sure, start with `utterances.html` (easiest setup).

### Step 2: Copy to your template
Copy the entire content of the example file into your `templates/post.html`:

```html
{% extends "base.html" %}

{% block content %}
<!-- Your existing post content -->
{% endblock %}

{% block comments %}
<!-- Paste example content here -->
{% endblock %}
```

### Step 3: Configure
Each example has configuration placeholders marked with `YOUR-` or `your-`:
- `your-username/your-repo` → Replace with your GitHub repo
- `YOUR-SHORTNAME` → Replace with your Disqus shortname
- `https://api.yourdomain.com` → Replace with your API URL

### Step 4: Test
Run your blog locally and verify comments work:
```bash
cd ssdocs
cargo run -- build
cargo run -- watch
```

Open http://localhost:8080 and navigate to a post.

## Customization

All examples can be customized:
- **Styling**: Add your own CSS classes
- **Layout**: Modify HTML structure
- **Features**: Add/remove functionality
- **Translations**: Change text to your language

## Need Help?

See the comprehensive [COMMENT_SYSTEM.md](../../COMMENT_SYSTEM.md) guide for:
- Detailed implementation guides
- Best practices
- Security considerations
- Performance tips
- Troubleshooting
