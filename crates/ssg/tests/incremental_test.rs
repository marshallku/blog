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

#[test]
fn should_rebuild_when_config_changes() {
    // Arrange
    let env = TestEnvironment::minimal();

    let result = env.run_build_incremental();
    assert_success(&result);

    env.modify_config();

    // Act
    let result = env.run_build_incremental();

    // Assert - Environment change invalidates the whole cache
    assert_success(&result);
    assert!(stdout_contains(&result, "Building"));
    assert!(!stdout_contains(&result, "Skipping"));
}

#[test]
fn should_remove_output_when_post_deleted() {
    // Arrange
    let env = TestEnvironment::minimal();
    env.create_post("dev", "doomed-post", "Doomed Post");

    let result = env.run_build_incremental();
    assert_success(&result);
    assert!(env.output_exists("dev/doomed-post/index.html"));

    env.delete_post("dev", "doomed-post");

    // Act
    let result = env.run_build_incremental();

    // Assert
    assert_success(&result);
    assert!(!env.output_exists("dev/doomed-post/index.html"));
    assert!(env.output_exists("dev/test-post/index.html"));
}

#[test]
fn should_remove_output_when_post_becomes_hidden() {
    // Arrange
    let env = TestEnvironment::minimal();
    env.create_post("dev", "shy-post", "Shy Post");

    let result = env.run_build_incremental();
    assert_success(&result);
    assert!(env.output_exists("dev/shy-post/index.html"));

    env.create_hidden_post("dev", "shy-post", "Shy Post");

    // Act
    let result = env.run_build_incremental();

    // Assert
    assert_success(&result);
    assert!(!env.output_exists("dev/shy-post/index.html"));
}

#[test]
fn should_recover_from_corrupt_cache() {
    // Arrange
    let env = TestEnvironment::minimal();

    let result = env.run_build_incremental();
    assert_success(&result);

    env.corrupt_cache();

    // Act
    let result = env.run_build_incremental();

    // Assert - Falls back to a full rebuild instead of failing
    assert_success(&result);
    assert!(stdout_contains(&result, "Building"));
}

#[test]
fn should_remove_deleted_post_output_even_when_environment_changes() {
    // Arrange - deletion and cache invalidation happen in the same build
    // (e.g. new binary deployed + post deleted)
    let env = TestEnvironment::minimal();
    env.create_post("dev", "doomed-post", "Doomed Post");

    let result = env.run_build_incremental();
    assert_success(&result);
    assert!(env.output_exists("dev/doomed-post/index.html"));

    env.delete_post("dev", "doomed-post");
    env.modify_config();

    // Act
    let result = env.run_build_incremental();

    // Assert - old entries survive invalidation so cleanup still works
    assert_success(&result);
    assert!(!env.output_exists("dev/doomed-post/index.html"));
    assert!(env.output_exists("dev/test-post/index.html"));
}

#[test]
fn should_remove_stale_pagination_pages_when_post_count_shrinks() {
    // Arrange - 12 posts (posts_per_page: 10) → 2 pages
    let env = TestEnvironment::minimal();
    for i in 1..=11 {
        env.create_post("dev", &format!("bulk-{}", i), &format!("Bulk {}", i));
    }

    let result = env.run_build();
    assert_success(&result);
    assert!(env.output_exists("dev/page/2/index.html"));

    // Shrink below one page
    for i in 1..=11 {
        env.delete_post("dev", &format!("bulk-{}", i));
    }

    // Act
    let result = env.run_build();

    // Assert
    assert_success(&result);
    assert!(!env.output_exists("dev/page/2/index.html"));
    assert!(!env.output_exists("dev/page"));
    assert!(env.output_exists("dev/index.html"));
}

#[test]
fn should_remove_tag_dir_when_tag_disappears() {
    // Arrange - fixture posts use tag "test"; add one with a unique tag
    let env = TestEnvironment::minimal();
    env.write_file(
        "content/posts/dev/tagged.md",
        r#"---
title: "Tagged"
date: 2024-03-01T10:00:00Z
tags: [vanishing]
hidden: false
---

Tagged content.
"#,
    );

    let result = env.run_build();
    assert_success(&result);
    assert!(env.output_exists("tag/vanishing/index.html"));

    env.delete_post("dev", "tagged");

    // Act
    let result = env.run_build();

    // Assert
    assert_success(&result);
    assert!(!env.output_exists("tag/vanishing"));
    assert!(env.output_exists("tag/test/index.html"));
}

#[test]
fn should_keep_nested_tag_dirs_alive() {
    // Arrange - a tag containing '/' nests directories under tag/
    let env = TestEnvironment::minimal();
    env.write_file(
        "content/posts/dev/nested-tag.md",
        r#"---
title: "Nested Tag"
date: 2024-03-02T10:00:00Z
tags: [nested/inner]
hidden: false
---

Nested tag content.
"#,
    );

    let result = env.run_build();
    assert_success(&result);
    assert!(env.output_exists("tag/nested/inner/index.html"));

    // Act - rebuild without changes must not prune the live nested dir
    let result = env.run_build();

    // Assert
    assert_success(&result);
    assert!(env.output_exists("tag/nested/inner/index.html"));
}

#[test]
fn should_remove_vanished_nested_tag_under_live_sibling() {
    // Arrange - two nested tags sharing the top-level segment
    let env = TestEnvironment::minimal();
    for (slug, tag) in [("keep-post", "shared/new"), ("drop-post", "shared/old")] {
        env.write_file(
            &format!("content/posts/dev/{}.md", slug),
            &format!(
                r#"---
title: "{}"
date: 2024-03-03T10:00:00Z
tags: [{}]
hidden: false
---

Content.
"#,
                slug, tag
            ),
        );
    }

    let result = env.run_build();
    assert_success(&result);
    assert!(env.output_exists("tag/shared/old/index.html"));
    assert!(env.output_exists("tag/shared/new/index.html"));

    env.delete_post("dev", "drop-post");

    // Act
    let result = env.run_build();

    // Assert - vanished nested tag pruned, live sibling untouched
    assert_success(&result);
    assert!(!env.output_exists("tag/shared/old"));
    assert!(env.output_exists("tag/shared/new/index.html"));
}
