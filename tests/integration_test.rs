/// Live-browser integration tests.
///
/// Requires a Chromium-based browser (Chrome, Edge, Brave) to be installed,
/// or `CHROME_PATH` set to the binary.
///
/// Run with:
///   cargo test -- --ignored --test-threads=1

use chromiumctl::{CdpClient, PageEvaluator};

/// Inline HTML fixture as a data: URL — no file-system access required.
fn fixture_url() -> String {
    let html = r#"<!DOCTYPE html>
<html>
<head>
<style>
.btn { display: inline-flex; padding: 8px 16px; border-radius: 6px; font-weight: 500; cursor: pointer; border: none; }
.btn--primary { background-color: %232563eb; color: white; }
.btn--danger  { background-color: %23dc2626; color: white; }
.card { background: white; border-radius: 8px; padding: 16px; }
.hidden { display: none; }
</style>
</head>
<body>
<button class="btn btn--primary" id="primary-btn">Primary</button>
<button class="btn btn--danger"  id="danger-btn">Danger</button>
<div class="card" id="card"><div class="card__header">Title</div></div>
<div class="hidden" id="hidden-el">Hidden</div>
</body>
</html>"#;
    format!("data:text/html,{}", html)
}

// ---------------------------------------------------------------------------
// evaluate
// ---------------------------------------------------------------------------

#[test]
#[ignore]
fn test_evaluate_string() {
    let c = CdpClient::launch(&fixture_url()).unwrap();
    assert_eq!(c.evaluate("'hello'").unwrap(), "hello");
}

#[test]
#[ignore]
fn test_evaluate_number() {
    let c = CdpClient::launch(&fixture_url()).unwrap();
    assert_eq!(c.evaluate("2 + 2").unwrap(), "4");
}

#[test]
#[ignore]
fn test_evaluate_boolean() {
    let c = CdpClient::launch(&fixture_url()).unwrap();
    assert_eq!(c.evaluate("1 === 1").unwrap(), "true");
}

#[test]
#[ignore]
fn test_evaluate_undefined_returns_empty() {
    let c = CdpClient::launch(&fixture_url()).unwrap();
    assert_eq!(c.evaluate("void(0)").unwrap(), "");
}

#[test]
#[ignore]
fn test_evaluate_dom_element_found() {
    let c = CdpClient::launch(&fixture_url()).unwrap();
    let r = c.evaluate("document.querySelector('.btn--primary') !== null ? 'found' : 'missing'").unwrap();
    assert_eq!(r, "found");
}

// ---------------------------------------------------------------------------
// get_computed_style
// ---------------------------------------------------------------------------

#[test]
#[ignore]
fn test_computed_style_reads_display() {
    let c = CdpClient::launch(&fixture_url()).unwrap();
    assert_eq!(c.get_computed_style(".btn", "display").unwrap(), "inline-flex");
}

#[test]
#[ignore]
fn test_computed_style_hidden_element_is_none() {
    let c = CdpClient::launch(&fixture_url()).unwrap();
    assert_eq!(c.get_computed_style(".hidden", "display").unwrap(), "none");
}

#[test]
#[ignore]
fn test_computed_style_missing_element_returns_err() {
    let c = CdpClient::launch(&fixture_url()).unwrap();
    let r = c.get_computed_style(".no-such-thing", "color");
    assert!(r.is_err());
    assert!(r.unwrap_err().contains("not found"));
}

// ---------------------------------------------------------------------------
// get_bounding_rect
// ---------------------------------------------------------------------------

#[test]
#[ignore]
fn test_bounding_rect_visible_element_has_size() {
    let c = CdpClient::launch(&fixture_url()).unwrap();
    let r = c.get_bounding_rect(".btn--primary").unwrap();
    assert!(r.width > 0.0);
    assert!(r.height > 0.0);
}

#[test]
#[ignore]
fn test_bounding_rect_missing_element_returns_err() {
    let c = CdpClient::launch(&fixture_url()).unwrap();
    assert!(c.get_bounding_rect(".no-such-thing").is_err());
}

// ---------------------------------------------------------------------------
// set_viewport_width / get_viewport_size
// ---------------------------------------------------------------------------

/// Core regression test: verifies Emulation.setDeviceMetricsOverride actually
/// changes the viewport (not just a fake JS property).
#[test]
#[ignore]
fn test_set_viewport_width_changes_actual_viewport() {
    let c = CdpClient::launch(&fixture_url()).unwrap();

    c.set_viewport_width(375).unwrap();
    assert_eq!(c.get_viewport_size().unwrap().0, 375);

    c.set_viewport_width(1280).unwrap();
    assert_eq!(c.get_viewport_size().unwrap().0, 1280);
}

#[test]
#[ignore]
fn test_viewport_width_affects_media_queries() {
    let c = CdpClient::launch(&fixture_url()).unwrap();

    c.evaluate(r#"
        var s = document.createElement('style');
        s.textContent = '.mq { color: red; } @media (min-width: 768px) { .mq { color: blue; } }';
        document.head.appendChild(s);
        var el = document.createElement('div');
        el.className = 'mq';
        document.body.appendChild(el);
    "#).unwrap();

    c.set_viewport_width(375).unwrap();
    let _ = c.evaluate("document.body.offsetHeight");
    let narrow = c.get_computed_style(".mq", "color").unwrap();

    c.set_viewport_width(1024).unwrap();
    let _ = c.evaluate("document.body.offsetHeight");
    let wide = c.get_computed_style(".mq", "color").unwrap();

    assert_ne!(narrow, wide, "color should differ across breakpoint: narrow={narrow} wide={wide}");
}

// ---------------------------------------------------------------------------
// navigate
// ---------------------------------------------------------------------------

#[test]
#[ignore]
fn test_navigate_changes_page_content() {
    let mut c = CdpClient::launch(&fixture_url()).unwrap();
    c.navigate("data:text/html,<h1 id=marker>navigated</h1>").unwrap();
    let found = c.evaluate("document.getElementById('marker') !== null ? 'yes' : 'no'").unwrap();
    assert_eq!(found, "yes");
}

// ---------------------------------------------------------------------------
// attach
// ---------------------------------------------------------------------------

#[test]
#[ignore]
fn test_attach_to_existing_browser() {
    let c1 = CdpClient::launch(&fixture_url()).unwrap();
    let c2 = CdpClient::attach(c1.port()).unwrap();
    assert_eq!(c2.evaluate("1 + 1").unwrap(), "2");
}
