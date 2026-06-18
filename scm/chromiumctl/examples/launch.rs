use chromiumctl::{CdpClient, PageEvaluator};

fn main() {
    let client = CdpClient::launch("https://example.com")
        .expect("failed to launch browser — is Chrome/Edge/Brave installed?");

    let title = client.evaluate("document.title").unwrap_or_default();
    println!("title: {title}");

    let color = client
        .get_computed_style("body", "background-color")
        .unwrap_or_else(|_| "unknown".into());
    println!("body background-color: {color}");

    let (w, h) = client.get_viewport_size().unwrap_or((0, 0));
    println!("viewport: {w}x{h}");
}
