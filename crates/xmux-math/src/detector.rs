#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MathDelimiter {
    InlineDollar,      // $...$
    DisplayDollar,     // $$...$$
    OscIterm,          // OSC 1337;LaTeX=<base64>
}

#[derive(Debug, Clone)]
pub struct MathMatch {
    pub latex: String,
    pub delimiter: MathDelimiter,
    pub start: usize,
    pub end: usize,
}

pub struct MathDetector;

impl MathDetector {
    pub fn find_math(text: &str) -> Vec<MathMatch> {
        let mut matches = Vec::new();
        let bytes = text.as_bytes();
        let len = bytes.len();
        let mut i = 0;

        while i < len {
            // Check for $$ (display math) first
            if i + 1 < len && bytes[i] == b'$' && bytes[i + 1] == b'$' {
                let start = i;
                i += 2;
                let content_start = i;
                // Find closing $$
                while i + 1 < len && !(bytes[i] == b'$' && bytes[i + 1] == b'$') {
                    i += 1;
                }
                if i + 1 < len {
                    let latex = String::from_utf8_lossy(&bytes[content_start..i]).to_string();
                    if !latex.is_empty() {
                        matches.push(MathMatch {
                            latex,
                            delimiter: MathDelimiter::DisplayDollar,
                            start,
                            end: i + 2,
                        });
                    }
                    i += 2;
                } else {
                    i = start + 1; // backtrack, not a valid display math
                }
            }
            // Check for $ (inline math)
            else if bytes[i] == b'$' {
                let start = i;
                i += 1;
                let content_start = i;
                // Skip if next char is space or $ (not valid inline math start)
                if i < len && bytes[i] != b' ' && bytes[i] != b'$' {
                    // Find closing $ (not preceded by backslash)
                    while i < len && bytes[i] != b'$' {
                        if bytes[i] == b'\\' && i + 1 < len {
                            i += 2; // skip escaped char
                        } else {
                            i += 1;
                        }
                    }
                    if i < len && i > content_start {
                        // Check that char before closing $ is not space
                        if bytes[i - 1] != b' ' {
                            let latex = String::from_utf8_lossy(&bytes[content_start..i]).to_string();
                            matches.push(MathMatch {
                                latex,
                                delimiter: MathDelimiter::InlineDollar,
                                start,
                                end: i + 1,
                            });
                        }
                    }
                    if i < len { i += 1; }
                }
            } else {
                i += 1;
            }
        }
        matches
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_inline_math() {
        let text = "$E=mc^2$";
        let matches = MathDetector::find_math(text);
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].latex, "E=mc^2");
        assert_eq!(matches[0].delimiter, MathDelimiter::InlineDollar);
        assert_eq!(matches[0].start, 0);
        assert_eq!(matches[0].end, 8);
    }

    #[test]
    fn test_display_math() {
        let text = "$$\\int_0^1 f(x) dx$$";
        let matches = MathDetector::find_math(text);
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].latex, "\\int_0^1 f(x) dx");
        assert_eq!(matches[0].delimiter, MathDelimiter::DisplayDollar);
    }

    #[test]
    fn test_mixed_text() {
        let text = "The formula $a+b$ equals $c$";
        let matches = MathDetector::find_math(text);
        assert_eq!(matches.len(), 2);
        assert_eq!(matches[0].latex, "a+b");
        assert_eq!(matches[1].latex, "c");
    }

    #[test]
    fn test_no_math() {
        let text = "just regular text with no dollar signs here";
        let matches = MathDetector::find_math(text);
        assert_eq!(matches.len(), 0);
    }

    #[test]
    fn test_empty_math() {
        let text = "$$$$";
        let matches = MathDetector::find_math(text);
        assert_eq!(matches.len(), 0);
    }

    #[test]
    fn test_display_math_with_content() {
        let text = "text $$x^2$$ more";
        let matches = MathDetector::find_math(text);
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].latex, "x^2");
        assert_eq!(matches[0].delimiter, MathDelimiter::DisplayDollar);
    }

    #[test]
    fn test_dollar_with_number() {
        // "$5 price" should not match because $ followed by digit is ambiguous
        // and closing $ is needed
        let text = "cost is $5";
        let matches = MathDetector::find_math(text);
        assert_eq!(matches.len(), 0);
    }
}
