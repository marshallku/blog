mod common;

use common::{assert_success, stdout_contains, TestEnvironment};

#[test]
fn should_create_cache_on_first_build() {
    // Arrange
    let env = TestEnvironment::minimal();
    assert!(!env.cache_exists());

    // Act
    let result = env.run_build();

    // Assert
    assert_success(&result);
    assert!(env.cache_exists());
}

#[test]
fn should_skip_unchanged_posts_on_incremental_build() {
    // Arrange
    let env = TestEnvironment::minimal();

    // First incremental build (creates and saves cache)
    let result = env.run_build_incremental();
    assert_success(&result);

    // Act - Second incremental build without changes
    let result = env.run_build_incremental();

    // Assert
    assert_success(&result);
    assert!(stdout_contains(&result, "Skipping"));
}

#[test]
fn should_rebuild_when_post_content_changes() {
    // Arrange
    let env = TestEnvironment::minimal();

    // First build
    let result = env.run_build();
    assert_success(&result);

    // Modify post
    env.modify_post("dev", "test-post");

    // Act - Incremental build
    let result = env.run_build_incremental();

    // Assert
    assert_success(&result);
    assert!(stdout_contains(&result, "Building"));

    // Verify content was updated
    let output = env.read_output("dev/test-post/index.html");
    assert!(output.contains("Modified content"));
}

#[test]
fn should_rebuild_when_template_changes() {
    // Arrange
    let env = TestEnvironment::minimal();

    // First build
    let result = env.run_build();
    assert_success(&result);

    // Modify template
    env.modify_template("post.html");

    // Act - Incremental build
    let result = env.run_build_incremental();

    // Assert
    assert_success(&result);
    assert!(stdout_contains(&result, "Building"));
}

#[test]
fn should_handle_deleted_cache_gracefully() {
    // Arrange
    let env = TestEnvironment::minimal();

    // First build creates cache
    let result = env.run_build();
    assert_success(&result);
    assert!(env.cache_exists());

    // Delete cache
    env.delete_cache();
    assert!(!env.cache_exists());

    // Act - Incremental build without cache
    let result = env.run_build_incremental();

    // Assert - Should rebuild everything
    assert_success(&result);
    assert!(env.cache_exists());
}
