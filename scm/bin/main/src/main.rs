use std::env;
use std::process;

mod commands;
mod os_process;
mod session;

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
        "set-files" => commands::set_files::execute(cmd_args),
        "get-dom" => commands::get_dom::execute(cmd_args),
        "metrics" => commands::metrics::execute(cmd_args),
        "stop" => commands::stop::execute(cmd_args),
        "reap" => commands::reap::execute(cmd_args),
        "mock" => commands::mock::execute(cmd_args),
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
    eprintln!("    set-files    Set files on an <input type=\"file\"> element");
    eprintln!("    get-dom      Export current DOM as JSON");
    eprintln!("    metrics      Get performance metrics");
    eprintln!("    stop         Terminate exactly the browser session at --port/--package");
    eprintln!("    reap         Clean up sessions whose launch caller has died or gone stale");
    eprintln!("    mock         Intercept matching requests with a fake response (blocks until Ctrl-C)");
    eprintln!("    help         Print this message\n");
    eprintln!("OPTIONS:\n");
    eprintln!("    --url <URL>           Target URL (launch, navigate)");
    eprintln!("    --port <PORT>         Debug port (default: 9222)");
    eprintln!("    --package <PKG>       Attach to a debuggable Android WebView via adb, instead");
    eprintln!("                          of --port (eval, navigate, wait, click, input, screenshot,");
    eprintln!("                          get-dom, metrics; requires the `android` build feature)");
    eprintln!("    --script <JS>         JavaScript to evaluate (eval)");
    eprintln!("    --selector <SEL>      CSS selector (wait, click, input, set-files)");
    eprintln!("    --text <TEXT>         Text to match (wait) or type (input)");
    eprintln!("    --files <PATHS>       Comma-separated file paths to set (set-files)");
    eprintln!("    --navigation          Wait for document.readyState to complete (wait)");
    eprintln!("    --width <PX>          Viewport width (launch; default: 1920)");
    eprintln!("    --height <PX>         Viewport height (launch; default: 1080)");
    eprintln!("    --output <PATH>       Output format text|json|yaml (eval) or file path (screenshot, get-dom, metrics)");
    eprintln!("    --format <FMT>        Image format: png, jpeg, webp (screenshot; default: png)");
    eprintln!("    --full-page           Capture beyond the viewport (screenshot)");
    eprintln!("    --timeout <SECS>      Operation timeout in seconds (wait; default: 30)");
    eprintln!("    --headless            Accepted for compatibility; Chromium always runs headless");
    eprintln!("    --reap-stale          Before launching, reap other sessions whose caller has died (launch)");
    eprintln!("    --dry-run             List what reap would do without closing/deleting anything (reap)");
    eprintln!("    --max-age <DUR>       Also reap sessions older than this even if their caller is alive");
    eprintln!("                          (reap; e.g. 30, 30s, 5m, 1h)");
    eprintln!("    --url-pattern <PAT>   Glob pattern of request URLs to intercept (mock; e.g. \"*api.example.com*\")");
    eprintln!("    --status <CODE>       Fake HTTP status code to respond with (mock; default: 200)");
    eprintln!("    --body <TEXT>         Fake response body (mock; default: empty)\n");
    eprintln!("EXAMPLES:\n");
    eprintln!("    chromiumctl launch --url https://example.com --port 9222\n");
    eprintln!("    chromiumctl eval --port 9222 --script \"document.title\"\n");
    eprintln!("    chromiumctl eval --package com.example.app --script \"document.title\"\n");
    eprintln!("    chromiumctl screenshot --port 9222 --output page.png\n");
    eprintln!("    chromiumctl stop --port 9222\n");
    eprintln!("    chromiumctl reap --dry-run\n");
    eprintln!("    chromiumctl reap --max-age 1h\n");
    eprintln!("    chromiumctl set-files --port 9222 --selector \"#file-input\" --files \"./a.png,./b.pdf\"\n");
    eprintln!("    chromiumctl mock --port 9222 --url-pattern \"*api.example.com*\" --status 200 --body '{{\"ok\":true}}'\n");
}
