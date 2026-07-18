use crate::api::js::{deep_query_selector_js, js_string_literal};
use crate::api::Rect;

/// Evaluate JavaScript in a live browser page and query the DOM.
///
/// `CdpClient` implements this trait. You can also implement it yourself
/// to back the interface with a different transport (mock, remote proxy, etc.).
pub trait PageEvaluator {
    /// Evaluate a JavaScript expression and return its string result.
    fn evaluate(&self, js: &str) -> Result<String, String>;

    /// Get the computed CSS value for `property` on the first element
    /// matching `selector`. Selector resolution pierces into open shadow
    /// roots (see [`deep_query_selector_js`]); closed shadow roots remain
    /// unreachable.
    fn get_computed_style(&self, selector: &str, property: &str) -> Result<String, String> {
        let js = format!(
            r#"(function() {{
                {deep_query_selector}
                var el = __chromiumctl_deepQuerySelector(document, {selector});
                if (!el) return '__NOT_FOUND__';
                return window.getComputedStyle(el).getPropertyValue({property});
            }})()"#,
            deep_query_selector = deep_query_selector_js(),
            selector = js_string_literal(selector)?,
            property = js_string_literal(property)?,
        );
        match self.evaluate(&js)?.as_str() {
            "__NOT_FOUND__" => Err(format!("element not found: {}", selector)),
            other           => Ok(other.to_string()),
        }
    }

    /// Get the computed CSS value for `property` on a pseudo-element.
    /// Selector resolution pierces into open shadow roots; closed shadow
    /// roots remain unreachable.
    fn get_pseudo_style(
        &self,
        selector: &str,
        pseudo:   &str,
        property: &str,
    ) -> Result<String, String> {
        let js = format!(
            r#"(function() {{
                {deep_query_selector}
                var el = __chromiumctl_deepQuerySelector(document, {selector});
                if (!el) return '__NOT_FOUND__';
                return window.getComputedStyle(el, {pseudo}).getPropertyValue({property});
            }})()"#,
            deep_query_selector = deep_query_selector_js(),
            selector = js_string_literal(selector)?,
            pseudo = js_string_literal(pseudo)?,
            property = js_string_literal(property)?,
        );
        match self.evaluate(&js)?.as_str() {
            "__NOT_FOUND__" => Err(format!("element not found: {}", selector)),
            other           => Ok(other.to_string()),
        }
    }

    /// Get the bounding rect of the first element matching `selector`.
    /// Selector resolution pierces into open shadow roots; closed shadow
    /// roots remain unreachable.
    fn get_bounding_rect(&self, selector: &str) -> Result<Rect, String> {
        let js = format!(
            r#"(function() {{
                {deep_query_selector}
                var el = __chromiumctl_deepQuerySelector(document, {selector});
                if (!el) return '__NOT_FOUND__';
                var r = el.getBoundingClientRect();
                return JSON.stringify({{ x: r.x, y: r.y, width: r.width, height: r.height }});
            }})()"#,
            deep_query_selector = deep_query_selector_js(),
            selector = js_string_literal(selector)?,
        );
        let raw = self.evaluate(&js)?;
        if raw == "__NOT_FOUND__" {
            return Err(format!("element not found: {}", selector));
        }
        serde_json::from_str(&raw).map_err(|e| format!("failed to parse rect: {}", e))
    }

    /// Set the viewport width via `Emulation.setDeviceMetricsOverride`.
    fn set_viewport_width(&self, width: u32) -> Result<(), String>;

    /// Get the current viewport dimensions as `(width, height)`.
    fn get_viewport_size(&self) -> Result<(u32, u32), String> {
        let raw = self.evaluate("JSON.stringify({ w: window.innerWidth, h: window.innerHeight })")?;
        let v: serde_json::Value = serde_json::from_str(&raw).map_err(|e| e.to_string())?;
        Ok((
            v["w"].as_u64().unwrap_or(0) as u32,
            v["h"].as_u64().unwrap_or(0) as u32,
        ))
    }
}
