use anyhow::{Context, Result};
use std::fs;
use std::path::Path;

use crate::types::Category;

pub fn discover_categories(content_dir: &Path) -> Result<Vec<Category>> {
    let mut categories = Vec::new();
    discover_categories_recursive(content_dir, content_dir, None, &mut categories)?;

    categories.sort_by(|a, b| match a.index.cmp(&b.index) {
        std::cmp::Ordering::Equal => a.name.cmp(&b.name),
        other => other,
    });

    Ok(categories)
}

fn discover_categories_recursive(
    base_dir: &Path,
    current_dir: &Path,
    parent_category: Option<&Category>,
    categories: &mut Vec<Category>,
) -> Result<()> {
    for entry in fs::read_dir(current_dir).with_context(|| {
        format!(
            "Failed to read content directory: {}",
            current_dir.display()
        )
    })? {
        let entry = entry?;
        let path = entry.path();

        if !path.is_dir() || is_hidden(&path) {
            continue;
        }

        if !has_markdown_files_recursive(&path)? {
            continue;
        }

        let slug = path
            .strip_prefix(base_dir)
            .map_err(|_| anyhow::anyhow!("Path is not under base directory"))?
            .to_str()
            .ok_or_else(|| anyhow::anyhow!("Invalid directory name: {}", path.display()))?
            .to_string();

        let category = load_category_metadata(&path, &slug, parent_category)?;

        discover_categories_recursive(base_dir, &path, Some(&category), categories)?;

        categories.push(category);
    }

    Ok(())
}

fn load_category_metadata(dir: &Path, slug: &str, parent: Option<&Category>) -> Result<Category> {
    let metadata_path = dir.join(".category.yaml");

    let mut category = if metadata_path.exists() {
        let content = fs::read_to_string(&metadata_path).with_context(|| {
            format!(
                "Failed to read category metadata: {}",
                metadata_path.display()
            )
        })?;
        serde_yaml::from_str::<Category>(&content)
            .with_context(|| format!("Failed to parse .category.yaml in '{}'", dir.display()))?
    } else {
        let name_part = slug.rsplit('/').next().unwrap_or(slug);
        Category {
            slug: slug.to_string(),
            name: capitalize(name_part),
            description: String::new(),
            index: 999,
            hidden: false,
            icon: None,
            color: None,
            cover_image: None,
        }
    };

    category.slug = slug.to_string();

    // Inherit hidden status from parent if parent is hidden
    if let Some(parent) = parent {
        if parent.hidden && !category.hidden {
            category.hidden = true;
        }
    }

    Ok(category)
}

fn is_hidden(path: &Path) -> bool {
    path.file_name()
        .and_then(|n| n.to_str())
        .map(|n| n.starts_with('.') || n.starts_with('_'))
        .unwrap_or(false)
}

fn has_markdown_files(dir: &Path) -> Result<bool> {
    for entry in fs::read_dir(dir)? {
        let path = entry?.path();
        if path.is_file() {
            if let Some(ext) = path.extension() {
                if ext == "md" || ext == "markdown" {
                    return Ok(true);
                }
            }
        }
    }
    Ok(false)
}

fn has_markdown_files_recursive(dir: &Path) -> Result<bool> {
    if has_markdown_files(dir)? {
        return Ok(true);
    }

    // Check subdirectories recursively
    for entry in fs::read_dir(dir)? {
        let path = entry?.path();
        if path.is_dir() && !is_hidden(&path) && has_markdown_files_recursive(&path)? {
            return Ok(true);
        }
    }

    Ok(false)
}

fn capitalize(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        None => String::new(),
        Some(first) => first.to_uppercase().chain(chars).collect(),
    }
}

pub fn validate_category(slug: &str, categories: &[Category]) -> bool {
    categories.iter().any(|c| c.slug == slug)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_discover_categories() {
        let temp = TempDir::new().unwrap();
        let content = temp.path();

        fs::create_dir(content.join("dev")).unwrap();
        fs::write(content.join("dev/post.md"), "# Test").unwrap();

        let categories = discover_categories(content).unwrap();
        assert_eq!(categories.len(), 1);
        assert_eq!(categories[0].slug, "dev");
        assert_eq!(categories[0].name, "Dev");
    }

    #[test]
    fn test_load_category_metadata() {
        let temp = TempDir::new().unwrap();
        let cat_dir = temp.path().join("dev");
        fs::create_dir(&cat_dir).unwrap();

        let yaml = r#"
name: Development
description: Tech posts
index: 0
        "#;
        fs::write(cat_dir.join(".category.yaml"), yaml).unwrap();

        let category = load_category_metadata(&cat_dir, "dev", None).unwrap();
        assert_eq!(category.name, "Development");
        assert_eq!(category.description, "Tech posts");
        assert_eq!(category.index, 0);
    }

    #[test]
    fn test_nested_category_hidden_inheritance() {
        let temp = TempDir::new().unwrap();
        let content = temp.path();

        // Create parent category with hidden: true
        let parent_dir = content.join("work");
        fs::create_dir(&parent_dir).unwrap();
        fs::write(parent_dir.join(".category.yaml"), "hidden: true").unwrap();

        // Create nested category without its own .category.yaml
        let nested_dir = parent_dir.join("other");
        fs::create_dir(&nested_dir).unwrap();
        fs::write(nested_dir.join("post.md"), "# Test").unwrap();

        let categories = discover_categories(content).unwrap();

        // Both categories should be hidden
        let work = categories.iter().find(|c| c.slug == "work").unwrap();
        let work_other = categories.iter().find(|c| c.slug == "work/other").unwrap();

        assert!(work.hidden);
        assert!(work_other.hidden); // Should inherit hidden from parent
    }

    #[test]
    fn test_hidden_directories_ignored() {
        let temp = TempDir::new().unwrap();
        let content = temp.path();

        fs::create_dir(content.join(".hidden")).unwrap();
        fs::write(content.join(".hidden/post.md"), "# Test").unwrap();

        fs::create_dir(content.join("_private")).unwrap();
        fs::write(content.join("_private/post.md"), "# Test").unwrap();

        let categories = discover_categories(content).unwrap();
        assert_eq!(categories.len(), 0);
    }

    #[test]
    fn test_category_sorting() {
        let temp = TempDir::new().unwrap();
        let content = temp.path();

        for (name, index) in &[("zzz", 0), ("aaa", 2), ("mmm", 1)] {
            let dir = content.join(name);
            fs::create_dir(&dir).unwrap();
            fs::write(dir.join("post.md"), "# Test").unwrap();
            fs::write(dir.join(".category.yaml"), format!("index: {}", index)).unwrap();
        }

        let categories = discover_categories(content).unwrap();
        assert_eq!(categories[0].slug, "zzz");
        assert_eq!(categories[1].slug, "mmm");
        assert_eq!(categories[2].slug, "aaa");
    }

    #[test]
    fn test_validate_category() {
        let categories = vec![
            Category {
                slug: "dev".to_string(),
                name: "Development".to_string(),
                description: String::new(),
                index: 0,
                hidden: false,
                icon: None,
                color: None,
                cover_image: None,
            },
            Category {
                slug: "blog".to_string(),
                name: "Blog".to_string(),
                description: String::new(),
                index: 1,
                hidden: false,
                icon: None,
                color: None,
                cover_image: None,
            },
        ];

        assert!(validate_category("dev", &categories));
        assert!(validate_category("blog", &categories));
        assert!(!validate_category("invalid", &categories));
    }

    #[test]
    fn test_capitalize() {
        assert_eq!(capitalize("dev"), "Dev");
        assert_eq!(capitalize("tutorials"), "Tutorials");
        assert_eq!(capitalize(""), "");
    }
}
