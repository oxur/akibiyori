use std::path::PathBuf;

use clap::{Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(
    name = "akibiyori",
    version,
    about = "Generate readable Cobalt source from OpenAPI contracts"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Generate Cobalt Markdown, layouts, and data files from a JSON config.
    Generate {
        /// Generator config JSON path.
        #[arg(long)]
        config: PathBuf,
        /// Output source directory; it must be absent or empty.
        #[arg(long)]
        output: PathBuf,
        /// Normalized path prefix such as /docs.
        #[arg(long, default_value = "")]
        base_path: String,
    },
}

fn main() -> std::process::ExitCode {
    let cli = Cli::parse();
    match cli.command {
        Command::Generate {
            config,
            output,
            base_path,
        } => match akibiyori::generate(&config, &output, &base_path) {
            Ok(report) => {
                println!(
                    "generated {} authored pages, {} operation pages, {} model pages",
                    report.authored_pages, report.operation_pages, report.model_pages
                );
                for diagnostic in report.diagnostics {
                    eprintln!(
                        "warning: {} at {}: {}",
                        diagnostic.contract, diagnostic.pointer, diagnostic.message
                    );
                }
                std::process::ExitCode::SUCCESS
            }
            Err(error) => {
                eprintln!("error: {error}");
                std::process::ExitCode::FAILURE
            }
        },
    }
}
