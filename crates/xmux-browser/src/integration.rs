pub struct UrlDetector;

impl UrlDetector {
    pub fn find_urls(text: &str) -> Vec<UrlMatch> {
        let mut urls = Vec::new();
        let mut i = 0;
        let bytes = text.as_bytes();
        while i < bytes.len() {
            // Look for http:// or https://
            if i + 7 < bytes.len() && &bytes[i..i+7] == b"http://" {
                let start = i;
                // Find end of URL (whitespace, quotes, parens, or end of string)
                i += 7;
                while i < bytes.len() && !matches!(bytes[i], b' ' | b'\t' | b'\n' | b'\r' | b'"' | b'\'' | b'>' | b')') {
                    i += 1;
                }
                // Remove trailing punctuation
                while i > start && matches!(bytes[i-1], b'.' | b',' | b';' | b':') {
                    i -= 1;
                }
                if i > start + 7 {
                    if let Ok(url) = std::str::from_utf8(&bytes[start..i]) {
                        urls.push(UrlMatch {
                            url: url.to_string(),
                            start,
                            end: i,
                        });
                    }
                }
            } else if i + 8 <= bytes.len() && &bytes[i..i+8] == b"https://" {
                let start = i;
                // Find end of URL (whitespace, quotes, parens, or end of string)
                i += 8;
                while i < bytes.len() && !matches!(bytes[i], b' ' | b'\t' | b'\n' | b'\r' | b'"' | b'\'' | b'>' | b')') {
                    i += 1;
                }
                // Remove trailing punctuation
                while i > start && matches!(bytes[i-1], b'.' | b',' | b';' | b':') {
                    i -= 1;
                }
                if i > start + 8 {
                    if let Ok(url) = std::str::from_utf8(&bytes[start..i]) {
                        urls.push(UrlMatch {
                            url: url.to_string(),
                            start,
                            end: i,
                        });
                    }
                }
            } else {
                i += 1;
            }
        }
        urls
    }
}

#[derive(Debug, Clone)]
pub struct UrlMatch {
    pub url: String,
    pub start: usize,
    pub end: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_http_url() {
        let text = "visit http://example.com for info";
        let urls = UrlDetector::find_urls(text);
        assert_eq!(urls.len(), 1);
        assert_eq!(urls[0].url, "http://example.com");
    }

    #[test]
    fn test_find_https_url() {
        let text = "see https://github.com/foo/bar";
        let urls = UrlDetector::find_urls(text);
        assert_eq!(urls.len(), 1);
        assert_eq!(urls[0].url, "https://github.com/foo/bar");
    }

    #[test]
    fn test_find_multiple_urls() {
        let text = "visit http://example.com and https://github.com for more";
        let urls = UrlDetector::find_urls(text);
        assert_eq!(urls.len(), 2);
        assert_eq!(urls[0].url, "http://example.com");
        assert_eq!(urls[1].url, "https://github.com");
    }

    #[test]
    fn test_no_urls() {
        let text = "just plain text";
        let urls = UrlDetector::find_urls(text);
        assert_eq!(urls.len(), 0);
    }

    #[test]
    fn test_url_with_trailing_punctuation() {
        let text = "check http://example.com.";
        let urls = UrlDetector::find_urls(text);
        assert_eq!(urls.len(), 1);
        assert_eq!(urls[0].url, "http://example.com");
    }

    #[test]
    fn test_localhost_port() {
        let text = "running at http://localhost:3000/api";
        let urls = UrlDetector::find_urls(text);
        assert_eq!(urls.len(), 1);
        assert_eq!(urls[0].url, "http://localhost:3000/api");
    }
}
