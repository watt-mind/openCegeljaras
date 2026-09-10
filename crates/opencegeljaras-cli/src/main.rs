//! Command-line entry point for the openCegeljaras scaffold.

mod fields;
mod form;
mod output;
mod schemas;
mod validate;
mod xmllint;

use std::ffi::OsString;
use std::process::ExitCode;

use clap::{Args, Parser, Subcommand};
use opencegeljaras_core::{INTROSPECTION_COMMANDS, OPERATIONS, PROJECT, STAGE, source_catalog};
use serde_json::{Value, json};

use crate::fields::{DiffArgs, FieldsArgs};
use crate::form::InspectArgs;
use crate::output::{
    CommandError, CommandResult, failure, output, success, write_diagnostic, write_json, write_text,
};
use crate::schemas::SchemasCommand;
use crate::validate::ValidateArgs;

const SKILL: &str = include_str!("../skills/opencegeljaras/SKILL.md");
/// The exact option that selects JSON output.
const JSON_OPTION: &str = "--json";
/// The argument that ends option parsing; a later `--json` is a value.
const OPTION_TERMINATOR: &str = "--";

#[derive(Debug, Parser)]
#[command(
    name = "opencegeljaras",
    version,
    about = "Inspect the openCegeljaras scaffold and its reviewed source catalogue",
    disable_help_subcommand = true
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Report implemented domain operations and introspection commands.
    Capabilities(OutputArgs),
    /// List the reviewed source metadata compiled into this build.
    Sources(OutputArgs),
    /// Print the embedded agent skill.
    Skill(OutputArgs),
    /// Read-only operations on one local company-form document.
    Form {
        #[command(subcommand)]
        command: FormCommand,
    },
    /// Read-only operations on the pinned schema manifest and a local mirror.
    Schemas {
        #[command(subcommand)]
        command: SchemasCommand,
    },
}

#[derive(Debug, Subcommand)]
enum FormCommand {
    /// Recognise the format of one local form file. Not validation.
    Inspect(InspectArgs),
    /// Check one local form file against a verified local schema bundle.
    Validate(ValidateArgs),
    /// List the fields of one recognised local form file as stable paths.
    Fields(FieldsArgs),
    /// Compare two recognised local form files field by field.
    Diff(DiffArgs),
}

#[derive(Debug, Args)]
struct OutputArgs {
    /// Emit one stable JSON object on stdout.
    #[arg(long)]
    json: bool,
}

fn capabilities(args: &OutputArgs) -> CommandResult {
    let data = json!({
        "project": PROJECT,
        "stage": STAGE,
        "operations": OPERATIONS,
        "introspection_commands": INTROSPECTION_COMMANDS,
    });
    if args.json {
        return output(write_json(&success("capabilities", data)));
    }

    let operations = if OPERATIONS.is_empty() {
        String::from("none")
    } else {
        OPERATIONS.join(", ")
    };
    output(write_text(&format!(
        "{PROJECT}\nstage: {STAGE}\nimplemented operations: {operations}\nintrospection commands: {}\nverification: not performed\nsubmission: not performed\n",
        INTROSPECTION_COMMANDS.join(", ")
    )))
}

fn sources(args: &OutputArgs) -> CommandResult {
    let catalog = match source_catalog() {
        Ok(catalog) => catalog,
        Err(_) => {
            write_diagnostic("opencegeljaras: internal source catalogue error (exit 3)\n");
            return Err(CommandError::Reported(3));
        }
    };

    if args.json {
        let count = catalog.sources.len();
        return output(write_json(&success(
            "sources",
            json!({
                "reviewed_at": catalog.reviewed_at,
                "count": count,
                "sources": catalog.sources,
                "notice": "catalogue metadata only; no source content was fetched or checked",
            }),
        )));
    }

    let mut report = format!(
        "Reviewed source metadata: {}\nCatalogue entries: {}\n",
        catalog.reviewed_at,
        catalog.sources.len()
    );
    for source in catalog.sources {
        report.push_str(&format!(
            "- {} [{}; {}] {} — {}\n  {}\n",
            source.id, source.kind, source.status, source.title, source.url, source.notes
        ));
    }
    report.push_str("Catalogue metadata only; no source content was fetched or checked.\n");
    output(write_text(&report))
}

fn skill(args: &OutputArgs) -> CommandResult {
    if args.json {
        output(write_json(&success("skill", json!({ "document": SKILL }))))
    } else {
        output(write_text(SKILL))
    }
}

/// Whether the raw arguments select JSON output for a rejected command line.
///
/// The parse already failed, so clap cannot answer this: the raw arguments are
/// scanned instead. Only the exact option before any `--` terminator counts,
/// and the program name is never one of them. Nothing read here is echoed.
fn json_requested(raw_args: &[OsString]) -> bool {
    raw_args
        .iter()
        .skip(1)
        .take_while(|argument| argument.as_os_str() != OPTION_TERMINATOR)
        .any(|argument| argument.as_os_str() == JSON_OPTION)
}

/// Report a rejected command line, in JSON when `--json` asked for it.
///
/// Both forms are fixed text: no rejected argument, path, or value is echoed,
/// and the JSON form writes one object on stdout and nothing on stderr.
fn usage_error(raw_args: &[OsString]) -> u8 {
    if !json_requested(raw_args) {
        write_diagnostic(
            "opencegeljaras: invalid command line; run opencegeljaras --help (exit 2)\n",
        );
        return 2;
    }
    let envelope = failure(
        "usage",
        "usage.arguments",
        "Unsupported invocation. Run opencegeljaras --help.",
        Value::Null,
    );
    if write_json(&envelope).is_ok() {
        return 2;
    }
    write_diagnostic("opencegeljaras: failed to write output (exit 3)\n");
    3
}

fn run() -> u8 {
    let raw_args: Vec<_> = std::env::args_os().collect();
    let cli = match Cli::try_parse_from(&raw_args) {
        Ok(cli) => cli,
        Err(error) if error.exit_code() == 0 => {
            return if error.print().is_ok() { 0 } else { 3 };
        }
        Err(_) => return usage_error(&raw_args),
    };

    let result = match cli.command {
        Command::Capabilities(args) => capabilities(&args),
        Command::Sources(args) => sources(&args),
        Command::Skill(args) => skill(&args),
        Command::Form {
            command: FormCommand::Inspect(args),
        } => form::inspect(&args),
        Command::Form {
            command: FormCommand::Validate(args),
        } => validate::validate(&args),
        Command::Form {
            command: FormCommand::Fields(args),
        } => fields::fields(&args),
        Command::Form {
            command: FormCommand::Diff(args),
        } => fields::diff(&args),
        Command::Schemas { command } => schemas::run(&command),
    };

    match result {
        Ok(()) => 0,
        Err(CommandError::Output) => {
            write_diagnostic("opencegeljaras: failed to write output (exit 3)\n");
            3
        }
        Err(CommandError::Reported(code)) => code,
    }
}

fn main() -> ExitCode {
    ExitCode::from(run())
}
