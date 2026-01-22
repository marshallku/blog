#![allow(dead_code)]

pub const MINIMAL_CONFIG: &str = r#"site:
  title: "Test Blog"
  url: "https://test.example.com"
  author: "Test Author"
  description: "A test blog"

build:
  content_dir: "content/posts"
  output_dir: "dist"
  posts_per_page: 10
  search:
    enabled: true
"#;

pub const MINIMAL_POST: &str = r#"---
title: "Test Post"
date: 2024-01-15T10:00:00Z
tags: [test, example]
hidden: false
---

This is a test post content.

## Heading

Some more content here.
"#;

pub const HIDDEN_POST: &str = r#"---
title: "Hidden Post"
date: 2024-01-14T10:00:00Z
tags: [hidden]
hidden: true
---

This content should not appear in outputs.
"#;

pub const POST_MISSING_TITLE: &str = r#"---
date: 2024-01-15T10:00:00Z
tags: []
---

Post without title.
"#;

pub const POST_INVALID_DATE: &str = r#"---
title: "Invalid Date Post"
date: not-a-date
tags: []
---

Post with invalid date.
"#;

pub const POST_MALFORMED_YAML: &str = r#"---
title: "Broken
date: 2024-01-15
tags: [
---

Malformed YAML frontmatter.
"#;

pub const MINIMAL_TEMPLATES: &[(&str, &str)] = &[
    ("base.html", BASE_TEMPLATE),
    ("post.html", POST_TEMPLATE),
    ("page.html", PAGE_TEMPLATE),
    ("index.html", INDEX_TEMPLATE),
    ("category.html", CATEGORY_TEMPLATE),
    ("tag.html", TAG_TEMPLATE),
    ("tags.html", TAGS_TEMPLATE),
    ("components/code.html", CODE_COMPONENT),
];

const BASE_TEMPLATE: &str = r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <title>{% block title %}{{ config.site_title }}{% endblock %}</title>
</head>
<body>
    <main>{% block content %}{% endblock %}</main>
</body>
</html>
"#;

const POST_TEMPLATE: &str = r#"{% extends "base.html" %}
{% block title %}{{ post.frontmatter.title }} | {{ config.site_title }}{% endblock %}
{% block content %}
<article>
    <h1>{{ post.frontmatter.title }}</h1>
    <time>{{ post.frontmatter.date.posted }}</time>
    <div class="content">{{ post.rendered_html | safe }}</div>
    {% if prev_post %}
    <a href="{{ prev_post.url }}">Previous</a>
    {% endif %}
    {% if next_post %}
    <a href="{{ next_post.url }}">Next</a>
    {% endif %}
</article>
{% endblock %}
"#;

const PAGE_TEMPLATE: &str = r#"{% extends "base.html" %}
{% block title %}{{ page.frontmatter.title }} | {{ config.site_title }}{% endblock %}
{% block content %}
<article>
    <h1>{{ page.frontmatter.title }}</h1>
    <div class="content">{{ page.rendered_html | safe }}</div>
</article>
{% endblock %}
"#;

const INDEX_TEMPLATE: &str = r#"{% extends "base.html" %}
{% block content %}
<h1>{{ config.site_title }}</h1>
<ul>
{% for post in posts %}
    <li><a href="/{{ post.category }}/{{ post.slug }}/">{{ post.frontmatter.title }}</a></li>
{% endfor %}
</ul>
{% if pagination %}
<nav class="pagination">
    {% if pagination.prev_url %}<a href="{{ pagination.prev_url }}">Previous</a>{% endif %}
    <span>Page {{ pagination.current_page }} of {{ pagination.total_pages }}</span>
    {% if pagination.next_url %}<a href="{{ pagination.next_url }}">Next</a>{% endif %}
</nav>
{% endif %}
{% endblock %}
"#;

const CATEGORY_TEMPLATE: &str = r#"{% extends "base.html" %}
{% block title %}{{ category.name }} | {{ config.site_title }}{% endblock %}
{% block content %}
<h1>{{ category.name }}</h1>
<ul>
{% for post in posts %}
    <li><a href="/{{ post.category }}/{{ post.slug }}/">{{ post.frontmatter.title }}</a></li>
{% endfor %}
</ul>
{% if pagination %}
<nav class="pagination">
    {% if pagination.prev_url %}<a href="{{ pagination.prev_url }}">Previous</a>{% endif %}
    <span>Page {{ pagination.current_page }} of {{ pagination.total_pages }}</span>
    {% if pagination.next_url %}<a href="{{ pagination.next_url }}">Next</a>{% endif %}
</nav>
{% endif %}
{% endblock %}
"#;

const TAG_TEMPLATE: &str = r#"{% extends "base.html" %}
{% block title %}#{{ tag }} | {{ config.site_title }}{% endblock %}
{% block content %}
<h1>#{{ tag }}</h1>
<ul>
{% for post in posts %}
    <li><a href="/{{ post.category }}/{{ post.slug }}/">{{ post.frontmatter.title }}</a></li>
{% endfor %}
</ul>
{% if pagination %}
<nav class="pagination">
    {% if pagination.prev_url %}<a href="{{ pagination.prev_url }}">Previous</a>{% endif %}
    <span>Page {{ pagination.current_page }} of {{ pagination.total_pages }}</span>
    {% if pagination.next_url %}<a href="{{ pagination.next_url }}">Next</a>{% endif %}
</nav>
{% endif %}
{% endblock %}
"#;

const TAGS_TEMPLATE: &str = r#"{% extends "base.html" %}
{% block title %}Tags | {{ config.site_title }}{% endblock %}
{% block content %}
<h1>All Tags</h1>
<ul>
{% for tag in tags %}
    <li><a href="/tags/{{ tag.0 }}/">{{ tag.0 }} ({{ tag.1 }})</a></li>
{% endfor %}
</ul>
{% endblock %}
"#;

const CODE_COMPONENT: &str = r#"<pre><code class="language-{{ lang }}">{{ code }}</code></pre>
"#;
