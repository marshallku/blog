pub mod fixtures;

use assert_cmd::Command;
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

use fixtures::{MINIMAL_CONFIG, MINIMAL_POST, MINIMAL_TEMPLATES};

pub struct TestEnvironment {
    pub temp_dir: TempDir,
    pub root: PathBuf,
}

impl TestEnvironment {
    pub fn minimal() -> Self {
        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        let root = temp_dir.path().to_path_buf();

        let env = Self { temp_dir, root };
        env.setup_minimal();
        env
    }

    pub fn with_categories(categories: &[&str]) -> Self {
        let env = Self::minimal();

        for category in categories {
            env.create_category(category);
            env.create_post(
                category,
                &format!("test-{}", category),
                &format!("Test {} Post", category),
            );
        }

        env
    }

    fn setup_minimal(&self) {
        self.create_dir("content/posts/dev");
        self.create_dir("templates/components");
        self.create_dir("static");

        self.write_file("config.yaml", MINIMAL_CONFIG);

        for (name, content) in MINIMAL_TEMPLATES.iter() {
            self.write_file(&format!("templates/{}", name), content);
        }

        self.write_file("content/posts/dev/test-post.md", MINIMAL_POST);
    }

    pub fn create_category(&self, name: &str) {
        self.create_dir(&format!("content/posts/{}", name));
        self.write_file(
            &format!("content/posts/{}/.category.yaml", name),
            &format!("name: {}\nindex: 1", name),
        );
    }

    pub fn create_post(&self, category: &str, slug: &str, title: &str) {
        let content = format!(
            r#"---
title: "{}"
date: 2024-01-15T10:00:00Z
tags: [test]
hidden: false
---

Test content for {}.
"#,
            title, slug
        );
        self.write_file(&format!("content/posts/{}/{}.md", category, slug), &content);
    }

    pub fn create_hidden_post(&self, category: &str, slug: &str, title: &str) {
        let content = format!(
            r#"---
title: "{}"
date: 2024-01-15T10:00:00Z
tags: [test]
hidden: true
---

Hidden content for {}.
"#,
            title, slug
        );
        self.write_file(&format!("content/posts/{}/{}.md", category, slug), &content);
    }

    pub fn create_dir(&self, path: &str) {
        fs::create_dir_all(self.root.join(path)).expect("Failed to create directory");
    }

    pub fn write_file(&self, path: &str, content: &str) {
        let full_path = self.root.join(path);
        if let Some(parent) = full_path.parent() {
            fs::create_dir_all(parent).expect("Failed to create parent directory");
        }
        fs::write(full_path, content).expect("Failed to write file");
    }

    pub fn read_file(&self, path: &str) -> String {
        fs::read_to_string(self.root.join(path)).expect("Failed to read file")
    }

    pub fn run_build(&self) -> std::process::Output {
        Command::cargo_bin("blog")
            .expect("Failed to find blog binary")
            .current_dir(&self.root)
            .args(["build", "--parallel=false"])
            .output()
            .expect("Failed to execute build command")
    }

    pub fn run_build_incremental(&self) -> std::process::Output {
        Command::cargo_bin("blog")
            .expect("Failed to find blog binary")
            .current_dir(&self.root)
            .args(["build", "--incremental", "--parallel=false"])
            .output()
            .expect("Failed to execute build command")
    }

    pub fn run_build_parallel(&self) -> std::process::Output {
        Command::cargo_bin("blog")
            .expect("Failed to find blog binary")
            .current_dir(&self.root)
            .args(["build", "--parallel=true"])
            .output()
            .expect("Failed to execute build command")
    }

    pub fn run_new_post(&self, category: &str, title: &str) -> std::process::Output {
        Command::cargo_bin("blog")
            .expect("Failed to find blog binary")
            .current_dir(&self.root)
            .args(["new", category, title])
            .output()
            .expect("Failed to execute new command")
    }

    pub fn run_help(&self) -> std::process::Output {
        Command::cargo_bin("blog")
            .expect("Failed to find blog binary")
            .current_dir(&self.root)
            .arg("--help")
            .output()
            .expect("Failed to execute help command")
    }

    pub fn output_exists(&self, path: &str) -> bool {
        self.root.join("dist").join(path).exists()
    }

    pub fn read_output(&self, path: &str) -> String {
        fs::read_to_string(self.root.join("dist").join(path)).expect("Failed to read output file")
    }

    pub fn cache_exists(&self) -> bool {
        self.root.join(".build-cache").exists()
    }

    pub fn modify_post(&self, category: &str, slug: &str) {
        let path = format!("content/posts/{}/{}.md", category, slug);
        let mut content = self.read_file(&path);
        content.push_str("\n\nModified content.");
        self.write_file(&path, &content);
    }

    pub fn modify_template(&self, name: &str) {
        let path = format!("templates/{}", name);
        let mut content = self.read_file(&path);
        content.push_str("\n<!-- Modified -->");
        self.write_file(&path, &content);
    }

    pub fn modify_config(&self) {
        let mut content = self.read_file("config.yaml");
        content.push_str("\n# Modified");
        self.write_file("config.yaml", &content);
    }

    pub fn delete_cache(&self) {
        let cache_path = self.root.join(".build-cache");
        if cache_path.exists() {
            fs::remove_dir_all(cache_path).expect("Failed to delete cache");
        }
    }

    pub fn file_exists(&self, path: &str) -> bool {
        self.root.join(path).exists()
    }
}

pub fn assert_success(output: &std::process::Output) {
    if !output.status.success() {
        eprintln!("STDOUT: {}", String::from_utf8_lossy(&output.stdout));
        eprintln!("STDERR: {}", String::from_utf8_lossy(&output.stderr));
        panic!("Command failed with status: {:?}", output.status);
    }
}

pub fn assert_failure(output: &std::process::Output) {
    assert!(
        !output.status.success(),
        "Expected command to fail but it succeeded"
    );
}

pub fn stdout_contains(output: &std::process::Output, text: &str) -> bool {
    String::from_utf8_lossy(&output.stdout).contains(text)
}

pub fn stderr_contains(output: &std::process::Output, text: &str) -> bool {
    String::from_utf8_lossy(&output.stderr).contains(text)
}
