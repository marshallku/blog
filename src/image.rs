use anyhow::Result;
use serde::Serialize;
use std::path::Path;

/// Standard image sizes for responsive images
pub const IMAGE_SIZES: [u32; 4] = [480, 600, 860, 1180];

/// Thumbnail size for post cards and navigation
pub const THUMBNAIL_SIZE: u32 = 500;

/// LQIP (Low-Quality Image Placeholder) size
pub const LQIP_SIZE: u32 = 10;

/// Individual image source with URL and width
#[derive(Debug, Clone, Serialize)]
pub struct ImageSource {
    pub url: String,
    pub width: u32,
}

/// Image metadata with dimensions and generated source URLs
#[derive(Debug, Clone, Serialize)]
pub struct ImageMetadata {
    /// Original image width
    pub width: u32,
    /// Original image height
    pub height: u32,
    /// Default src URL (original full-size)
    pub src: String,
    /// LQIP placeholder URL (w10)
    pub lqip: String,
    /// Sources for original format (ascending by width)
    pub sources: Vec<ImageSource>,
    /// Sources for WebP format (ascending by width)
    pub webp_sources: Vec<ImageSource>,
}

/// Thumbnail metadata for post cards and navigation (500px)
#[derive(Debug, Clone, Serialize)]
pub struct ThumbnailMetadata {
    /// 500px original format URL
    pub src: String,
    /// 500px WebP format URL
    pub webp_src: String,
}

pub struct ImageProcessor {
    cdn_url: Option<String>,
}

impl ImageProcessor {
    pub fn new(cdn_url: Option<String>) -> Self {
        Self { cdn_url }
    }

    /// Process an image and generate metadata with separate sources
    pub fn process_image(
        &self,
        src: &str,
        content_dir: &Path,
        base_path: &str,
    ) -> Result<Option<ImageMetadata>> {
        if src.starts_with("http://") || src.starts_with("https://") || src.starts_with("//") {
            return Ok(None);
        }

        let cdn_url = match &self.cdn_url {
            Some(url) => url.trim_end_matches('/'),
            None => return Ok(None),
        };

        let image_path = self.resolve_local_path(src, content_dir);

        let (width, height) = match self.get_image_dimensions(&image_path) {
            Ok(dims) => dims,
            Err(_) => {
                return Ok(None);
            }
        };

        // Filter sizes to only include those <= original width
        let sizes: Vec<u32> = IMAGE_SIZES
            .iter()
            .copied()
            .filter(|&s| s <= width)
            .collect();

        let (filename, ext) = self.parse_image_path(src);

        // Generate individual sources for each size
        let sources = self.generate_sources(cdn_url, base_path, &filename, &ext, &sizes, false);
        let webp_sources = self.generate_sources(cdn_url, base_path, &filename, &ext, &sizes, true);

        // Full-size fallback (original)
        let src_url = self.build_cdn_url(cdn_url, base_path, &filename, None, &ext, false);
        let lqip = self.build_cdn_url(cdn_url, base_path, &filename, Some(LQIP_SIZE), &ext, false);

        Ok(Some(ImageMetadata {
            width,
            height,
            src: src_url,
            lqip,
            sources,
            webp_sources,
        }))
    }

    /// Process an image and generate thumbnail metadata (500px)
    pub fn process_thumbnail(
        &self,
        src: &str,
        content_dir: &Path,
        base_path: &str,
    ) -> Result<Option<ThumbnailMetadata>> {
        if src.starts_with("http://") || src.starts_with("https://") || src.starts_with("//") {
            return Ok(None);
        }

        let cdn_url = match &self.cdn_url {
            Some(url) => url.trim_end_matches('/'),
            None => return Ok(None),
        };

        let image_path = self.resolve_local_path(src, content_dir);

        // Verify image exists
        if !image_path.exists() {
            return Ok(None);
        }

        let (filename, ext) = self.parse_image_path(src);

        let src_url =
            self.build_cdn_url(cdn_url, base_path, &filename, Some(THUMBNAIL_SIZE), &ext, false);
        let webp_src =
            self.build_cdn_url(cdn_url, base_path, &filename, Some(THUMBNAIL_SIZE), &ext, true);

        Ok(Some(ThumbnailMetadata {
            src: src_url,
            webp_src,
        }))
    }

    fn resolve_local_path(&self, src: &str, content_dir: &Path) -> std::path::PathBuf {
        let src = src.trim_start_matches("./");
        content_dir.join(src)
    }

    fn get_image_dimensions(&self, path: &Path) -> Result<(u32, u32)> {
        let dimensions = image::image_dimensions(path)?;
        Ok(dimensions)
    }

    fn parse_image_path(&self, src: &str) -> (String, String) {
        let src = src.trim_start_matches("./");

        if let Some(dot_pos) = src.rfind('.') {
            let filename = &src[..dot_pos];
            let ext = &src[dot_pos + 1..];
            (filename.to_string(), ext.to_string())
        } else {
            (src.to_string(), "jpg".to_string())
        }
    }

    /// Build CDN URL with images/ prefix and base_path
    /// Format: {cdn_url}/images/{base_path}/{filename}.w{size}.{ext}[.webp]
    fn build_cdn_url(
        &self,
        cdn_url: &str,
        base_path: &str,
        filename: &str,
        size: Option<u32>,
        ext: &str,
        is_webp: bool,
    ) -> String {
        let size_suffix = size.map(|s| format!(".w{}", s)).unwrap_or_default();
        let webp_suffix = if is_webp { ".webp" } else { "" };
        format!(
            "{}/images/{}/{}{}.{}{}",
            cdn_url, base_path, filename, size_suffix, ext, webp_suffix
        )
    }

    fn generate_sources(
        &self,
        cdn_url: &str,
        base_path: &str,
        filename: &str,
        ext: &str,
        sizes: &[u32],
        is_webp: bool,
    ) -> Vec<ImageSource> {
        let mut sources: Vec<ImageSource> = sizes
            .iter()
            .map(|&size| ImageSource {
                url: self.build_cdn_url(cdn_url, base_path, filename, Some(size), ext, is_webp),
                width: size,
            })
            .collect();

        // Add full-size fallback at the end
        sources.push(ImageSource {
            url: self.build_cdn_url(cdn_url, base_path, filename, None, ext, is_webp),
            width: u32::MAX, // Indicates fallback (no media query)
        });

        sources
    }

}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_image_path() {
        let processor = ImageProcessor::new(Some("https://cdn.example.com".to_string()));

        let (path, ext) = processor.parse_image_path("./images/photo.jpg");
        assert_eq!(path, "images/photo");
        assert_eq!(ext, "jpg");

        let (path, ext) = processor.parse_image_path("images/photo.png");
        assert_eq!(path, "images/photo");
        assert_eq!(ext, "png");
    }

    #[test]
    fn test_build_cdn_url() {
        let processor = ImageProcessor::new(Some("https://cdn.example.com".to_string()));

        // With size - base_path is category only, filename includes post slug directory
        let url = processor.build_cdn_url(
            "https://cdn.example.com",
            "dev",
            "my-post/images/photo",
            Some(480),
            "jpg",
            false,
        );
        assert_eq!(
            url,
            "https://cdn.example.com/images/dev/my-post/images/photo.w480.jpg"
        );

        // Without size (full-size fallback)
        let url = processor.build_cdn_url(
            "https://cdn.example.com",
            "dev",
            "my-post/photo",
            None,
            "png",
            false,
        );
        assert_eq!(
            url,
            "https://cdn.example.com/images/dev/my-post/photo.png"
        );

        // With WebP
        let url = processor.build_cdn_url(
            "https://cdn.example.com",
            "dev",
            "my-post/photo",
            Some(600),
            "png",
            true,
        );
        assert_eq!(
            url,
            "https://cdn.example.com/images/dev/my-post/photo.w600.png.webp"
        );
    }

    #[test]
    fn test_generate_sources() {
        let processor = ImageProcessor::new(Some("https://cdn.example.com".to_string()));

        let sources = processor.generate_sources(
            "https://cdn.example.com",
            "dev",
            "post/photo",
            "jpg",
            &[480, 600],
            false,
        );

        assert_eq!(sources.len(), 3); // 2 sizes + 1 fallback
        assert_eq!(sources[0].width, 480);
        assert!(sources[0].url.contains(".w480."));
        assert_eq!(sources[1].width, 600);
        assert!(sources[1].url.contains(".w600."));
        assert_eq!(sources[2].width, u32::MAX); // fallback
        assert!(!sources[2].url.contains(".w")); // no size suffix
    }

    #[test]
    fn test_skip_external_images() {
        let processor = ImageProcessor::new(Some("https://cdn.example.com".to_string()));
        let content_dir = Path::new("content/posts/dev");

        let result = processor.process_image("https://example.com/image.jpg", content_dir, "dev");
        assert!(result.unwrap().is_none());
    }

    #[test]
    fn test_skip_without_cdn() {
        let processor = ImageProcessor::new(None);
        let content_dir = Path::new("content/posts/dev");

        let result = processor.process_image("./test/image.jpg", content_dir, "dev");
        assert!(result.unwrap().is_none());
    }
}
