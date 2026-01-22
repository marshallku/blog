mod common;

use common::{assert_success, stdout_contains, TestEnvironment};

#[test]
fn should_build_minimal_site_with_single_post() {
    // Arrange
    let env = TestEnvironment::minimal();

    // Act
    let result = env.run_build();

    // Assert
    assert_success(&result);
    assert!(env.output_exists("dev/test-post/index.html"));
    assert!(env.output_exists("index.html"));
    assert!(env.output_exists("sitemap.xml"));
    assert!(env.output_exists("feed.xml"));
}

#[test]
fn should_generate_rss_feeds_for_each_category() {
    // Arrange
    let env = TestEnvironment::with_categories(&["dev", "tutorials"]);

    // Act
    let result = env.run_build();

    // Assert
    assert_success(&result);
    assert!(env.output_exists("feed.xml"));
    assert!(env.output_exists("dev/feed.xml"));
    assert!(env.output_exists("tutorials/feed.xml"));
}

#[test]
fn should_generate_sitemap_with_all_posts() {
    // Arrange
    let env = TestEnvironment::with_categories(&["dev", "tutorials"]);

    // Act
    let result = env.run_build();

    // Assert
    assert_success(&result);
    assert!(env.output_exists("sitemap.xml"));

    let sitemap = env.read_output("sitemap.xml");
    assert!(sitemap.contains("<loc>https://test.example.com/dev/test-dev/</loc>"));
    assert!(sitemap.contains("<loc>https://test.example.com/tutorials/test-tutorials/</loc>"));
}

#[test]
fn should_generate_search_index() {
    // Arrange
    let env = TestEnvironment::minimal();

    // Act
    let result = env.run_build();

    // Assert
    assert_success(&result);
    assert!(env.output_exists("search-index.json"));

    let search_index = env.read_output("search-index.json");
    assert!(search_index.contains("Test Post"));
}

#[test]
fn should_exclude_hidden_posts_from_all_outputs() {
    // Arrange
    let env = TestEnvironment::minimal();
    env.create_hidden_post("dev", "hidden-post", "Hidden Post");

    // Act
    let result = env.run_build();

    // Assert
    assert_success(&result);

    // Hidden post should not have output file
    assert!(!env.output_exists("dev/hidden-post/index.html"));

    // Hidden post should not be in sitemap
    let sitemap = env.read_output("sitemap.xml");
    assert!(!sitemap.contains("hidden-post"));

    // Hidden post should not be in search index
    let search_index = env.read_output("search-index.json");
    assert!(!search_index.contains("Hidden Post"));

    // Hidden post should not be in RSS
    let rss = env.read_output("feed.xml");
    assert!(!rss.contains("Hidden Post"));
}

#[test]
fn should_generate_category_index_pages() {
    // Arrange
    let env = TestEnvironment::with_categories(&["dev", "tutorials"]);

    // Act
    let result = env.run_build();

    // Assert
    assert_success(&result);
    assert!(env.output_exists("dev/index.html"));
    assert!(env.output_exists("tutorials/index.html"));

    let dev_index = env.read_output("dev/index.html");
    assert!(dev_index.contains("test-dev"));
}

#[test]
fn should_generate_tag_pages() {
    // Arrange
    let env = TestEnvironment::minimal();

    // Act
    let result = env.run_build();

    // Assert
    assert_success(&result);
    assert!(env.output_exists("tags/index.html"));
    assert!(env.output_exists("tag/test/index.html"));

    let tag_page = env.read_output("tag/test/index.html");
    assert!(tag_page.contains("Test Post"));
}

#[test]
fn should_copy_static_files() {
    // Arrange
    let env = TestEnvironment::minimal();
    env.write_file("static/style.css", "body { color: black; }");
    env.write_file("static/js/app.js", "console.log('test');");

    // Act
    let result = env.run_build();

    // Assert
    assert_success(&result);
    assert!(env.output_exists("style.css"));
    assert!(env.output_exists("js/app.js"));
}

#[test]
fn should_generate_robots_txt() {
    // Arrange
    let env = TestEnvironment::minimal();

    // Act
    let result = env.run_build();

    // Assert
    assert_success(&result);
    assert!(env.output_exists("robots.txt"));

    let robots = env.read_output("robots.txt");
    assert!(robots.contains("Sitemap:"));
    assert!(robots.contains("sitemap.xml"));
}

#[test]
fn should_generate_atom_feed() {
    // Arrange
    let env = TestEnvironment::minimal();

    // Act
    let result = env.run_build();

    // Assert
    assert_success(&result);
    assert!(env.output_exists("atom.xml"));

    let atom = env.read_output("atom.xml");
    assert!(atom.contains("<feed"));
    assert!(atom.contains("Test Post"));
}

#[test]
fn should_generate_recent_json() {
    // Arrange
    let env = TestEnvironment::minimal();

    // Act
    let result = env.run_build();

    // Assert
    assert_success(&result);
    assert!(env.output_exists("recent.json"));

    let recent = env.read_output("recent.json");
    assert!(recent.contains("Test Post"));
}

#[test]
fn should_show_build_summary() {
    // Arrange
    let env = TestEnvironment::minimal();

    // Act
    let result = env.run_build();

    // Assert
    assert_success(&result);
    assert!(stdout_contains(&result, "Build complete"));
    assert!(stdout_contains(&result, "Built:"));
    assert!(stdout_contains(&result, "Categories:"));
}

#[test]
fn should_build_with_parallel_flag() {
    // Arrange
    let env = TestEnvironment::minimal();

    // Act
    let result = env.run_build_parallel();

    // Assert
    assert_success(&result);
    assert!(env.output_exists("dev/test-post/index.html"));
    assert!(stdout_contains(&result, "threads"));
}
