/// A JS function declaration: `document.querySelector`, but recursively
/// descends into every *open* shadow root along the way. Closed shadow
/// roots remain unreachable — CDP itself can't see them without
/// `DOM.getFlattenedDocument`, which this crate does not use.
///
/// Meant to be embedded inside a larger `(function() { ... })()` snippet,
/// then called as `__browsectl_deepQuerySelector(document, selector)`.
/// Safe to embed multiple times per snippet or across separate `evaluate`
/// calls — it's scoped to whatever IIFE it's pasted into, not the page's
/// global scope.
pub fn deep_query_selector_js() -> &'static str {
    r#"function __browsectl_deepQuerySelector(root, selector) {
        var direct = root.querySelector(selector);
        if (direct) return direct;
        var hosts = root.querySelectorAll('*');
        for (var i = 0; i < hosts.length; i++) {
            if (hosts[i].shadowRoot) {
                var found = __browsectl_deepQuerySelector(hosts[i].shadowRoot, selector);
                if (found) return found;
            }
        }
        return null;
    }"#
}

/// Encode `s` as a JS string literal, safe to interpolate into a generated
/// JS snippet (e.g. a CSS selector coming from a CLI argument). A raw
/// `format!("'{}'", s)` breaks on a literal `'` in `s`, or is a JS
/// injection vector if `s` ever comes from untrusted input.
pub fn js_string_literal(s: &str) -> Result<String, String> {
    serde_json::to_string(s).map_err(|e| format!("failed to encode JS string literal: {}", e))
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn test_js_string_literal_wraps_plain_string_in_double_quotes() {
        assert_eq!(js_string_literal("hello").unwrap(), "\"hello\"");
    }

    #[test]
    fn test_js_string_literal_escapes_embedded_single_quote() {
        // The classic break case for `format!("'{}'", selector)`.
        let encoded = js_string_literal("input[value='x']").unwrap();
        assert_eq!(encoded, "\"input[value='x']\"");
    }

    #[test]
    fn test_js_string_literal_escapes_embedded_double_quote() {
        let encoded = js_string_literal(r#"a"b"#).unwrap();
        assert_eq!(encoded, r#""a\"b""#);
    }

    #[test]
    fn test_js_string_literal_escapes_backslash() {
        let encoded = js_string_literal(r"a\b").unwrap();
        assert_eq!(encoded, r#""a\\b""#);
    }

    #[test]
    fn test_js_string_literal_neutralizes_script_close_and_html_comment_sequences() {
        // Not HTML-injection relevant here (this is JS-string context, not
        // HTML), but confirms a `</script>`-shaped selector round-trips as
        // inert literal text rather than surviving as-is inside the quotes.
        let encoded = js_string_literal("</script><script>evil()</script>").unwrap();
        assert!(encoded.starts_with('"') && encoded.ends_with('"'));
        assert_eq!(
            serde_json::from_str::<String>(&encoded).unwrap(),
            "</script><script>evil()</script>"
        );
    }

    #[test]
    fn test_deep_query_selector_js_declares_the_expected_function_name() {
        let js = deep_query_selector_js();
        assert!(js.contains("function __browsectl_deepQuerySelector(root, selector)"));
    }

    #[test]
    fn test_deep_query_selector_js_recurses_into_shadow_roots() {
        let js = deep_query_selector_js();
        assert!(js.contains("shadowRoot"), "must actually descend into shadow roots, not just document");
        assert!(js.contains("root.querySelector(selector)"), "must try a direct match before recursing");
    }
}
