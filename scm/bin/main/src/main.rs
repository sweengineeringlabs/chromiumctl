use std::env;
use std::process;

mod commands;
mod core;

use crate::core::help;

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        help::print_help();
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
        "set-files" => commands::file_selection::execute(cmd_args),
        "get-dom" => commands::dom_snapshot::execute(cmd_args),
        "metrics" => commands::metrics::execute(cmd_args),
        "stop" => commands::stop::execute(cmd_args),
        "reap" => commands::reap::execute(cmd_args),
        "mock" => commands::mock::execute(cmd_args),
        "help" | "-h" | "--help" => {
            help::print_help();
            Ok(())
        }
        "version" | "-V" | "--version" => {
            help::print_version();
            Ok(())
        }
        _ => {
            eprintln!("Unknown command: {}", command);
            help::print_help();
            process::exit(2);
        }
    };

    if let Err(e) = result {
        eprintln!("Error: {}", e);
        process::exit(e.exit_code());
    }
}
