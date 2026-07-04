use std::env;
use std::process;

mod commands;

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        print_help();
        process::exit(2);
    }

    let command = &args[1];
    let cmd_args = &args[2..];

    let result = match command.as_str() {
        "launch" => commands::launch::execute(cmd_args),
        "eval" => commands::eval::execute(cmd_args),
        "screenshot" => commands::screenshot::execute(cmd_args),
        "navigate" => commands::navigate::execute(cmd_args),
        "wait" => commands::wait::execute(cmd_args),
        "click" => commands::click::execute(cmd_args),
        "input" => commands::input::execute(cmd_args),
        "get-dom" => commands::get_dom::execute(cmd_args),
        "metrics" => commands::metrics::execute(cmd_args),
        "help" | "-h" | "--help" => {
            print_help();
            Ok(())
        }
        _ => {
            eprintln!("Unknown command: {}", command);
            print_help();
            process::exit(2);
        }
    };

    if let Err(e) = result {
        eprintln!("Error: {}", e);
        process::exit(e.exit_code());
    }
}

fn print_help() {
    eprintln!("chromiumctl — Chromium DevTools Protocol CLI\n");
    eprintln!("USAGE:\n");
    eprintln!("    chromiumctl <COMMAND> [OPTIONS]\n");
    eprintln!("COMMANDS:\n");
    eprintln!("    launch       Launch headless browser and keep alive");
    eprintln!("    eval         Evaluate JavaScript in running session");
    eprintln!("    screenshot   Capture page screenshot");
    eprintln!("    navigate     Navigate to URL in running session");
    eprintln!("    wait         Wait for condition (selector, text, navigation)");
    eprintln!("    click        Click element on page");
    eprintln!("    input        Type text into input field");
    eprintln!("    get-dom      Export current DOM as JSON");
    eprintln!("    metrics      Get performance metrics");
    eprintln!("    help         Print this message\n");
    eprintln!("OPTIONS:\n");
    eprintln!("    --url <URL>           Target URL (launch, navigate)");
    eprintln!("    --port <PORT>         Debug port (default: 9222)");
    eprintln!("    --script <JS>         JavaScript to evaluate (eval)");
    eprintln!("    --selector <SEL>      CSS selector (wait, click, input)");
    eprintln!("    --text <TEXT>         Text to match (wait) or type (input)");
    eprintln!("    --width <PX>          Viewport width (launch; default: 1920)");
    eprintln!("    --height <PX>         Viewport height (launch; default: 1080)");
    eprintln!("    --output <PATH>       Output format text|json|yaml (eval) or file path (screenshot, get-dom, metrics)");
    eprintln!("    --format <FMT>        Image format: png, jpeg, webp (screenshot; default: png)");
    eprintln!("    --full-page           Capture beyond the viewport (screenshot)");
    eprintln!("    --timeout <SECS>      Operation timeout in seconds (wait; default: 30)");
    eprintln!("    --headless            Accepted for compatibility; Chromium always runs headless\n");
    eprintln!("EXAMPLES:\n");
    eprintln!("    chromiumctl launch --url https://example.com --port 9222\n");
    eprintln!("    chromiumctl eval --port 9222 --script \"document.title\"\n");
    eprintln!("    chromiumctl screenshot --port 9222 --output page.png\n");
}
