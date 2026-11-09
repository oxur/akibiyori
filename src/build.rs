use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use crate::config::{check_output, validate_base_path};
use crate::{Error, GenerationOptions, GenerationReport, ShowcaseOptions, generate_with_options};

/// Inputs for a complete source-generation and native-site build.
#[derive(Debug, Clone)]
pub struct BuildOptions {
    /// JSON site configuration, resolved from the caller's working directory.
    pub config: PathBuf,
    /// Absent or empty output root owned by this build.
    pub output: PathBuf,
    /// Normalized URL prefix, such as `/docs`, or empty for a root build.
    pub base_path: String,
    /// Cobalt executable or PATH-resolved command name.
    pub cobalt: PathBuf,
    /// Pagefind executable or PATH-resolved command name.
    pub pagefind: PathBuf,
}

impl BuildOptions {
    /// Construct build inputs with the native tool names resolved through PATH.
    pub fn new(config: impl Into<PathBuf>, output: impl Into<PathBuf>) -> Self {
        Self {
            config: config.into(),
            output: output.into(),
            base_path: String::new(),
            cobalt: PathBuf::from("cobalt"),
            pagefind: PathBuf::from("pagefind"),
        }
    }
}

/// Locations and generation counts for a completed native build.
#[derive(Debug, Clone)]
pub struct BuildReport {
    /// Counts and non-fatal documentation diagnostics.
    pub generation: GenerationReport,
    /// Generated Markdown, layouts and configuration.
    pub source_dir: PathBuf,
    /// Static server root, before any URL prefix subdirectory.
    pub site_root: PathBuf,
    /// Directory containing the served pages and Pagefind bundle.
    pub served_dir: PathBuf,
}

/// Generate Cobalt source, then build and index it with real native tools.
///
/// Both tools are version-checked before generation writes anything. The output
/// root must be absent or an empty real directory. A later Cobalt or Pagefind
/// failure can leave partial generated output; this function does not roll it
/// back or delete caller data.
pub fn build(options: &BuildOptions) -> Result<BuildReport, Error> {
    build_with_options(options, &GenerationOptions::default())
}

/// Generate, build, and index a site with an opt-in showcase context.
pub fn build_with_showcase(
    options: &BuildOptions,
    showcase: &ShowcaseOptions,
) -> Result<BuildReport, Error> {
    build_with_options(
        options,
        &GenerationOptions::default().with_showcase(showcase.clone()),
    )
}

/// Generate, build, and index a site with reusable generation options.
///
/// The optional presentation override has the same precedence as
/// [`crate::generate_with_options`]; showcase context composes with it.
///
/// # Errors
/// Returns [`Error`] when inputs, theme selection, output state or native tool
/// execution is invalid. Validation and tool-version failures occur before
/// source generation; later native failures may leave partial output.
pub fn build_with_options(
    options: &BuildOptions,
    generation_options: &GenerationOptions,
) -> Result<BuildReport, Error> {
    validate_base_path(&options.base_path)?;
    check_output(&options.output)?;
    let caller_dir = std::env::current_dir().map_err(|source| Error::NativeSpawn {
        program: "current directory".to_owned(),
        stage: "resolve build paths",
        source,
    })?;
    let build_root = absolute_path(&caller_dir, &options.output);
    let source_dir = build_root.join("source");
    let site_root = build_root.join("site");
    let served_dir = if options.base_path.is_empty() {
        site_root.clone()
    } else {
        site_root.join(options.base_path.trim_start_matches('/'))
    };

    let cobalt = resolve_program(&caller_dir, &options.cobalt, "Cobalt version")?;
    let pagefind = resolve_program(&caller_dir, &options.pagefind, "Pagefind version")?;
    verify_version(&cobalt, "cobalt-bin 0.20.4", "Cobalt version")?;
    verify_version(&pagefind, "pagefind 1.5.2", "Pagefind version")?;

    let generation = generate_with_options(
        &options.config,
        &source_dir,
        &options.base_path,
        generation_options,
    )?;
    run_native(
        &cobalt,
        "Cobalt build",
        &source_dir,
        &["build", "--config", "_cobalt.yml", "--destination"],
        Some(&served_dir),
    )?;
    run_native(
        &pagefind,
        "Pagefind index",
        &caller_dir,
        &["--site"],
        Some(&served_dir),
    )?;

    Ok(BuildReport {
        generation,
        source_dir,
        site_root,
        served_dir,
    })
}

fn absolute_path(caller_dir: &Path, path: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        caller_dir.join(path)
    }
}

fn resolve_program(
    caller_dir: &Path,
    program: &Path,
    stage: &'static str,
) -> Result<PathBuf, Error> {
    let is_path = program.is_absolute() || program.components().count() > 1;
    if !is_path {
        return Ok(program.to_path_buf());
    }
    let path = absolute_path(caller_dir, program);
    path.canonicalize().map_err(|source| Error::NativeSpawn {
        program: program.display().to_string(),
        stage,
        source,
    })
}

fn verify_version(
    program: &Path,
    expected: &'static str,
    stage: &'static str,
) -> Result<(), Error> {
    let output = Command::new(program)
        .arg("--version")
        .output()
        .map_err(|source| Error::NativeSpawn {
            program: program.display().to_string(),
            stage,
            source,
        })?;
    ensure_success(program, stage, &output)?;
    let actual = output_text(&output);
    if actual.trim() != expected {
        return Err(Error::NativeVersion {
            program: program.display().to_string(),
            stage,
            expected,
            actual,
        });
    }
    Ok(())
}

fn run_native(
    program: &Path,
    stage: &'static str,
    current_dir: &Path,
    leading_args: &[&str],
    final_path: Option<&Path>,
) -> Result<(), Error> {
    let mut command = Command::new(program);
    command.args(leading_args).current_dir(current_dir);
    if let Some(path) = final_path {
        command.arg(path);
    }
    let output = command.output().map_err(|source| Error::NativeSpawn {
        program: program.display().to_string(),
        stage,
        source,
    })?;
    ensure_success(program, stage, &output)
}

fn ensure_success(program: &Path, stage: &'static str, output: &Output) -> Result<(), Error> {
    if output.status.success() {
        return Ok(());
    }
    Err(Error::NativeStatus {
        program: program.display().to_string(),
        stage,
        status: output.status.code().map_or_else(
            || "terminated by signal".to_owned(),
            |code| code.to_string(),
        ),
        stderr: String::from_utf8_lossy(&output.stderr).trim().to_owned(),
    })
}

fn output_text(output: &Output) -> String {
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_owned();
    if stdout.is_empty() {
        String::from_utf8_lossy(&output.stderr).trim().to_owned()
    } else {
        stdout
    }
}

#[cfg(test)]
mod tests {
    use super::{resolve_program, run_native, verify_version};
    use crate::Error;
    use std::path::PathBuf;

    #[test]
    fn actual_cobalt_nonzero_exit_is_reported_as_native_status() {
        let temp = tempfile::tempdir().expect("temporary Cobalt working directory");
        let caller_dir = std::env::current_dir().expect("caller directory");
        let configured = std::env::var_os("COBALT")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("cobalt"));
        let cobalt = resolve_program(&caller_dir, &configured, "Cobalt version")
            .expect("actual installed Cobalt executable");
        verify_version(&cobalt, "cobalt-bin 0.20.4", "Cobalt version")
            .expect("supported actual Cobalt version");

        let error = run_native(
            &cobalt,
            "Cobalt missing-config control",
            temp.path(),
            &["build", "--config", "missing-config.yml"],
            None,
        )
        .expect_err("real Cobalt rejects its missing config");
        match error {
            Error::NativeStatus {
                stage,
                status,
                stderr,
                ..
            } => {
                assert_eq!(stage, "Cobalt missing-config control");
                assert_ne!(status, "0");
                assert!(!stderr.is_empty());
            }
            other => panic!("expected typed native status error, got {other:?}"),
        }
    }
}
