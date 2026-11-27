//! Command-line front end: argument parsing and result publication.
//!
//! The commands borrow the public analysis modules; they add no second decoding
//! path. File publication is atomic after complete serialization. A classified
//! rendering stop retains its prefix but exits nonzero; a sink failure does not
//! replace an existing output file.

mod diff;
mod export;
mod extract_cli;
mod get;
mod jvm_code;
mod jvm_outline;
mod matching;
mod outline;
mod output;
mod resources;
mod skill;
mod table;
mod tree;
mod xref;

use std::path::PathBuf;

use clap::{Parser, Subcommand};

use crate::analysis::dex::{ClassFilter, ClassKind};
use crate::analysis::input::InputOptions;
use crate::analysis::manifest;
use crate::analysis::query::{ImageInfo, Query, QueryError, QueryKind, QueryPlan, Row};
use crate::analysis::request::RequestScope;
use matching::MatchArgs;

const AFTER_HELP: &str = "Examples:
  diana classes app.apk --filter MainActivity
  diana strings app.apk --filter Authorization
  diana get-class app.apk 'e0:l0:c123'
  diana get-method app.apk 'e0:l0:c123:m45'
  diana outline-class app.apk 'e0:l0:c123' --call-depth 2
  diana xref app.apk string Authorization
  diana manifest app.apk
  diana export app.apk -o project
  diana diff old.apk new.apk
  diana get-class game.jar com.example.Start
  diana export game.diana-jvm.json -o project
Reuse classes.at with get-class and methods.at with get-method or outline-method to focus on one original definition.
APK commands inspect root classes.dex and classes<N>.dex. Extract DEX stored elsewhere to analyze it directly.
JVM commands accept JARs, class files, and .diana-jvm.json descriptors for multiple archives or a Java release view.
Lists show 50 rows by default and report truncation. Use --filter or -o for large inventories.
Recovered code and declaration graphs mark local gaps in the available input.";

use output::{Failure, Format, publish};
use table::{Locator, QueryRow, RowFormat};

#[derive(Parser)]
#[command(
    name = "diana",
    about = "Analyze APK, DEX, JAR, JVM class files and JVM projects",
    after_help = AFTER_HELP,
    version
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Compare methods across two APK, DEX, JAR or JVM class inputs
    #[command(
        long_about = "Compare methods in two DEX inputs or two JVM inputs. Reports paired methods, pairing evidence, changes within methods, unresolved groups, and local read issues. Each method locator belongs to its own input."
    )]
    Diff(diff::Args),
    /// Show the original bytecode of a JVM method
    Bytecode {
        /// JAR or standalone class file
        input: PathBuf,
        /// JVM method coordinate from methods, such as j4:m2
        selector: String,
        #[arg(long, value_enum, default_value_t = Format::Text)]
        format: Format,
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// Print the Diana Agent Skill, or install it with -i
    Skill {
        #[arg(short = 'i', long)]
        /// Install to ~/.agents/skills/diana/SKILL.md without overwriting an existing file
        install: bool,
    },
    /// List the entries an archive holds, in central-directory order
    #[command(
        long_about = "Columns: name, compressed_size, uncompressed_size, compression. Physical records in directory order; numeric ZIP method, no label."
    )]
    Entries {
        /// Input file (APK, ZIP or bare DEX)
        input: PathBuf,
        #[arg(long, value_enum, default_value_t = Format::Text)]
        /// Presentation of the same result
        format: Format,
        #[arg(short, long)]
        /// Write the result to this file instead of stdout
        output: Option<PathBuf>,
    },
    /// List classes defined in APK, DEX or JVM inputs
    #[command(
        long_about = "DEX scope: classes.dex and classes<N>.dex at the archive root, or a bare DEX; DEX elsewhere is not scanned. JAR and standalone class files also supported. --filter, --extends, --implements, --subinterface and --kind restrict; hierarchy filters use direct edges only. 50 rows by default; --no-limit reaches several MB, so prefer --filter or -o."
    )]
    Classes {
        /// Input file (APK, DEX, JAR or JVM class)
        input: PathBuf,
        #[arg(long)]
        /// Ranked case-insensitive fuzzy matching over the descriptor; select another matching mode explicitly
        filter: Option<String>,
        #[command(flatten)]
        matching: MatchArgs,
        #[arg(long)]
        /// Only definitions whose own superclass is this class
        extends: Option<String>,
        #[arg(long)]
        /// Only classes whose own interface list contains this interface
        implements: Option<String>,
        #[arg(long)]
        /// Only interfaces whose own interface list contains this interface
        subinterface: Option<String>,
        #[arg(long, value_enum)]
        /// Restrict to classes or interfaces
        kind: Option<KindArg>,
        #[arg(long)]
        /// Workers for independent input units (default: available CPUs)
        jobs: Option<usize>,
        #[arg(long, default_value_t = xref::DEFAULT_LIMIT, conflicts_with = "no_limit")]
        /// Maximum rows to show; use --no-limit for every match
        limit: usize,
        #[arg(long, conflicts_with = "limit")]
        /// Show every matching row without a display limit
        no_limit: bool,
        #[arg(long, value_enum, default_value_t = Format::Text)]
        /// Presentation of the same result
        format: Format,
        #[arg(short, long)]
        /// Write the result to this file instead of stdout
        output: Option<PathBuf>,
    },
    /// Show the package hierarchy of defined classes
    Tree(tree::TreeArgs),
    /// List fields declared in APK, DEX or JVM inputs
    #[command(
        long_about = "DEX scope: classes.dex and classes<N>.dex at the archive root, or a bare DEX; DEX elsewhere is not scanned. JAR and standalone class files also supported. Only declared fields; --class selects the defining class exactly. 50 rows by default; --no-limit reaches several MB, so prefer --filter or -o."
    )]
    Fields {
        /// Input file (APK, DEX, JAR or JVM class)
        input: PathBuf,
        #[arg(long)]
        /// Ranked case-insensitive fuzzy matching over the name; select another matching mode explicitly
        filter: Option<String>,
        #[command(flatten)]
        matching: MatchArgs,
        #[arg(long)]
        /// Exact defining class (descriptor or dotted name)
        class: Option<String>,
        #[arg(long)]
        /// Workers for independent input units (default: available CPUs)
        jobs: Option<usize>,
        #[arg(long, default_value_t = xref::DEFAULT_LIMIT, conflicts_with = "no_limit")]
        /// Maximum rows to show; use --no-limit for every match
        limit: usize,
        #[arg(long, conflicts_with = "limit")]
        /// Show every matching row without a display limit
        no_limit: bool,
        #[arg(long, value_enum, default_value_t = Format::Text)]
        /// Presentation of the same result
        format: Format,
        #[arg(short, long)]
        /// Write the result to this file instead of stdout
        output: Option<PathBuf>,
    },
    /// List methods defined in APK, DEX or JVM inputs
    #[command(
        long_about = "DEX scope: classes.dex and classes<N>.dex at the archive root, or a bare DEX; DEX elsewhere is not scanned. JAR and standalone class files also supported. Only defined members, including abstract, native, constructor and class initializer methods. 50 rows by default; --no-limit reaches several MB, so prefer --filter or -o."
    )]
    Methods {
        /// Input file (APK, DEX, JAR or JVM class)
        input: PathBuf,
        #[arg(long)]
        /// Ranked case-insensitive fuzzy matching over the name; select another matching mode explicitly
        filter: Option<String>,
        #[command(flatten)]
        matching: MatchArgs,
        #[arg(long)]
        /// Exact defining class (descriptor or dotted name)
        class: Option<String>,
        #[arg(long)]
        /// Workers for independent input units (default: available CPUs)
        jobs: Option<usize>,
        #[arg(long, default_value_t = xref::DEFAULT_LIMIT, conflicts_with = "no_limit")]
        /// Maximum rows to show; use --no-limit for every match
        limit: usize,
        #[arg(long, conflicts_with = "limit")]
        /// Show every matching row without a display limit
        no_limit: bool,
        #[arg(long, value_enum, default_value_t = Format::Text)]
        /// Presentation of the same result
        format: Format,
        #[arg(short, long)]
        /// Write the result to this file instead of stdout
        output: Option<PathBuf>,
    },
    /// List DEX strings or JVM string constants
    #[command(
        long_about = "Columns: at, value. Root DEX only: classes.dex and classes<N>.dex at the archive root, or a bare DEX; DEX elsewhere is not scanned. JAR and standalone class files also supported. Each DEX string_ids or JVM CONSTANT_String entry keeps its own row. Text escapes control characters. Find code reference sites with xref. 50 rows by default; --no-limit reaches several MB, so prefer --filter or -o."
    )]
    Strings {
        /// Input file (APK, DEX, JAR or JVM class)
        input: PathBuf,
        #[arg(long)]
        /// Ranked case-insensitive fuzzy matching over the value; select another matching mode explicitly
        filter: Option<String>,
        #[command(flatten)]
        matching: MatchArgs,
        #[arg(long)]
        /// Workers for independent input units (default: available CPUs)
        jobs: Option<usize>,
        #[arg(long, default_value_t = xref::DEFAULT_LIMIT, conflicts_with = "no_limit")]
        /// Maximum rows to show; use --no-limit for every match
        limit: usize,
        #[arg(long, conflicts_with = "limit")]
        /// Show every matching row without a display limit
        no_limit: bool,
        #[arg(long, value_enum, default_value_t = Format::Text)]
        /// Presentation of the same result
        format: Format,
        #[arg(short, long)]
        /// Write the result to this file instead of stdout
        output: Option<PathBuf>,
    },
    /// List Android resource values or original JAR resource entries
    Resources {
        input: PathBuf,
        #[arg(long)]
        filter: Option<String>,
        #[command(flatten)]
        matching: MatchArgs,
        #[arg(long, value_enum, default_value_t = Format::Text)]
        format: Format,
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// Decompile a class from an APK, DEX, JAR or standalone JVM class
    #[command(
        long_about = "Displays Java-like source recovered by the engine, including original members and local recovery gaps. JSON schema 5 carries the same source and classified issues."
    )]
    GetClass {
        /// Input file (APK, ZIP, DEX, JAR or JVM class)
        input: PathBuf,
        /// Original class locator, descriptor or dotted name
        selector: String,
        #[arg(long, value_enum, default_value_t = Format::Text)]
        /// Presentation of the same result
        format: Format,
        #[arg(short, long)]
        /// Write the result to this file instead of stdout
        output: Option<PathBuf>,
        /// Parallel method workers (default: 1)
        #[arg(long, value_parser = clap::value_parser!(u32).range(1..))]
        jobs: Option<u32>,
    },
    /// Decompile an original method from an APK, DEX, JAR or JVM class
    #[command(
        long_about = "Displays Java-like source for the original method selected by a locator or exact reference."
    )]
    GetMethod {
        /// Input file (APK, ZIP, DEX, JAR or JVM class)
        input: PathBuf,
        /// Original method locator or complete owner/name/prototype reference
        selector: String,
        #[arg(long, value_enum, default_value_t = Format::Text)]
        /// Presentation of the same result
        format: Format,
        #[arg(short, long)]
        /// Write the result to this file instead of stdout
        output: Option<PathBuf>,
    },
    /// Show one class call tree with aggregated imports
    OutlineClass(outline::ClassArgs),
    /// Show the call tree below one method
    OutlineMethod(outline::OutlineArgs),
    /// Export Java-like sources by package, archive resources and manifest files
    Export(export::Args),
    /// Write one original entry to -o; --raw keeps binary XML
    #[command(
        long_about = "Writes exactly one entry. A binary XML payload is decoded to text XML by default; --raw keeps the original payload. Publication is atomic."
    )]
    Extract {
        /// Input APK, JAR, ZIP or JVM project
        input: PathBuf,
        /// Exact physical entry name, or a JVM location with --at
        entry: String,
        #[arg(long)]
        /// Physical record index when the name repeats
        entry_index: Option<usize>,
        /// Interpret the entry as a JVM archive coordinate and copy its original bytes
        #[arg(long, conflicts_with = "entry_index")]
        at: bool,
        #[arg(short, long)]
        /// Output file for the extracted payload
        output: PathBuf,
        /// Keep the original payload bytes even for binary XML
        #[arg(long)]
        /// Write the original payload bytes instead of decoded XML
        raw: bool,
        /// Replace an existing ordinary output file
        #[arg(long)]
        /// Replace an existing ordinary output file
        force: bool,
        /// Exit non-zero when a decoded document is partial
        #[arg(long)]
        /// Exit non-zero when the delivered document is partial
        strict: bool,
    },
    /// Find code references in the input
    #[command(
        long_about = "Columns: context, match, use. Inspects root DEX images or the selected JVM archive and project classes. Kinds: string, type, method, field, impl, extend, resource. Canonical coordinates select exact values. Use --filter or -o for large inventories."
    )]
    Xref {
        /// Input file (APK, DEX, JAR or JVM class)
        input: PathBuf,
        /// Target kind: string, type, method, field, impl, extend or resource
        kind: String,
        /// Name/descriptor pattern, owner->member, or canonical coordinate
        target: String,
        /// Include indirect impl/extend relationships
        #[arg(long)]
        recursive: bool,
        /// Expand type matches through referenced member signatures
        #[arg(long)]
        typed_uses: bool,
        /// Include method parameter type declarations in type queries
        #[arg(long)]
        parameter_types: bool,
        /// Method relationships instead of reference sites
        #[arg(long, value_enum, default_value_t = MethodRelationArg::References)]
        relation: MethodRelationArg,
        #[command(flatten)]
        matching: MatchArgs,
        #[arg(long)]
        /// Exact referencing class (descriptor or dotted name)
        class: Option<String>,
        #[arg(long)]
        /// Workers for independent input units (default: available CPUs)
        jobs: Option<usize>,
        #[arg(long)]
        /// Follow callers upstream for this many levels (``method`` kind only)
        transitive: Option<u32>,
        #[arg(long, default_value_t = xref::DEFAULT_LIMIT, conflicts_with = "no_limit")]
        /// Maximum total result rows across all depths; 0 prints all
        limit: usize,
        #[arg(long, conflicts_with = "limit")]
        /// Show all matching references without a display limit
        no_limit: bool,
        #[arg(long, value_enum, default_value_t = Format::Text)]
        /// Presentation of the same result
        format: Format,
        #[arg(short, long)]
        /// Write the result to this file instead of stdout
        output: Option<PathBuf>,
    },
    /// Display the Android or JAR manifest
    #[command(
        long_about = "Displays the last physical AndroidManifest.xml or META-INF/MANIFEST.MF in text or JSON."
    )]
    Manifest {
        /// Input file (APK, ZIP or bare DEX)
        input: PathBuf,
        #[arg(long, value_enum, default_value_t = Format::Text)]
        /// Presentation of the same result
        format: Format,
        #[arg(short, long)]
        /// Write the result to this file instead of stdout
        output: Option<PathBuf>,
        /// Exit non-zero when the document is partial
        #[arg(long)]
        /// Exit non-zero when the delivered document is partial
        strict: bool,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, clap::ValueEnum)]
enum KindArg {
    Class,
    Interface,
}

#[derive(Clone, Copy, Debug, clap::ValueEnum)]
enum MethodRelationArg {
    References,
    Implementations,
    Overrides,
}

/// Runs one command and maps its classified failure to an exit code.
pub fn run() -> std::process::ExitCode {
    let cli = Cli::parse();
    let result = match cli.command {
        Command::Diff(args) => diff::run(args),
        Command::Bytecode {
            input,
            selector,
            format,
            output,
        } => jvm_bytecode_command(&input, &selector, format, output.as_deref()),
        Command::Skill { install } => skill::run(install),
        Command::Resources {
            input,
            filter,
            matching,
            format,
            output,
        } => resources::run(
            &input,
            filter.as_deref(),
            matching,
            format,
            output.as_deref(),
        ),
        Command::Tree(args) => tree::run(args),
        Command::OutlineClass(args) => outline::class(args),
        Command::OutlineMethod(args) => outline::method(args),
        Command::Entries {
            input,
            format,
            output,
        } => entries(&input, format, output.as_deref()),
        Command::Manifest {
            input,
            format,
            output,
            strict,
        } => manifest_command(&input, format, output.as_deref(), strict),
        Command::GetClass {
            input,
            selector,
            format,
            output,
            jobs,
        } => get::run(
            &input,
            &selector,
            get::Kind::Class {
                jobs: jobs.unwrap_or(1),
            },
            format,
            output.as_deref(),
        ),
        Command::GetMethod {
            input,
            selector,
            format,
            output,
        } => get::run(
            &input,
            &selector,
            get::Kind::Method,
            format,
            output.as_deref(),
        ),
        Command::Export(args) => export::run(args),
        Command::Extract {
            input,
            entry,
            entry_index,
            at,
            output,
            raw,
            force,
            strict,
        } => extract_cli::run(&input, &entry, entry_index, at, &output, raw, force, strict),
        Command::Xref {
            input,
            kind,
            target,
            recursive,
            typed_uses,
            parameter_types,
            relation,
            matching,
            class,
            jobs,
            transitive,
            limit,
            no_limit,
            format,
            output,
        } => xref::run(
            &input,
            &kind,
            &target,
            matching.mode(),
            recursive,
            typed_uses,
            parameter_types,
            match relation {
                MethodRelationArg::References => crate::analysis::xref::MethodRelation::References,
                MethodRelationArg::Implementations => {
                    crate::analysis::xref::MethodRelation::Implementations
                }
                MethodRelationArg::Overrides => crate::analysis::xref::MethodRelation::Overrides,
            },
            class.as_deref(),
            transitive.unwrap_or(0),
            if no_limit { 0 } else { limit },
            match jobs {
                Some(0) => {
                    return std::process::ExitCode::from(2);
                }
                Some(jobs) => jobs,
                None => default_jobs(),
            },
            format,
            output.as_deref(),
        ),
        Command::Classes {
            input,
            filter,
            matching,
            extends,
            implements,
            subinterface,
            kind,
            jobs,
            limit,
            no_limit,
            format,
            output,
        } => query_command(
            &input,
            QueryKind::Classes,
            QueryArgs {
                filter,
                matching,
                class: None,
                jobs,
                class_filter: ClassFilter {
                    extends: extends.as_deref().map(selector_units),
                    implements: implements.as_deref().map(selector_units),
                    subinterface: subinterface.as_deref().map(selector_units),
                    kind: kind.map(|value| match value {
                        KindArg::Class => ClassKind::Class,
                        KindArg::Interface => ClassKind::Interface,
                    }),
                },
            },
            format,
            output.as_deref(),
            if no_limit { 0 } else { limit },
        ),
        Command::Fields {
            input,
            filter,
            matching,
            class,
            jobs,
            limit,
            no_limit,
            format,
            output,
        } => query_command(
            &input,
            QueryKind::Fields,
            QueryArgs {
                filter,
                matching,
                class,
                jobs,
                class_filter: ClassFilter::default(),
            },
            format,
            output.as_deref(),
            if no_limit { 0 } else { limit },
        ),
        Command::Methods {
            input,
            filter,
            matching,
            class,
            jobs,
            limit,
            no_limit,
            format,
            output,
        } => query_command(
            &input,
            QueryKind::Methods,
            QueryArgs {
                filter,
                matching,
                class,
                jobs,
                class_filter: ClassFilter::default(),
            },
            format,
            output.as_deref(),
            if no_limit { 0 } else { limit },
        ),
        Command::Strings {
            input,
            filter,
            matching,
            jobs,
            limit,
            no_limit,
            format,
            output,
        } => query_command(
            &input,
            QueryKind::Strings,
            QueryArgs {
                filter,
                matching,
                class: None,
                jobs,
                class_filter: ClassFilter::default(),
            },
            format,
            output.as_deref(),
            if no_limit { 0 } else { limit },
        ),
    };
    match result {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(failure) => {
            eprintln!("diana: {}", failure.message);
            std::process::ExitCode::from(failure.code)
        }
    }
}

fn options() -> InputOptions {
    InputOptions::default()
}

fn open_scope(
    input: &std::path::Path,
    output: Option<&std::path::Path>,
) -> Result<RequestScope, Failure> {
    let scope = RequestScope::open(input, &options()).map_err(Failure::input)?;
    output::protect_project_targets(&scope, output)?;
    Ok(scope)
}

fn jvm_bytecode_command(
    input: &std::path::Path,
    selector: &str,
    format: Format,
    output: Option<&std::path::Path>,
) -> Result<(), Failure> {
    let scope = open_scope(input, output)?;
    let (location, _) =
        crate::analysis::jvm_code::method_at(selector).map_err(|error| Failure {
            code: 2,
            message: error.to_string(),
        })?;
    let roots = crate::analysis::jvm::JvmRoots::focused(&scope, &location)
        .map_err(|error| Failure::query(error, 1))?;
    jvm_code::bytecode(&roots, selector, format, input, output)
}

fn entries(
    input: &std::path::Path,
    format: Format,
    output: Option<&std::path::Path>,
) -> Result<(), Failure> {
    let scope = open_scope(input, output)?;
    if scope.jvm_artifacts().is_some() {
        let roots = crate::analysis::jvm::JvmRoots::select_with_jobs(&scope, default_jobs())
            .map_err(|error| Failure::query(error, 1))?
            .ok_or_else(|| {
                Failure::output(std::io::Error::other("JVM project roots are unavailable"))
            })?;
        return resources::publish_jvm(&roots, "entries", None, input, format, output);
    }
    let query = Query::new(
        &scope,
        QueryPlan {
            kind: QueryKind::Entries,
            filter: None,
            class: None,
            class_filter: Default::default(),
            jobs: 1,
            limit: 0,
        },
    )
    .map_err(|error| Failure::query(error, 1))?;
    let columns = [
        "name",
        "compressed_size",
        "uncompressed_size",
        "compression",
    ];
    let rows = RowFormat::new(format, &columns);
    table::publish_table(
        "entries",
        "entries",
        &columns,
        format,
        input,
        output,
        |sink| {
            query
                .run(
                    &|row, out, first| {
                        if let Row::Entry(row) = row {
                            rows.entry_row(
                                &row.name,
                                row.compressed_size,
                                row.uncompressed_size,
                                u64::from(row.method),
                                out,
                                first,
                            );
                        }
                        Ok(())
                    },
                    &mut |chunk: Vec<u8>| sink.chunk(&chunk),
                )
                .map(|_| ())
                .map_err(|error| Failure::query(error, 1))
        },
    )
}

fn manifest_command(
    input: &std::path::Path,
    format: Format,
    output: Option<&std::path::Path>,
    strict: bool,
) -> Result<(), Failure> {
    let scope = open_scope(input, output)?;
    if crate::analysis::jvm::JvmRoots::supports(&scope) {
        use crate::analysis::{jvm::MANIFEST_NAME, jvm_resources};
        let (at, bytes) = jvm_resources::manifest(&scope)
            .map_err(|error| Failure::query(error, 1))?
            .ok_or_else(|| Failure {
                code: 3,
                message: format!("the input has no {MANIFEST_NAME} entry"),
            })?;
        return match format {
            Format::Text => output::publish_with(output, input, |writer| {
                use std::io::Write;
                writer.write_all(&bytes).map_err(Failure::output)
            }),
            Format::Json => {
                let text = std::str::from_utf8(&bytes).ok();
                let document = serde_json::json!({
                    "schema_version": 2, "command": "manifest", "kind": "jar",
                    "status": "complete", "at": at.to_string(), "text": text,
                    "bytes": text.is_none().then_some(&bytes),
                });
                output::publish_with(output, input, |writer| {
                    serde_json::to_writer(writer, &document)
                        .map_err(|error| Failure::output(std::io::Error::other(error)))
                })
            }
        };
    }
    let manifest = match manifest::read(&scope) {
        Ok(manifest) => manifest,
        Err(error) => {
            let code = match error {
                manifest::ManifestSelectionError::Missing => 3,
                _ => 1,
            };
            return Err(Failure {
                code,
                message: error.to_string(),
            });
        }
    };
    for warning in manifest.warnings() {
        eprintln!(
            "diana: warning: byte {}: {}",
            warning.offset,
            warning.kind.describe()
        );
    }
    let text = match format {
        Format::Text => manifest.xml().to_owned(),
        Format::Json => serde_json::json!({
            "schema_version": 2,
            "command": "manifest",
            "status": if manifest.is_partial() { "partial" } else { "complete" },
            "xml": manifest.xml(),
            "warnings": manifest.warnings().iter().map(|warning| serde_json::json!({
                "code": warning.kind.code(),
                "offset": warning.offset,
                "detail": warning.kind.describe(),
            })).collect::<Vec<_>>(),
        })
        .to_string(),
    };
    publish(output, input, text.as_bytes(), format)?;
    if strict && manifest.is_partial() {
        return Err(Failure {
            code: 1,
            message: "manifest is partial and --strict was requested".to_owned(),
        });
    }
    Ok(())
}

struct QueryArgs {
    filter: Option<String>,
    matching: MatchArgs,
    class: Option<String>,
    jobs: Option<usize>,
    class_filter: ClassFilter,
}

fn selector_units(text: &str) -> Vec<u16> {
    crate::analysis::dex::class_descriptor(text)
}

fn default_jobs() -> usize {
    std::thread::available_parallelism()
        .map(|count| count.get())
        .unwrap_or(1)
}

fn query_command(
    input: &std::path::Path,
    kind: QueryKind,
    args: QueryArgs,
    format: Format,
    output: Option<&std::path::Path>,
    limit: usize,
) -> Result<(), Failure> {
    let filter = args.matching.filter(args.filter.as_deref())?;
    let scope = open_scope(input, output)?;
    let jobs = match args.jobs {
        Some(0) => {
            return Err(Failure {
                code: 2,
                message: "--jobs must be greater than zero".to_owned(),
            });
        }
        Some(jobs) => jobs,
        None => default_jobs(),
    };
    let plan = QueryPlan {
        kind,
        filter,
        class: args.class.as_deref().map(selector_units),
        class_filter: args.class_filter,

        jobs,
        limit,
    };
    let query = Query::new(&scope, plan).map_err(|error| Failure::query(error, 1))?;
    let images = query.images().to_vec();
    let columns: &[&str] = match kind {
        QueryKind::Strings => &["at", "value"],
        QueryKind::Classes => &["at", "descriptor"],
        QueryKind::Methods | QueryKind::Fields => &["at", "member"],
        QueryKind::Entries => &["at"],
    };
    let command = match kind {
        QueryKind::Classes => "classes",
        QueryKind::Fields => "fields",
        QueryKind::Methods => "methods",
        QueryKind::Strings => "strings",
        QueryKind::Entries => "entries",
    };
    let rows = RowFormat::new(format, columns);
    let mut issue_count = 0;
    table::publish_table(command, "items", columns, format, input, output, |sink| {
        query
            .run_detailed(
                &|row, out, first| {
                    if let Row::Jvm(row) = &row {
                        return rows.jvm_row(row, out, first);
                    }
                    let row = query_row(&row, &images)?;
                    rows.query_row(&row, out, first)
                },
                &mut |chunk: Vec<u8>| sink.chunk(&chunk),
            )
            .and_then(|outcome| {
                if outcome.truncated {
                    sink.truncated(limit)?;
                }
                issue_count = outcome.issues.len();
                sink.issues(outcome.issues);
                Ok(())
            })
            .map_err(|error| Failure::query(error, 1))
    })?;
    if issue_count > 0 {
        return Err(Failure {
            code: 1,
            message: format!("{issue_count} JVM input issue(s), see query output"),
        });
    }
    Ok(())
}

/// One analysis row as the columns this command publishes.
fn query_row<'a>(row: &'a Row<'a>, images: &[ImageInfo]) -> Result<QueryRow<'a>, QueryError> {
    let image_index = match row {
        Row::Class(row) => row.image.0,
        Row::Method(row) => row.image.0,
        Row::Field(row) => row.image.0,
        Row::String(row) => row.image.0,
        Row::Entry(_) | Row::Jvm(_) => return Err(QueryError::WorkerLost),
    };
    let image = images.get(image_index).ok_or(QueryError::WorkerLost)?;
    let entry = image.entry.map(|entry| entry.0);
    Ok(match row {
        Row::Class(row) => QueryRow {
            at: Locator::Class {
                entry,
                logical: row.logical.0,
                class_def: row.class_def.0,
                descriptor: row.descriptor,
            },
            value: None,
        },
        Row::Method(row) => QueryRow {
            at: Locator::Method {
                entry,
                logical: row.logical.0,
                class_def: row.class_def.0,
                method: row.method.0,
                descriptor: row.class_descriptor,
                name: row.name,
                parameters: row.prototype.parameters,
                returns: row.prototype.returns,
            },
            value: None,
        },
        Row::Field(row) => QueryRow {
            at: Locator::Field {
                entry,
                logical: row.logical.0,
                class_def: row.class_def.0,
                field: row.field.0,
                descriptor: row.class_descriptor,
                name: row.name,
                field_type: row.field_type,
            },
            value: None,
        },
        Row::String(row) => QueryRow {
            at: Locator::String {
                entry,
                logical: row.logical.0,
                string: row.string.0,
            },
            value: Some(row.value),
        },
        Row::Entry(_) | Row::Jvm(_) => return Err(QueryError::WorkerLost),
    })
}
