use xmux_core::XmuxError;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct RenderedMath {
    pub pixels: Vec<u8>,  // RGBA
    pub width: u32,
    pub height: u32,
}

pub struct MathRenderer {
    cache: HashMap<String, RenderedMath>,
}

impl MathRenderer {
    pub fn new() -> Self {
        Self {
            cache: HashMap::new(),
        }
    }

    pub fn render(&mut self, latex: &str, font_size: f32, fg_color: [u8; 4]) -> Result<RenderedMath, XmuxError> {
        // Check cache
        if let Some(cached) = self.cache.get(latex) {
            return Ok(cached.clone());
        }

        // Stub renderer: create a small placeholder image
        // Real implementation would use ratex-render or katex-rs + resvg
        let width = (latex.len() as u32 * font_size as u32 / 2).max(1);
        let height = (font_size * 1.5) as u32;
        let mut pixels = vec![0u8; (width * height * 4) as usize];

        // Fill with fg_color at 50% opacity as placeholder
        for pixel in pixels.chunks_exact_mut(4) {
            pixel[0] = fg_color[0];
            pixel[1] = fg_color[1];
            pixel[2] = fg_color[2];
            pixel[3] = fg_color[3] / 4;
        }

        let result = RenderedMath { pixels, width, height };
        self.cache.insert(latex.to_string(), result.clone());
        Ok(result)
    }

    pub fn cache_size(&self) -> usize {
        self.cache.len()
    }

    pub fn clear_cache(&mut self) {
        self.cache.clear();
    }
}

impl Default for MathRenderer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_render_stub() {
        let mut renderer = MathRenderer::new();
        let result = renderer.render("E=mc^2", 12.0, [255, 255, 255, 255]).unwrap();
        assert!(result.width > 0);
        assert!(result.height > 0);
        assert!(!result.pixels.is_empty());
        assert_eq!(result.pixels.len(), (result.width * result.height * 4) as usize);
    }

    #[test]
    fn test_cache() {
        let mut renderer = MathRenderer::new();
        let expr = "E=mc^2";
        let color = [255, 255, 255, 255];

        renderer.render(expr, 12.0, color).unwrap();
        assert_eq!(renderer.cache_size(), 1);

        renderer.render(expr, 12.0, color).unwrap();
        assert_eq!(renderer.cache_size(), 1);
    }

    #[test]
    fn test_clear_cache() {
        let mut renderer = MathRenderer::new();
        renderer.render("E=mc^2", 12.0, [255, 255, 255, 255]).unwrap();
        assert_eq!(renderer.cache_size(), 1);

        renderer.clear_cache();
        assert_eq!(renderer.cache_size(), 0);
    }
}
