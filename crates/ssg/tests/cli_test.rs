#![allow(deprecated)]

mod common;

use assert_cmd::Command;
use common::{assert_success, stdout_contains, TestEnvironment};
use predicates::prelude::*;

#[test]
fn should_show_help_with_help_flag() {
    // Arrange
    let env = TestEnvironment::minimal();

    // Act
    let result = env.run_help();

    // Assert
    assert_success(&result);
    assert!(stdout_contains(&result, "Static site generator"));
    assert!(stdout_contains(&result, "build"));
    assert!(stdout_contains(&result, "watch"));
    assert!(stdout_contains(&result, "new"));
}

#[test]
fn should_show_version_with_version_flag() {
    // Arrange & Act
    let mut cmd = Command::cargo_bin("blog").expect("Failed to find blog binary");
    let assert = cmd.arg("--version").assert();

    // Assert
    assert.success().stdout(predicate::str::contains("blog"));
}

#[test]
fn should_create_new_post_in_existing_category() {
    // Arrange
    let env = TestEnvironment::minimal();

    // Act
    let result = env.run_new_post("dev", "My New Post");

    // Assert
    assert_success(&result);
    assert!(stdout_contains(&result, "Created"));
    assert!(env.file_exists("content/posts/dev/my-new-post.md"));

    // Verify content
    let content = env.read_file("content/posts/dev/my-new-post.md");
    assert!(content.contains("title: \"My New Post\""));
    assert!(content.contains("category: dev"));
}

#[test]
fn should_show_available_categories_for_nonexistent_category() {
    // Arrange
    let env = TestEnvironment::minimal();

    // Act
    let result = env.run_new_post("nonexistent", "Test Post");

    // Assert
    // Command exits with 0 but shows warning
    assert!(stdout_contains(&result, "doesn't exist"));
    assert!(stdout_contains(&result, "dev")); // Should show available categories
}

#[test]
fn should_accept_incremental_flag() {
    // Arrange
    let env = TestEnvironment::minimal();

    // Act
    let result = env.run_build_incremental();

    // Assert
    assert_success(&result);
    assert!(stdout_contains(&result, "Incremental build uses cache"));
}

#[test]
fn should_accept_parallel_flag() {
    // Arrange
    let env = TestEnvironment::minimal();

    // Act - parallel is default true, test explicit setting
    let result = env.run_build_parallel();

    // Assert
    assert_success(&result);
    assert!(stdout_contains(&result, "threads"));
}

#[test]
fn should_show_build_subcommand_help() {
    // Arrange & Act
    let mut cmd = Command::cargo_bin("blog").expect("Failed to find blog binary");
    let assert = cmd.args(["build", "--help"]).assert();

    // Assert
    assert
        .success()
        .stdout(predicate::str::contains("incremental"))
        .stdout(predicate::str::contains("parallel"));
}

#[test]
fn should_show_new_subcommand_help() {
    // Arrange & Act
    let mut cmd = Command::cargo_bin("blog").expect("Failed to find blog binary");
    let assert = cmd.args(["new", "--help"]).assert();

    // Assert
    assert
        .success()
        .stdout(predicate::str::contains("CATEGORY"))
        .stdout(predicate::str::contains("TITLE"));
}

#[test]
fn should_show_watch_subcommand_help() {
    // Arrange & Act
    let mut cmd = Command::cargo_bin("blog").expect("Failed to find blog binary");
    let assert = cmd.args(["watch", "--help"]).assert();

    // Assert
    assert
        .success()
        .stdout(predicate::str::contains("port"));
}
