#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod cli;
mod elevation;
mod gui;
mod models;
mod platform;

use clap::Parser;

fn main() {
    // Parse CLI arguments
    let args = cli::Cli::parse();

    // If CLI mode or subcommand present, attach console and run CLI
    if args.cli || args.command.is_some() {
        #[cfg(target_os = "windows")]
        unsafe {
            use windows::Win32::System::Console::{AttachConsole, ATTACH_PARENT_PROCESS};
            let _ = AttachConsole(ATTACH_PARENT_PROCESS);
        }

        let code = cli::run_cli(&args).unwrap_or_else(|e| {
            eprintln!("Error: {}", e);
            1
        });
        std::process::exit(code);
    }

    // GUI mode: check for elevation
    match elevation::elevate_if_needed(true) {
        Ok(true) => {
            // Elevated instance launched, exit current
            return;
        }
        Ok(false) => {
            // Already elevated or not needed, continue
        }
        Err(e) => {
            eprintln!("Warning: elevation check failed: {}", e);
            // Continue anyway, operations may fail later
        }
    }

    // Run GUI
    if let Err(e) = gui::run_gui() {
        eprintln!("GUI error: {}", e);
        std::process::exit(1);
    }
}
