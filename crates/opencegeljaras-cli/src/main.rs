//! Command-line entry point for the openCegeljaras scaffold.

mod form;
mod output;
mod schemas;
mod validate;
mod xmllint;

use std::process::ExitCode;

use clap::{Args, Parser, Subcommand};
use opencegeljaras_core::{INTROSPECTION_COMMANDS, OPERATIONS, PROJECT, STAGE, source_catalog};
use serde_json::json;

use crate::form::InspectArgs;
use crate::output::{
    CommandError, CommandResult, output, success, write_diagnostic, write_json, write_text,
};
use crate::schemas::SchemasCommand;
use crate::validate::ValidateArgs;

const SKILL: &str = include_str!("../skills/opencegeljaras/SKILL.md");

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

fn run() -> u8 {
    let raw_args: Vec<_> = std::env::args_os().collect();
    let cli = match Cli::try_parse_from(&raw_args) {
        Ok(cli) => cli,
        Err(error) if error.exit_code() == 0 => {
            return if error.print().is_ok() { 0 } else { 3 };
        }
        Err(_) => {
            write_diagnostic(
                "opencegeljaras: invalid command line; run opencegeljaras --help (exit 2)\n",
            );
            return 2;
        }
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
