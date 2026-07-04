// E2e tests for PageEvaluator trait methods exercised via CdpClient.
//
// All tests require a Chromium-based browser. Run with:
//   cargo test -- --ignored --test-threads=1
#![allow(clippy::unwrap_used, clippy::expect_used)]

use chromiumctl::{CdpClient, PageEvaluator};

fn fixture_url() -> &'static str {
    "data:text/html,<html><head><style>.box{width:100px;height:50px;background:red}</style></head>\
     <body><div class='box' id='b'>text</div></body></html>"
}

#[test]
#[ignore]
fn test_page_evaluator_evaluate_returns_string() {
    let c = CdpClient::launch(fixture_url()).unwrap();
    assert_eq!(c.evaluate("'ok'").unwrap(), "ok");
}

#[test]
#[ignore]
fn test_page_evaluator_get_computed_style_reads_property() {
    let c = CdpClient::launch(fixture_url()).unwrap();
    let bg = c.get_computed_style(".box", "background-color").unwrap();
    assert!(!bg.is_empty());
}

#[test]
#[ignore]
fn test_page_evaluator_get_computed_style_missing_selector_returns_err() {
    let c = CdpClient::launch(fixture_url()).unwrap();
    assert!(c.get_computed_style(".no-such", "color").is_err());
}

#[test]
#[ignore]
fn test_page_evaluator_get_bounding_rect_has_nonzero_dimensions() {
    let c = CdpClient::launch(fixture_url()).unwrap();
    let r = c.get_bounding_rect(".box").unwrap();
    assert!(r.width > 0.0 && r.height > 0.0);
}

#[test]
#[ignore]
fn test_page_evaluator_set_viewport_width_changes_width() {
    let c = CdpClient::launch(fixture_url()).unwrap();
    c.set_viewport_width(480).unwrap();
    assert_eq!(c.get_viewport_size().unwrap().0, 480);
}
