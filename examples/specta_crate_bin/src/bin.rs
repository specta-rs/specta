#[cfg(feature = "cli")]
use clap::Parser;
#[cfg(feature = "cli")]
use specta::export;

// Example usage:
// cargo run --features cli -- generate-ts-types --export-path=./bindings.ts

// Import all types and modules from `specta_crate_bin` to make them accessible to specta here
#[cfg(feature = "cli")]
use specta_crate_bin::*;

// Root CLI argument structure
#[cfg(feature = "cli")]
#[derive(Parser, Debug)]
#[clap(name = "cli")]
struct Cli {
    #[clap(subcommand)]
    pub cmd: SubCommand,
}

// Enum for available subcommands
#[cfg(feature = "cli")]
#[derive(Parser, Debug)]
enum SubCommand {
    GenerateTsTypes(GenerateTsTypes),
}

// Subcommand for generating TypeScript types
#[cfg(feature = "cli")]
#[derive(Parser, Debug)]
struct GenerateTsTypes {
    #[clap(long)]
    export_path: String,
}

#[cfg(feature = "cli")]
fn main() {
    let args = Cli::parse();
    match args.cmd {
        SubCommand::GenerateTsTypes(sub_args) => {
            generate_ts_types(&sub_args.export_path);
        }
    }
}

#[cfg(feature = "cli")]
fn generate_ts_types(export_path: &str) {
    match export::ts(export_path) {
        Ok(_) => println!("Successfully generated TypeScript types at {}", export_path),
        Err(e) => eprintln!("Failed to generate TypeScript types: {:?}", e),
    }
}
