use anyhow::Result;
use std::path::Path;

/// Standard image sizes for responsive images
pub const IMAGE_SIZES: [u32; 4] = [480, 600, 860, 1180];

/// Image metadata with dimensions and generated srcset URLs
#[derive(Debug, Clone)]
pub struct ImageMetadata {
    /// Original image width
    pub width: u32,
    /// Original image height
    pub height: u32,
    /// Default src URL (largest available size)
    pub src: String,
    /// Srcset for original format
    pub srcset: String,
    /// Srcset for WebP format
    pub webp_srcset: String,
}

pub struct ImageProcessor {
    cdn_url: Option<String>,
}

impl ImageProcessor {
    pub fn new(cdn_url: Option<String>) -> Self {
        Self { cdn_url }
    }

    /// Process an image and generate metadata with srcset
    pub fn process_image(&self, src: &str, content_dir: &Path) -> Result<Option<ImageMetadata>> {
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

        if sizes.is_empty() {
            return Ok(None);
        }

        let (path_without_ext, ext) = self.parse_image_path(src);

        let srcset = self.generate_srcset(cdn_url, &path_without_ext, &ext, &sizes);
        let webp_srcset = self.generate_webp_srcset(cdn_url, &path_without_ext, &ext, &sizes);

        let default_size = sizes.last().copied().unwrap_or(860);
        let default_src = format!("{}{}.w{}.{}", cdn_url, path_without_ext, default_size, ext);

        Ok(Some(ImageMetadata {
            width,
            height,
            src: default_src,
            srcset,
            webp_srcset,
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
            let path_without_ext = &src[..dot_pos];
            let ext = &src[dot_pos + 1..];
            (path_without_ext.to_string(), ext.to_string())
        } else {
            (src.to_string(), "jpg".to_string())
        }
    }

    fn generate_srcset(&self, cdn_url: &str, path: &str, ext: &str, sizes: &[u32]) -> String {
        sizes
            .iter()
            .map(|&size| format!("{}{}.w{}.{} {}w", cdn_url, path, size, ext, size))
            .collect::<Vec<_>>()
            .join(", ")
    }

    fn generate_webp_srcset(&self, cdn_url: &str, path: &str, ext: &str, sizes: &[u32]) -> String {
        sizes
            .iter()
            .map(|&size| format!("{}{}.w{}.{}.webp {}w", cdn_url, path, size, ext, size))
            .collect::<Vec<_>>()
            .join(", ")
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
    fn test_generate_srcset() {
        let processor = ImageProcessor::new(Some("https://cdn.example.com".to_string()));

        let srcset = processor.generate_srcset(
            "https://cdn.example.com",
            "/images/photo",
            "jpg",
            &[480, 600, 860],
        );

        assert_eq!(
            srcset,
            "https://cdn.example.com/images/photo.w480.jpg 480w, \
             https://cdn.example.com/images/photo.w600.jpg 600w, \
             https://cdn.example.com/images/photo.w860.jpg 860w"
        );
    }

    #[test]
    fn test_generate_webp_srcset() {
        let processor = ImageProcessor::new(Some("https://cdn.example.com".to_string()));

        let srcset = processor.generate_webp_srcset(
            "https://cdn.example.com",
            "/images/photo",
            "jpg",
            &[480, 600],
        );

        assert_eq!(
            srcset,
            "https://cdn.example.com/images/photo.w480.jpg.webp 480w, \
             https://cdn.example.com/images/photo.w600.jpg.webp 600w"
        );
    }

    #[test]
    fn test_skip_external_images() {
        let processor = ImageProcessor::new(Some("https://cdn.example.com".to_string()));
        let content_dir = Path::new("content/posts/dev/test");

        let result = processor.process_image("https://example.com/image.jpg", content_dir);
        assert!(result.unwrap().is_none());
    }

    #[test]
    fn test_skip_without_cdn() {
        let processor = ImageProcessor::new(None);
        let content_dir = Path::new("content/posts/dev/test");

        let result = processor.process_image("./image.jpg", content_dir);
        assert!(result.unwrap().is_none());
    }
}
