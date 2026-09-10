use std::io::{self, Write};
use std::process::ExitCode;

use clap::{Args, Parser, Subcommand};
use opencegeljaras_core::{INTROSPECTION_COMMANDS, OPERATIONS, PROJECT, STAGE, source_catalog};
use serde::Serialize;
use serde_json::{Value, json};

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
}

#[derive(Debug, Args)]
struct OutputArgs {
    /// Emit one stable JSON object on stdout.
    #[arg(long)]
    json: bool,
}

#[derive(Debug, Serialize)]
struct Envelope<'a> {
    schema_version: u8,
    ok: bool,
    command: &'a str,
    data: Value,
    verified: bool,
    submitted: bool,
}

fn success(command: &str, data: Value) -> Envelope<'_> {
    Envelope {
        schema_version: 1,
        ok: true,
        command,
        data,
        verified: false,
        submitted: false,
    }
}

fn write_bytes(bytes: &[u8]) -> Result<(), io::Error> {
    let mut stdout = io::stdout().lock();
    stdout.write_all(bytes)
}

fn write_text(text: &str) -> Result<(), io::Error> {
    write_bytes(text.as_bytes())
}

fn write_json(envelope: &Envelope<'_>) -> Result<(), io::Error> {
    let mut bytes = serde_json::to_vec(envelope).map_err(io::Error::other)?;
    bytes.push(b'\n');
    write_bytes(&bytes)
}

enum CommandError {
    Output,
    InternalReported,
}

type CommandResult = Result<(), CommandError>;

fn output(result: Result<(), io::Error>) -> CommandResult {
    result.map_err(|_| CommandError::Output)
}

fn write_diagnostic(message: &str) {
    let _ = io::stderr().lock().write_all(message.as_bytes());
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

    output(write_text(&format!(
        "{PROJECT}\nstage: {STAGE}\nimplemented operations: none\nintrospection commands: {}\nverification: not performed\nsubmission: not performed\n",
        INTROSPECTION_COMMANDS.join(", ")
    )))
}

fn sources(args: &OutputArgs) -> CommandResult {
    let catalog = match source_catalog() {
        Ok(catalog) => catalog,
        Err(_) => {
            write_diagnostic("opencegeljaras: internal source catalogue error (exit 3)\n");
            return Err(CommandError::InternalReported);
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
    };

    match result {
        Ok(()) => 0,
        Err(CommandError::Output) => {
            write_diagnostic("opencegeljaras: failed to write output (exit 3)\n");
            3
        }
        Err(CommandError::InternalReported) => 3,
    }
}

fn main() -> ExitCode {
    ExitCode::from(run())
}
