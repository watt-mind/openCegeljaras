//! The explicit `xmllint` process adapter used by `form validate`.
//!
//! Nothing in this module reads the user's path a second time, resolves a
//! network location, or lets the document choose what the child process does.
//! The validator is located explicitly, its version is checked against the
//! range whose message formats were established by review, and it is spawned
//! once, in a private temporary directory, with a cleared environment and a
//! fixed argument list. Its standard output is discarded and its standard
//! error is drained under a hard cap so that it can never block on a full
//! pipe. A run of this adapter establishes XSD conformance only: it is not a
//! signature check and not acceptance by any authority.

use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

/// The validator name reported in output. Only libxml2's tool is supported.
pub(crate) const VALIDATOR_NAME: &str = "xmllint";
/// The executable name looked up on `PATH` when `--xmllint` is not given.
const EXECUTABLE: &str = "xmllint";
/// Lowest libxml2 version whose xmllint diagnostics were established.
pub(crate) const MIN_LIBXML_VERSION: u32 = 20914;
/// First libxml2 version whose xmllint diagnostics were *not* established.
pub(crate) const MAX_LIBXML_VERSION: u32 = 21300;
/// The name the document is given inside the private temporary directory.
pub(crate) const DOCUMENT_NAME: &str = "document.xml";
/// Bytes of child standard error kept; everything beyond this is discarded.
const STDERR_CAP: usize = 1024 * 1024;
/// Marker introducing the version number on `xmllint --version` output.
const VERSION_MARKER: &str = "using libxml version ";
/// How long a poll of the child waits before looking again.
const POLL_INTERVAL: Duration = Duration::from_millis(5);
/// Stderr fragments that mean the schema graph did not load completely.
///
/// `--nonet` only makes libxml2 *warn* when it skips an absolute location, and
/// it can still exit 0 afterwards, so a run that printed any of these did not
/// answer the conformance question and is reported as a failure. With a
/// verified form closure the package 3 import audit makes this unreachable.
pub(crate) const INCOMPLETE_GRAPH_MARKERS: [&str; 3] = [
    "Attempt to load network entity",
    "failed to load external entity",
    "Schemas parser warning",
];

/// Environment variables passed through to the child on this platform.
///
/// Nothing else survives, explicitly not `LD_LIBRARY_PATH`, `DYLD_*`, or any
/// catalog variable: an xmllint that needs one of those fails closed.
#[cfg(windows)]
const ENVIRONMENT_ALLOWLIST: &[&str] =
    &["PATH", "SystemRoot", "SYSTEMROOT", "TEMP", "TMP", "PATHEXT"];
#[cfg(not(windows))]
const ENVIRONMENT_ALLOWLIST: &[&str] = &["PATH"];

/// Why the adapter could not answer the conformance question.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AdapterError {
    /// No supported validator could be located, spawned, or version-checked.
    Unavailable {
        /// The libxml2 version, when a version line was read and parsed.
        version: Option<u32>,
    },
    /// A supported validator ran but produced no conformance result.
    Failed {
        /// The libxml2 version of the validator that was run.
        version: Option<u32>,
        /// The child's exit code, absent on a timeout or a signal.
        exit_code: Option<i32>,
    },
}

/// One completed validation run.
#[derive(Debug)]
pub(crate) struct Run {
    /// The libxml2 version that produced this result.
    pub(crate) version: u32,
    /// The validator's exit code: 0 conforming, 3 nonconforming.
    pub(crate) exit_code: i32,
    /// Whether the document conformed.
    pub(crate) conforming: bool,
    /// The captured standard error, lossily decoded and capped.
    pub(crate) stderr: String,
}

/// Locate the validator: the explicit path if given, else the first on `PATH`.
///
/// The result is canonicalised so that the spawned program is fixed before the
/// child is created, and so that no relative name is resolved twice.
pub(crate) fn locate(explicit: Option<&Path>) -> Option<PathBuf> {
    if let Some(path) = explicit {
        return canonical_file(path);
    }
    let search = std::env::var_os("PATH")?;
    for directory in std::env::split_paths(&search) {
        if directory.as_os_str().is_empty() {
            continue;
        }
        for name in candidate_names() {
            if let Some(found) = canonical_file(&directory.join(name)) {
                return Some(found);
            }
        }
    }
    None
}

/// The executable file names tried in one `PATH` directory.
fn candidate_names() -> Vec<String> {
    let mut names = vec![EXECUTABLE.to_owned()];
    if cfg!(windows) {
        names.push(format!("{EXECUTABLE}.exe"));
    }
    names
}

/// Canonicalise a path that must name a regular file after following links.
fn canonical_file(path: &Path) -> Option<PathBuf> {
    let canonical = std::fs::canonicalize(path).ok()?;
    canonical.is_file().then_some(canonical)
}

/// Apply the environment allowlist to a command about to be spawned.
fn clear_environment(command: &mut Command) {
    command.env_clear();
    for name in ENVIRONMENT_ALLOWLIST {
        if let Some(value) = std::env::var_os(name) {
            command.env(name, value);
        }
    }
}

/// Read a child pipe to its end, keeping at most the cap and discarding more.
fn drain(mut source: impl Read) -> Vec<u8> {
    let mut kept = Vec::new();
    let mut chunk = [0u8; 8192];
    loop {
        let read = match source.read(&mut chunk) {
            Ok(0) | Err(_) => break,
            Ok(read) => read,
        };
        if kept.len() < STDERR_CAP {
            let room = STDERR_CAP - kept.len();
            kept.extend_from_slice(&chunk[..read.min(room)]);
        }
    }
    kept
}

/// Wait for a child under a deadline, killing and reaping it on a timeout.
///
/// `None` means the deadline passed. `Some(None)` means the child ended
/// without an exit code, which on Unix is a signal.
fn wait_until(child: &mut Child, deadline: Option<Instant>) -> Option<Option<i32>> {
    loop {
        match child.try_wait() {
            Ok(Some(status)) => return Some(status.code()),
            Err(_) => return Some(None),
            Ok(None) => {}
        }
        if deadline.is_none_or(|deadline| Instant::now() >= deadline) {
            let _ = child.kill();
            let _ = child.wait();
            return None;
        }
        thread::sleep(POLL_INTERVAL);
    }
}

/// Collect the drained standard error, giving up at the same deadline.
///
/// A grandchild that inherited the pipe keeps it open after the child itself
/// has been reaped, so the reader is bounded exactly as the child is. Past the
/// deadline the thread is abandoned rather than joined: it holds only its own
/// buffer and one pipe end, and the process is about to report a failure.
fn collect_until(reader: JoinHandle<Vec<u8>>, deadline: Option<Instant>) -> Option<Vec<u8>> {
    loop {
        if reader.is_finished() {
            return reader.join().ok();
        }
        if deadline.is_none_or(|deadline| Instant::now() >= deadline) {
            return None;
        }
        thread::sleep(POLL_INTERVAL);
    }
}

/// Spawn one prepared command and collect its exit code and standard error.
///
/// `None` covers a failed spawn, a timeout, and standard error that stayed
/// open past the deadline alike; no child is left behind in any of them.
fn execute(command: &mut Command, timeout: Duration) -> Option<(Option<i32>, String)> {
    clear_environment(command);
    let mut child = command
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .ok()?;
    let Some(stderr) = child.stderr.take() else {
        let _ = child.kill();
        let _ = child.wait();
        return None;
    };
    let reader = thread::spawn(move || drain(stderr));
    let deadline = Instant::now().checked_add(timeout);
    let code = wait_until(&mut child, deadline);
    let captured = collect_until(reader, deadline)?;
    code.map(|code| (code, String::from_utf8_lossy(&captured).into_owned()))
}

/// Read `N` from the `using libxml version N` line of a version report.
fn parse_version(report: &str) -> Option<u32> {
    for line in report.lines() {
        let Some((_, rest)) = line.split_once(VERSION_MARKER) else {
            continue;
        };
        let digits: String = rest.chars().take_while(char::is_ascii_digit).collect();
        if let Ok(version) = digits.parse::<u32>() {
            return Some(version);
        }
    }
    None
}

/// Ask one located validator for its libxml2 version and accept the range.
///
/// The probe runs in the same private directory as the validation itself, so
/// that no child of this process ever runs in the operator's own working
/// directory or can leave a file there.
fn supported_version(exe: &Path, directory: &Path, timeout: Duration) -> Result<u32, AdapterError> {
    let unavailable = |version| AdapterError::Unavailable { version };
    let (_, report) = execute(
        Command::new(exe).current_dir(directory).arg("--version"),
        timeout,
    )
    .ok_or_else(|| unavailable(None))?;
    let version = parse_version(&report).ok_or_else(|| unavailable(None))?;
    if (MIN_LIBXML_VERSION..MAX_LIBXML_VERSION).contains(&version) {
        Ok(version)
    } else {
        Err(unavailable(Some(version)))
    }
}

/// Validate one document's bytes against one canonical schema root.
///
/// The bytes are the ones the `form inspect` boundary already accepted; they
/// are written into a private temporary directory so that the validator never
/// re-reads the operator's file and never sees its path.
pub(crate) fn run(
    exe: &Path,
    root: &Path,
    document: &[u8],
    timeout: Duration,
) -> Result<Run, AdapterError> {
    let directory = tempfile::TempDir::new().map_err(|_| AdapterError::Failed {
        version: None,
        exit_code: None,
    })?;
    let version = supported_version(exe, directory.path(), timeout)?;
    let failed = move |exit_code| AdapterError::Failed {
        version: Some(version),
        exit_code,
    };
    let document_path = directory.path().join(DOCUMENT_NAME);
    std::fs::write(&document_path, document).map_err(|_| failed(None))?;

    let mut command = Command::new(exe);
    command
        .current_dir(directory.path())
        .arg("--nonet")
        .arg("--noout")
        .arg("--schema")
        .arg(root)
        .arg(DOCUMENT_NAME);
    let (code, stderr) = execute(&mut command, timeout).ok_or_else(|| failed(None))?;
    if stderr.lines().any(|line| {
        INCOMPLETE_GRAPH_MARKERS
            .iter()
            .any(|mark| line.contains(mark))
    }) {
        return Err(failed(code));
    }
    match code {
        Some(0) => Ok(Run {
            version,
            exit_code: 0,
            conforming: true,
            stderr,
        }),
        Some(3) => Ok(Run {
            version,
            exit_code: 3,
            conforming: false,
            stderr,
        }),
        other => Err(failed(other)),
    }
}

#[cfg(test)]
mod tests;
