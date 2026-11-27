//! Publish a package tree after the last worker and report writer have finished.
use std::fs::{self, File};
use std::io::{self, BufWriter, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Mutex, MutexGuard};

use serde_json::json;

use super::output::Failure;
use crate::analysis::export_path::{SourceFiles, create_file, path_conflict, portable_component};
use crate::analysis::export_resources::{self, ResourceNotice};
use crate::analysis::jvm::JvmRoots;
use crate::analysis::jvm_source;
use crate::analysis::request::RequestScope;
use crate::analysis::source_export::{self, SourceEvent};

#[derive(clap::Args)]
pub(super) struct Args {
    /// APK, JAR, ZIP, standalone DEX or JVM class
    input: PathBuf,
    /// New output directory (its parent must exist)
    #[arg(short, long)]
    output: PathBuf,
    /// Concurrent class workers (default: available CPUs, up to 16). Lower this to reduce peak memory.
    #[arg(long, value_parser = clap::value_parser!(u32).range(1..))]
    jobs: Option<u32>,
}

pub(super) fn run(args: Args) -> Result<(), Failure> {
    let scope = RequestScope::open(&args.input, &super::options()).map_err(Failure::input)?;
    let jobs = args
        .jobs
        .map_or_else(|| super::default_jobs().min(16), |n| n as usize);
    let Summary {
        classes,
        resources,
        issues,
        warnings,
    } = write_export(&scope, &args.output, jobs).map_err(Failure::output)?;
    eprintln!(
        "Exported {classes} classes and {resources} resource files to {}",
        args.output.display()
    );
    if warnings > 0 {
        eprintln!("diana: {warnings} resource warning(s). See export-report.jsonl");
    }
    if issues > 0 {
        return Err(Failure {
            code: 1,
            message: format!(
                "{issues} recovery issue(s). See export-report.jsonl in the output directory"
            ),
        });
    }
    Ok(())
}

struct Summary {
    classes: usize,
    resources: usize,
    issues: usize,
    warnings: usize,
}

fn write_export(scope: &RequestScope, output: &Path, jobs: usize) -> io::Result<Summary> {
    match fs::symlink_metadata(output) {
        Ok(_) => {
            return Err(io::Error::new(
                io::ErrorKind::AlreadyExists,
                "export destination already exists",
            ));
        }
        Err(e) if e.kind() == io::ErrorKind::NotFound => (),
        Err(e) => return Err(e),
    }
    let parent = output
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let mut builder = tempfile::Builder::new();
    builder.prefix(".diana-export-");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        builder.permissions(fs::Permissions::from_mode(0o700));
    }
    let stage = builder.tempdir_in(parent)?;
    let root = stage.path();
    let report = Mutex::new(Report::default());
    let classes = AtomicUsize::new(0);
    let source_root = root.join("sources");
    fs::create_dir(&source_root)?;
    let source_files = SourceFiles::new(&source_root);
    let jvm = JvmRoots::select_with_jobs(scope, jobs).map_err(io::Error::other)?;
    let mut warnings = 0;
    let resources =
        if let Some(roots) = &jvm {
            export_resources::write_jvm(scope, roots, root, |at, entry, notice, reason| {
                warnings += usize::from(notice == ResourceNotice::Warning);
                lock_report(&report)?.record(
                    root,
                    notice == ResourceNotice::Issue,
                    &json!({
                        "kind": "resource", "notice": format!("{notice:?}"), "at": at,
                        "name": entry.name(), "reason": reason
                    }),
                )
            })?
        } else {
            export_resources::write(scope, root, |entry, notice, reason| {
                warnings += usize::from(notice == ResourceNotice::Warning);
                lock_report(&report)?.record(root, notice == ResourceNotice::Issue, &json!({
                "kind": "resource", "notice": format!("{notice:?}"), "entry": entry.index().0,
                "name": entry.name(), "reason": reason
            }))
            })?
        };
    let write = |event| {
        match event {
            SourceEvent::Class {
                entry,
                logical,
                class_def,
                descriptor,
                text,
                failures,
            } => {
                if let Some(path) =
                    write_definition(root, &source_files, &descriptor, true, &text, || {
                        let image = entry.map_or_else(|| "input".to_owned(), |n| format!("e{n}"));
                        format!("source-definitions/{image}-l{logical}-c{class_def}.java")
                    })?
                {
                    lock_report(&report)?.record(root, false, &json!({
                        "kind": "class-path", "entry":entry, "logical":logical, "class_def":class_def,
                        "descriptor_units":descriptor, "path":path
                    }))?;
                }
                classes.fetch_add(1, Ordering::Relaxed);
                if !failures.is_empty() {
                    lock_report(&report)?.record(root, true, &json!({
                        "kind": "methods", "entry":entry, "logical":logical, "class_def":class_def,
                        "failures":failures
                    }))?;
                }
            }
            SourceEvent::Issue {
                entry,
                logical,
                class_def,
                reason,
            } => {
                lock_report(&report)?.record(root, true, &json!({
                    "kind":"source", "entry":entry, "logical":logical, "class_def":class_def, "reason":reason
                }))?;
            }
            SourceEvent::JvmClass {
                at,
                versioned,
                mut source,
            } => {
                let mut descriptor = source.name;
                descriptor.insert(0, u16::from(b'L'));
                descriptor.push(u16::from(b';'));
                let class_at = std::cell::LazyCell::new(|| at.to_string());
                let failures = if source.issues.is_empty() {
                    Vec::new()
                } else {
                    jvm_source::export_issues(&class_at, &mut source.text, source.issues)
                        .map_err(io::Error::other)?
                };
                if let Some(path) = write_definition(
                    root,
                    &source_files,
                    &descriptor,
                    scope.java_version().is_some() || !versioned,
                    &source.text,
                    || format!("source-definitions/{}.java", class_at.replace('/', "-")),
                )? {
                    lock_report(&report)?.record(
                        root,
                        false,
                        &json!({
                            "kind": "class-path", "at": class_at.as_str(),
                            "descriptor_units": descriptor, "path": path
                        }),
                    )?;
                }
                classes.fetch_add(1, Ordering::Relaxed);
                if !failures.is_empty() {
                    lock_report(&report)?.record(
                        root,
                        true,
                        &json!({
                            "kind": "methods", "at": class_at.as_str(), "failures": failures
                        }),
                    )?;
                }
            }
            SourceEvent::JvmIssue(issue) => {
                lock_report(&report)?.record(
                    root,
                    true,
                    &json!({
                        "kind": "source", "at": issue.context,
                        "classification": issue.kind, "reason": issue.reason
                    }),
                )?;
            }
        }
        Ok(())
    };
    if let Some(roots) = &jvm {
        source_export::visit_jvm(roots, jobs, write)?;
    } else {
        source_export::visit(scope, jobs, write)?;
    }
    let mut report = report
        .into_inner()
        .map_err(|_| io::Error::other("export report lock poisoned"))?;
    if let Some(mut writer) = report.writer.take() {
        writer.flush()?;
    }
    publish_tree(root, output)?;
    // Renaming publishes the inode. Disarm cleanup of its former name.
    let _ = stage.keep();
    Ok(Summary {
        classes: classes.into_inner(),
        resources,
        issues: report.issues,
        warnings,
    })
}

#[derive(Default)]
struct Report {
    writer: Option<BufWriter<File>>,
    issues: usize,
}

fn lock_report(report: &Mutex<Report>) -> io::Result<MutexGuard<'_, Report>> {
    report
        .lock()
        .map_err(|_| io::Error::other("export report lock poisoned"))
}

impl Report {
    fn record(&mut self, root: &Path, incomplete: bool, row: &serde_json::Value) -> io::Result<()> {
        if self.writer.is_none() {
            self.writer = Some(BufWriter::new(File::create_new(
                root.join("export-report.jsonl"),
            )?));
        }
        let writer = self.writer.as_mut().expect("opened above");
        serde_json::to_writer(&mut *writer, row)?;
        writer.write_all(b"\n")?;
        self.issues += usize::from(incomplete);
        Ok(())
    }
}

fn write_definition(
    stage: &Path,
    sources: &SourceFiles<'_>,
    descriptor: &[u16],
    package_path: bool,
    text: &str,
    fallback: impl FnOnce() -> String,
) -> io::Result<Option<String>> {
    if package_path
        && let Some(relative) = String::from_utf16(descriptor)
            .ok()
            .as_deref()
            .and_then(class_path)
    {
        match sources.create(&relative) {
            Ok(mut file) => {
                file.write_all(text.as_bytes())?;
                return Ok(None);
            }
            Err(error) if path_conflict(&error) => {}
            Err(error) => return Err(error),
        }
    }
    let path = fallback();
    create_file(stage, Path::new(&path))?.write_all(text.as_bytes())?;
    Ok(Some(path))
}

fn class_path(descriptor: &str) -> Option<PathBuf> {
    let name = descriptor.strip_prefix('L')?.strip_suffix(';')?;
    if !name.split('/').all(portable_component) {
        return None;
    }
    let relative = format!("{name}.java");
    portable_component(relative.rsplit('/').next()?).then(|| PathBuf::from(relative))
}

#[cfg(any(target_os = "linux", target_vendor = "apple", target_os = "android"))]
fn publish_tree(from: &Path, to: &Path) -> io::Result<()> {
    use rustix::fs::{CWD, RenameFlags, renameat_with};
    renameat_with(CWD, from, CWD, to, RenameFlags::NOREPLACE).map_err(Into::into)
}

#[cfg(windows)]
fn publish_tree(from: &Path, to: &Path) -> io::Result<()> {
    // Windows rename cannot replace an existing directory.
    fs::rename(from, to)
}

#[cfg(not(any(
    target_os = "linux",
    target_vendor = "apple",
    target_os = "android",
    windows
)))]
fn publish_tree(_: &Path, _: &Path) -> io::Result<()> {
    Err(io::Error::new(
        io::ErrorKind::Unsupported,
        "exclusive directory rename unavailable",
    ))
}
