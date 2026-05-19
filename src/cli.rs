//! Command-line interface implementation.

use anyhow::Result;
use clap::{Parser, Subcommand};
use serde_json::json;

use crate::models::BootEntry;
use crate::platform::get_boot_manager;

/// Cross-platform next-boot OS switcher
#[derive(Parser, Debug)]
#[command(name = "sys-switch", version, about)]
pub struct Cli {
    /// Run in CLI mode (no GUI)
    #[arg(long)]
    pub cli: bool,

    /// Show Windows Recovery Environment entries (Windows only)
    #[arg(long)]
    pub show_recovery: bool,

    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// List available boot entries
    List {
        /// Output format: text or json
        #[arg(short, long, default_value = "text")]
        output: String,
    },
    /// Set next boot entry (one-time)
    Set {
        /// Entry ID (Linux: 0000..; Windows: {GUID})
        id: String,
    },
    /// Reboot immediately
    Reboot,
}

/// Format boot entries for output.
fn format_entries(entries: &[BootEntry], format: &str) -> String {
    if format == "json" {
        let json_entries: Vec<_> = entries
            .iter()
            .map(|e| {
                json!({
                    "id": e.id,
                    "description": e.description,
                    "is_current": e.is_current,
                    "is_next": e.is_next,
                })
            })
            .collect();
        serde_json::to_string_pretty(&json_entries).unwrap_or_default()
    } else {
        let mut lines = vec!["ID\tCURRENT\tNEXT\tDESCRIPTION".to_string()];
        for e in entries {
            lines.push(format!(
                "{}\t{}\t{}\t{}",
                e.id,
                if e.is_current { "1" } else { "0" },
                if e.is_next { "1" } else { "0" },
                e.description
            ));
        }
        lines.join("\n")
    }
}

/// Run the CLI with parsed arguments.
pub fn run_cli(args: &Cli) -> Result<i32> {
    let manager = get_boot_manager();

    if !manager.available() {
        eprintln!("No supported boot manager found on this platform. Install required tools or run as admin/root.");
        return Ok(2);
    }

    match &args.command {
        None | Some(Commands::List { .. }) => {
            let entries = manager.list_entries()?;
            let format = if let Some(Commands::List { output }) = &args.command {
                output.as_str()
            } else {
                "text"
            };
            println!("{}", format_entries(&entries, format));
            Ok(0)
        }
        Some(Commands::Set { id }) => match manager.set_next(id) {
            Ok(msg) => {
                println!("{}", msg);
                Ok(0)
            }
            Err(e) => {
                eprintln!("{}", e);
                Ok(1)
            }
        },
        Some(Commands::Reboot) => match manager.reboot_now() {
            Ok(()) => Ok(0),
            Err(e) => {
                eprintln!("{}", e);
                Ok(1)
            }
        },
    }
}
