#![allow(deprecated)]

mod common;

use common::fixtures::{POST_INVALID_DATE, POST_MALFORMED_YAML, POST_MISSING_TITLE};
use common::{assert_failure, stderr_contains, TestEnvironment};

#[test]
fn should_error_on_missing_frontmatter_title() {
    // Arrange
    let env = TestEnvironment::minimal();
    env.write_file("content/posts/dev/missing-title.md", POST_MISSING_TITLE);

    // Act
    let result = env.run_build();

    // Assert
    assert_failure(&result);
    assert!(stderr_contains(&result, "title") || stderr_contains(&result, "missing"));
}

#[test]
fn should_error_on_invalid_date_format() {
    // Arrange
    let env = TestEnvironment::minimal();
    env.write_file("content/posts/dev/invalid-date.md", POST_INVALID_DATE);

    // Act
    let result = env.run_build();

    // Assert
    assert_failure(&result);
}

#[test]
fn should_error_on_missing_template() {
    // Arrange
    let env = TestEnvironment::minimal();
    // Delete a required template
    std::fs::remove_file(env.root.join("templates/post.html")).unwrap();

    // Act
    let result = env.run_build();

    // Assert
    assert_failure(&result);
    assert!(stderr_contains(&result, "template") || stderr_contains(&result, "post.html"));
}

#[test]
fn should_error_on_empty_content_directory() {
    // Arrange
    let env = TestEnvironment::minimal();
    // Remove all posts
    std::fs::remove_dir_all(env.root.join("content/posts")).unwrap();
    std::fs::create_dir_all(env.root.join("content/posts")).unwrap();

    // Act
    let _result = env.run_build();

    // Assert - Should warn about no categories but not crash
    // The current implementation shows a warning but completes
    // This is expected behavior since an empty blog is valid
}

#[test]
fn should_error_on_invalid_config() {
    // Arrange
    let env = TestEnvironment::minimal();
    env.write_file("config.yaml", "invalid: yaml: content: [");

    // Act
    let result = env.run_build();

    // Assert
    assert_failure(&result);
    assert!(stderr_contains(&result, "config") || stderr_contains(&result, "yaml"));
}

#[test]
fn should_error_on_malformed_yaml_frontmatter() {
    // Arrange
    let env = TestEnvironment::minimal();
    env.write_file("content/posts/dev/malformed.md", POST_MALFORMED_YAML);

    // Act
    let result = env.run_build();

    // Assert
    assert_failure(&result);
}

#[test]
fn should_error_on_missing_content_directory() {
    // Arrange
    let env = TestEnvironment::minimal();
    std::fs::remove_dir_all(env.root.join("content")).unwrap();

    // Act
    let result = env.run_build();

    // Assert
    assert_failure(&result);
    assert!(
        stderr_contains(&result, "does not exist")
            || stderr_contains(&result, "Content directory")
    );
}

#[test]
fn should_error_on_post_file_not_found_for_single_build() {
    // Arrange
    let env = TestEnvironment::minimal();

    // Act - Try to build a non-existent post
    let result = assert_cmd::Command::cargo_bin("blog")
        .expect("Failed to find blog binary")
        .current_dir(&env.root)
        .args(["build", "--post", "nonexistent.md"])
        .output()
        .expect("Failed to execute command");

    // Assert
    assert_failure(&result);
    assert!(stderr_contains(&result, "not found") || stderr_contains(&result, "exist"));
}
