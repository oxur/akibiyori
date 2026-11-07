use std::path::PathBuf;

use akibiyori::{BuildOptions, ShowcaseOptions};
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
        /// Optional showcase manifest (requires --showcase-site).
        #[arg(long)]
        showcase_manifest: Option<PathBuf>,
        /// Explicit current site ID from the showcase manifest.
        #[arg(long)]
        showcase_site: Option<String>,
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
        /// Optional showcase manifest (requires --showcase-site).
        #[arg(long)]
        showcase_manifest: Option<PathBuf>,
        /// Explicit current site ID from the showcase manifest.
        #[arg(long)]
        showcase_site: Option<String>,
    },
}

fn main() -> std::process::ExitCode {
    let cli = Cli::parse();
    match cli.command {
        Command::Generate {
            config,
            output,
            base_path,
            showcase_manifest,
            showcase_site,
        } => match showcase_options(showcase_manifest, showcase_site) {
            Err(error) => {
                eprintln!("error: {error}");
                std::process::ExitCode::FAILURE
            }
            Ok(Some(showcase)) => {
                match akibiyori::generate_with_showcase(&config, &output, &base_path, &showcase) {
                    Ok(report) => print_generation(report, "generated"),
                    Err(error) => {
                        eprintln!("error: {error}");
                        std::process::ExitCode::FAILURE
                    }
                }
            }
            Ok(None) => match akibiyori::generate(&config, &output, &base_path) {
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
        },
        Command::Build {
            config,
            output,
            base_path,
            cobalt,
            pagefind,
            showcase_manifest,
            showcase_site,
        } => {
            let mut options = BuildOptions::new(config, output);
            options.base_path = base_path;
            options.cobalt = cobalt;
            options.pagefind = pagefind;
            let showcase = match showcase_options(showcase_manifest, showcase_site) {
                Ok(showcase) => showcase,
                Err(error) => {
                    eprintln!("error: {error}");
                    return std::process::ExitCode::FAILURE;
                }
            };
            let result = if let Some(showcase) = showcase.as_ref() {
                akibiyori::build_with_showcase(&options, showcase)
            } else {
                akibiyori::build(&options)
            };
            match result {
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

fn showcase_options(
    manifest: Option<PathBuf>,
    site: Option<String>,
) -> Result<Option<ShowcaseOptions>, String> {
    match (manifest, site) {
        (None, None) => Ok(None),
        (Some(manifest), Some(site)) => Ok(Some(ShowcaseOptions { manifest, site })),
        _ => Err("--showcase-manifest and --showcase-site must be provided together".to_owned()),
    }
}

fn print_generation(report: akibiyori::GenerationReport, verb: &str) -> std::process::ExitCode {
    println!(
        "{verb} {} authored pages, {} operation pages, {} model pages",
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
