use std::path::PathBuf;

use akibiyori::BuildOptions;
use clap::{Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(
    name = "akibiyori",
    version,
    about = "Build readable Cobalt sites from OpenAPI contracts and Markdown"
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
    /// Generate source, build with Cobalt, and index with Pagefind.
    Build {
        /// Generator config JSON path.
        #[arg(long)]
        config: PathBuf,
        /// Fresh or empty build-root directory.
        #[arg(long)]
        output: PathBuf,
        /// Normalized path prefix such as /docs.
        #[arg(long, default_value = "")]
        base_path: String,
        /// Cobalt executable or PATH-resolved command name.
        #[arg(long, default_value = "cobalt")]
        cobalt: PathBuf,
        /// Pagefind executable or PATH-resolved command name.
        #[arg(long, default_value = "pagefind")]
        pagefind: PathBuf,
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
        Command::Build {
            config,
            output,
            base_path,
            cobalt,
            pagefind,
        } => {
            let mut options = BuildOptions::new(config, output);
            options.base_path = base_path;
            options.cobalt = cobalt;
            options.pagefind = pagefind;
            match akibiyori::build(&options) {
                Ok(report) => {
                    println!(
                        "built {} authored pages, {} operation pages, {} model pages",
                        report.generation.authored_pages,
                        report.generation.operation_pages,
                        report.generation.model_pages
                    );
                    println!("Cobalt source: {}", report.source_dir.display());
                    println!("Site root: {}", report.site_root.display());
                    println!("Served directory: {}", report.served_dir.display());
                    for diagnostic in report.generation.diagnostics {
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
            }
        }
    }
}
