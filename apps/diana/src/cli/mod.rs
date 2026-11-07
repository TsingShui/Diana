//! Command-line front end: argument parsing and result publication.
//!
//! The commands borrow the public analysis modules; they add no second decoding
//! path. File publication is atomic after complete serialization. A classified
//! rendering stop retains its prefix but exits nonzero; a sink failure does not
//! replace an existing output file.

mod extract_cli;
mod get;
mod output;
mod table;
mod xref;

use std::path::PathBuf;

use clap::{Parser, Subcommand};

use crate::analysis::dex::{ClassFilter, ClassKind};
use crate::analysis::input::InputOptions;
use crate::analysis::manifest;
use crate::analysis::query::{ImageInfo, Query, QueryError, QueryKind, QueryPlan, Row};
use crate::analysis::request::RequestScope;

use output::{Failure, Format, publish};
use table::{Locator, QueryRow, RowFormat};

#[derive(Parser)]
#[command(name = "diana", about = "Native APK/DEX analysis", version)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// List physical archive entries
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
    /// List class definitions
    #[command(
        long_about = "Column: at. Restrict with --filter, --extends, --implements, --subinterface and --kind; hierarchy filters use direct edges only."
    )]
    Classes {
        /// Input file (APK, ZIP or bare DEX)
        input: PathBuf,
        #[arg(long)]
        /// Case-insensitive literal over the semantic name/descriptor
        filter: Option<String>,
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
        #[arg(long, value_enum, default_value_t = Format::Text)]
        /// Presentation of the same result
        format: Format,
        #[arg(short, long)]
        /// Write the result to this file instead of stdout
        output: Option<PathBuf>,
    },
    /// List defined fields
    #[command(
        long_about = "Column: at. Only class_data definitions; --class selects the defining class exactly and --filter is a literal substring."
    )]
    Fields {
        /// Input file (APK, ZIP or bare DEX)
        input: PathBuf,
        #[arg(long)]
        /// Case-insensitive literal over the semantic name/descriptor
        filter: Option<String>,
        #[arg(long)]
        /// Exact defining class (descriptor or dotted name)
        class: Option<String>,
        #[arg(long)]
        /// Workers for independent DEX images (default: available CPUs)
        jobs: Option<usize>,
        #[arg(long, value_enum, default_value_t = Format::Text)]
        /// Presentation of the same result
        format: Format,
        #[arg(short, long)]
        /// Write the result to this file instead of stdout
        output: Option<PathBuf>,
    },
    /// List defined methods
    #[command(
        long_about = "Column: at. Only class_data definitions, including abstract, native, constructor and clinit declarations."
    )]
    Methods {
        /// Input file (APK, ZIP or bare DEX)
        input: PathBuf,
        #[arg(long)]
        /// Case-insensitive literal over the semantic name/descriptor
        filter: Option<String>,
        #[arg(long)]
        /// Exact defining class (descriptor or dotted name)
        class: Option<String>,
        #[arg(long)]
        /// Workers for independent DEX images (default: available CPUs)
        jobs: Option<usize>,
        #[arg(long, value_enum, default_value_t = Format::Text)]
        /// Presentation of the same result
        format: Format,
        #[arg(short, long)]
        /// Write the result to this file instead of stdout
        output: Option<PathBuf>,
    },
    /// List original string values
    #[command(
        long_about = "Columns: at, value, entry. string_ids order with the complete UTF-16 value; a code reference site is found with xref."
    )]
    Strings {
        /// Input file (APK, ZIP or bare DEX)
        input: PathBuf,
        #[arg(long)]
        /// Case-insensitive literal over the semantic name/descriptor
        filter: Option<String>,
        #[arg(long)]
        /// Workers for independent DEX images (default: available CPUs)
        jobs: Option<usize>,
        #[arg(long, value_enum, default_value_t = Format::Text)]
        /// Presentation of the same result
        format: Format,
        #[arg(short, long)]
        /// Write the result to this file instead of stdout
        output: Option<PathBuf>,
    },
    /// Decompile one class
    #[command(
        long_about = "Displays the Engine/P9 Java-like text, including original members and local recovery gaps. JSON schema 3 carries the same text, source facts and byte spans."
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
    /// Decompile one method
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
    /// Extract one physical entry
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
    /// Find original reference sites
    #[command(
        long_about = "Columns: entry, context, match, use. Kinds: string, type, method, field; each real operand site is one row."
    )]
    Xref {
        /// Input file (APK, ZIP or bare DEX)
        input: PathBuf,
        /// string | type | method | field
        /// Target kind: string, type, method or field
        kind: String,
        /// Exact target value or complete reference
        target: String,
        #[arg(long)]
        /// Exact defining class (descriptor or dotted name)
        class: Option<String>,
        #[arg(long)]
        /// Workers for independent DEX images (default: available CPUs)
        jobs: Option<usize>,
        #[arg(long)]
        /// Follow callers upstream for this many levels (``method`` kind only)
        transitive: Option<u32>,
        #[arg(long)]
        /// Rows printed per transitive level; 0 prints every row
        limit: Option<usize>,
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

/// Runs one command and maps its classified failure to an exit code.
pub fn run() -> std::process::ExitCode {
    let cli = Cli::parse();
    let result = match cli.command {
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
            class,
            jobs,
            transitive,
            limit,
            format,
            output,
        } => xref::run(
            &input,
            &kind,
            &target,
            class.as_deref(),
            transitive.unwrap_or(0),
            limit.unwrap_or(10),
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
            extends,
            implements,
            subinterface,
            kind,
            jobs,
            format,
            output,
        } => query_command(
            &input,
            QueryKind::Classes,
            QueryArgs {
                filter,
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
        ),
        Command::Fields {
            input,
            filter,
            class,
            jobs,
            format,
            output,
        } => query_command(
            &input,
            QueryKind::Fields,
            QueryArgs {
                filter,
                class,
                jobs,
                class_filter: ClassFilter::default(),
            },
            format,
            output.as_deref(),
        ),
        Command::Methods {
            input,
            filter,
            class,
            jobs,
            format,
            output,
        } => query_command(
            &input,
            QueryKind::Methods,
            QueryArgs {
                filter,
                class,
                jobs,
                class_filter: ClassFilter::default(),
            },
            format,
            output.as_deref(),
        ),
        Command::Strings {
            input,
            filter,
            jobs,
            format,
            output,
        } => query_command(
            &input,
            QueryKind::Strings,
            QueryArgs {
                filter,
                class: None,
                jobs,
                class_filter: ClassFilter::default(),
            },
            format,
            output.as_deref(),
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
            "schema_version": 1,
            "command": "manifest",
            "entry": manifest.entry().name(),
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
) -> Result<(), Failure> {
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
        filter: args
            .filter
            .as_deref()
            .map(|text| text.encode_utf16().collect()),
        class: args.class.as_deref().map(selector_units),
        class_filter: args.class_filter,

        jobs,
    };
    let query = Query::new(&scope, plan).map_err(|error| Failure::query(error, 1))?;
    let images = query.images().to_vec();
    let columns: &[&str] = match kind {
        QueryKind::Strings => &["at", "value", "entry"],
        _ => &["at"],
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
            .map_err(|error| Failure::query(error, 1))
    })
}

/// One analysis row as the columns this command publishes.
fn query_row<'a>(row: &'a Row<'a>, images: &'a [ImageInfo]) -> Result<QueryRow<'a>, QueryError> {
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
            entry: None,
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
            entry: None,
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
            entry: None,
        },
        Row::String(row) => QueryRow {
            at: Locator::String {
                entry,
                logical: row.logical.0,
                string: row.string.0,
            },
            value: Some(row.value),
            entry: Some(image.label.as_str()),
        },
        Row::Entry(_) => return Err(QueryError::WorkerLost),
    })
}
