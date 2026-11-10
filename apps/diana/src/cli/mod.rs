//! Command-line front end: argument parsing and result publication.
//!
//! The commands borrow the public analysis modules; they add no second decoding
//! path. File publication is atomic after complete serialization. A classified
//! rendering stop retains its prefix but exits nonzero; a sink failure does not
//! replace an existing output file.

mod extract_cli;
mod get;
mod matching;
mod outline;
mod output;
mod resources;
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

const EXAMPLES: &str = "Examples:
  diana classes app.apk --filter MainActivity
  diana get-class app.apk 'e0:l0:c123'
  diana get-method app.apk 'e0:l0:c123:m45'
  diana outline-class app.apk 'e0:l0:c123' --call-depth 2
  diana xref app.apk string Authorization
  diana manifest app.apk
Copy classes.at into get-class; copy methods.at into get-method and outline-method.
Locators name the entry and index, so follow-up commands skip the APK-wide scan.
Outline follows calls across every DEX; a class outside the input stays [other-dex].
Java may contain explicit stubs for constructs the recovery cannot express.";

use output::{Failure, Format, publish};
use table::{Locator, QueryRow, RowFormat};

#[derive(Parser)]
#[command(
    name = "diana",
    about = "Analyze APK archives and standalone DEX files",
    after_help = EXAMPLES,
    version
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
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
    /// List classes defined across every DEX in an APK
    #[command(
        long_about = "Column: at. Restrict with --filter, --extends, --implements, --subinterface and --kind; hierarchy filters use direct edges only."
    )]
    Classes {
        /// Input file (APK, ZIP or bare DEX)
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
        /// Workers for independent DEX images (default: available CPUs)
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
    /// List every field the archive DEX files declare
    #[command(
        long_about = "Column: at. Only class_data definitions; --class selects the defining class exactly and --filter defaults to ranked case-insensitive fuzzy matching."
    )]
    Fields {
        /// Input file (APK, ZIP or bare DEX)
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
        /// Workers for independent DEX images (default: available CPUs)
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
    /// List defined methods, excluding reference-only method_ids entries
    #[command(
        long_about = "Column: at. Only class_data definitions, including abstract, native, constructor and clinit declarations."
    )]
    Methods {
        /// Input file (APK, ZIP or bare DEX)
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
        /// Workers for independent DEX images (default: available CPUs)
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
    /// List every string in every root DEX of an archive
    #[command(
        long_about = "Columns: at, value. Original string_ids values; filtered searches default to ranked fuzzy matching. Find code reference sites with xref."
    )]
    Strings {
        /// Input file (APK, ZIP or bare DEX)
        input: PathBuf,
        #[arg(long)]
        /// Ranked case-insensitive fuzzy matching over the value; select another matching mode explicitly
        filter: Option<String>,
        #[command(flatten)]
        matching: MatchArgs,
        #[arg(long)]
        /// Workers for independent DEX images (default: available CPUs)
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
    /// List resources.arsc IDs, names, configurations and original values
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
    /// Locate a class in an APK and decompile it
    #[command(
        long_about = "Displays the Engine/P9 Java-like text, including original members and local recovery gaps. JSON schema 4 carries the same text, source facts and byte spans."
    )]
    GetClass {
        /// Input file (APK, ZIP or bare DEX)
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
    /// Decompile one method as Java, selected by its methods.at coordinate
    #[command(
        long_about = "Displays the selected original method through the shared Engine/P9 chain, without analyzing sibling methods."
    )]
    GetMethod {
        /// Input file (APK, ZIP or bare DEX)
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
    /// Write one original entry to -o; --raw keeps binary XML
    #[command(
        long_about = "Writes exactly one entry. A binary XML payload is decoded to text XML by default; --raw keeps the original payload. Publication is atomic."
    )]
    Extract {
        /// Input file (APK, ZIP or bare DEX)
        input: PathBuf,
        /// Exact physical entry name
        entry: String,
        #[arg(long)]
        /// Physical record index when the name repeats
        entry_index: Option<usize>,
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
    /// Find code references across every DEX in an APK
    #[command(
        long_about = "Columns: context, match, use. Kinds: string, type, method, field, impl, extend, resource. Defaults to ranked case-insensitive fuzzy matching; --substring, --regex and --exact select other modes. Canonical coordinates always select exact values."
    )]
    Xref {
        /// Input file (APK, ZIP or bare DEX)
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
        /// Method relationships instead of reference sites
        #[arg(long, value_enum, default_value_t = MethodRelationArg::References)]
        relation: MethodRelationArg,
        #[command(flatten)]
        matching: MatchArgs,
        #[arg(long)]
        /// Exact referencing class (descriptor or dotted name)
        class: Option<String>,
        #[arg(long)]
        /// Workers for independent DEX images (default: available CPUs)
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
    /// Decode the binary AndroidManifest.xml
    #[command(
        long_about = "Selects the last physical AndroidManifest.xml and prints text XML or JSON with warnings for any recovered gap."
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
        Command::Extract {
            input,
            entry,
            entry_index,
            output,
            raw,
            force,
            strict,
        } => extract_cli::run(&input, &entry, entry_index, &output, raw, force, strict),
        Command::Xref {
            input,
            kind,
            target,
            recursive,
            typed_uses,
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

fn entries(
    input: &std::path::Path,
    format: Format,
    output: Option<&std::path::Path>,
) -> Result<(), Failure> {
    let scope = RequestScope::open(input, &options()).map_err(Failure::input)?;
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
    let scope = RequestScope::open(input, &options()).map_err(Failure::input)?;
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
    let scope = RequestScope::open(input, &options()).map_err(Failure::input)?;
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
    table::publish_table(command, "items", columns, format, input, output, |sink| {
        query
            .run(
                &|row, out, first| {
                    let row = query_row(&row, &images)?;
                    rows.query_row(&row, out, first)
                },
                &mut |chunk: Vec<u8>| sink.chunk(&chunk),
            )
            .and_then(|truncated| {
                if truncated {
                    sink.truncated(limit)?;
                }
                Ok(())
            })
            .map_err(|error| Failure::query(error, 1))
    })
}

/// One analysis row as the columns this command publishes.
fn query_row<'a>(row: &'a Row<'a>, images: &[ImageInfo]) -> Result<QueryRow<'a>, QueryError> {
    let image_index = match row {
        Row::Class(row) => row.image.0,
        Row::Method(row) => row.image.0,
        Row::Field(row) => row.image.0,
        Row::String(row) => row.image.0,
        Row::Entry(_) => return Err(QueryError::WorkerLost),
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
        Row::Entry(_) => return Err(QueryError::WorkerLost),
    })
}
