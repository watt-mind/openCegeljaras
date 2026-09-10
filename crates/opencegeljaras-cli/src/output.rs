//! Stable JSON envelopes, plain-text writing, and command failure kinds.

use std::io::{self, Write};

use serde::Serialize;
use serde_json::Value;

/// The JSON envelope shared by every command that produces one.
#[derive(Debug, Serialize)]
pub(crate) struct Envelope<'a> {
    schema_version: u8,
    ok: bool,
    command: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<ErrorBody<'a>>,
    data: Value,
    verified: bool,
    submitted: bool,
}

/// The fixed error description carried by a failing envelope.
#[derive(Debug, Serialize)]
struct ErrorBody<'a> {
    code: &'a str,
    message: &'a str,
}

/// A successful envelope. `ok` never describes validity, filing, or legality.
pub(crate) fn success<'a>(command: &'a str, data: Value) -> Envelope<'a> {
    Envelope {
        schema_version: 1,
        ok: true,
        command,
        error: None,
        data,
        verified: false,
        submitted: false,
    }
}

/// A failing envelope carrying a fixed code and message.
pub(crate) fn failure<'a>(
    command: &'a str,
    code: &'a str,
    message: &'a str,
    data: Value,
) -> Envelope<'a> {
    Envelope {
        schema_version: 1,
        ok: false,
        command,
        error: Some(ErrorBody { code, message }),
        data,
        verified: false,
        submitted: false,
    }
}

fn write_bytes(bytes: &[u8]) -> Result<(), io::Error> {
    let mut stdout = io::stdout().lock();
    stdout.write_all(bytes)
}

/// Write plain text to standard output.
pub(crate) fn write_text(text: &str) -> Result<(), io::Error> {
    write_bytes(text.as_bytes())
}

/// Write one envelope and a newline to standard output.
pub(crate) fn write_json(envelope: &Envelope<'_>) -> Result<(), io::Error> {
    let mut bytes = serde_json::to_vec(envelope).map_err(io::Error::other)?;
    bytes.push(b'\n');
    write_bytes(&bytes)
}

/// Why a command did not complete normally.
pub(crate) enum CommandError {
    /// Standard output could not be written.
    Output,
    /// The command already reported a failure and chose an exit code.
    Reported(u8),
}

/// The result of running one command.
pub(crate) type CommandResult = Result<(), CommandError>;

/// Turn a write failure into the output exit code.
pub(crate) fn output(result: Result<(), io::Error>) -> CommandResult {
    result.map_err(|_| CommandError::Output)
}

/// Write one fixed diagnostic line to standard error.
pub(crate) fn write_diagnostic(message: &str) {
    let _ = io::stderr().lock().write_all(message.as_bytes());
}
