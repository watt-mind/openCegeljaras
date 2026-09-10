//! A fake xmllint built out of the running test executable itself.
//!
//! No extra binary ships. A case directory holds a control file with a fixed
//! name and a tiny launcher; `--xmllint` is pointed at the launcher, which
//! records the arguments it was given and re-executes the test executable
//! through `std::env::current_exe()` with the marker test name
//! `fake_validator_entry`. That test calls [`dispatch`], which reads the
//! control file and behaves as the case demands: it reports a chosen version,
//! writes chosen standard-error lines, floods, sleeps, and exits with a chosen
//! code, while recording its working directory, its environment, and the
//! document it was handed.
//!
//! Everything here is synthetic. Nothing private is read and nothing is
//! fetched.

use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::Duration;

/// Environment variable naming the control file, set by the launcher.
const SCRIPT_VARIABLE: &str = "OPENCEGELJARAS_FAKE_SCRIPT";
/// Environment variable naming the directory recordings are written to.
const RECORD_VARIABLE: &str = "OPENCEGELJARAS_FAKE_RECORD";
/// Environment variable telling the fake which invocation this is.
const MODE_VARIABLE: &str = "OPENCEGELJARAS_FAKE_MODE";
/// The fixed name of the control file inside a case directory.
pub const SCRIPT_NAME: &str = "fake-validator.script";
/// The marker test name the launcher re-executes.
pub const MARKER: &str = "fake_validator_entry";
/// The version line reported when a case does not choose one.
pub const DEFAULT_VERSION: &str = "xmllint: using libxml version 20914";

/// One prepared fake-validator case.
pub struct Case {
    /// The case directory holding the control file and the recordings.
    pub directory: PathBuf,
    /// The launcher to pass to `--xmllint`.
    pub exe: PathBuf,
}

impl Case {
    /// The arguments of one recorded invocation: `version` or `run`.
    pub fn argv(&self, which: &str) -> Vec<String> {
        let recorded = fs::read_to_string(self.directory.join(format!("argv-{which}")))
            .unwrap_or_else(|_| panic!("the fake should have recorded its {which} arguments"));
        recorded.lines().map(str::to_owned).collect()
    }

    /// The working directory the fake was spawned in.
    pub fn cwd(&self) -> PathBuf {
        PathBuf::from(
            fs::read_to_string(self.directory.join("cwd"))
                .expect("the fake should have recorded its working directory"),
        )
    }

    /// The environment variable names the fake inherited, sorted.
    pub fn environment(&self) -> Vec<String> {
        let recorded = fs::read_to_string(self.directory.join("environment"))
            .expect("the fake should have recorded its environment");
        recorded.lines().map(str::to_owned).collect()
    }

    /// The bytes of the document copy the fake was handed.
    pub fn document(&self) -> Vec<u8> {
        fs::read(self.directory.join("document"))
            .expect("the fake should have recorded the document it was given")
    }
}

/// A fresh per-process scratch directory.
pub fn scratch(name: &str) -> PathBuf {
    static COUNTER: AtomicU32 = AtomicU32::new(0);
    let directory = std::env::temp_dir().join(format!(
        "opencegeljaras-fake-{}-{}-{name}",
        std::process::id(),
        COUNTER.fetch_add(1, Ordering::Relaxed)
    ));
    fs::create_dir_all(&directory).expect("a scratch directory should be creatable");
    directory
}

/// The one launcher shared by every case in this process.
///
/// It is written once and then hard-linked into each case directory. Writing
/// one executable per case raced with the fork of another case's child, which
/// inherited the open write descriptor and made the `exec` fail with
/// `ETXTBSY`; the settle below closes that window for good, because once the
/// write descriptor is closed no later child can inherit it.
fn launcher() -> &'static PathBuf {
    static LAUNCHER: OnceLock<PathBuf> = OnceLock::new();
    LAUNCHER.get_or_init(|| {
        let current = std::env::current_exe().expect("the test executable should have a path");
        let path = scratch("launcher").join("fake-xmllint");
        let script = format!(
            "#!/bin/sh\n\
             D=\"$(cd \"$(dirname \"$0\")\" && pwd)\"\n\
             if [ \"$1\" = \"--version\" ]; then M=version; F=argv-version; \
             else M=run; F=argv-run; fi\n\
             for a in \"$@\"; do printf '%s\\n' \"$a\"; done > \"$D/$F\"\n\
             {SCRIPT_VARIABLE}=\"$D/{SCRIPT_NAME}\" \
             {RECORD_VARIABLE}=\"$D\" \
             {MODE_VARIABLE}=\"$M\" \
             exec \"{}\" --nocapture --test-threads=1 {MARKER}\n",
            current.display()
        );
        fs::write(&path, script).expect("the launcher should be writable");
        make_executable(&path);
        std::thread::sleep(Duration::from_millis(100));
        path
    })
}

/// Prepare one case directory, its control file, and its launcher link.
pub fn case(name: &str, script: &str) -> Case {
    let directory = scratch(name);
    fs::write(directory.join(SCRIPT_NAME), script).expect("the control file should be writable");
    let exe = directory.join("fake-xmllint");
    fs::hard_link(launcher(), &exe).expect("the launcher should be linkable");
    Case { directory, exe }
}

/// Give the launcher the executable bit it needs to be spawned.
fn make_executable(path: &std::path::Path) {
    use std::os::unix::fs::PermissionsExt;
    let mut permissions = fs::metadata(path)
        .expect("the launcher should exist")
        .permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(path, permissions).expect("the launcher should be made executable");
}

/// Behave as the fake validator when the launcher re-executed this binary.
///
/// Returns immediately during an ordinary test run, when no control file is
/// named, so the marker test is a no-op unless a case invoked it.
pub fn dispatch() {
    let Ok(script) = std::env::var(SCRIPT_VARIABLE) else {
        return;
    };
    let record = PathBuf::from(std::env::var(RECORD_VARIABLE).unwrap_or_default());
    let version_probe = std::env::var(MODE_VARIABLE).as_deref() == Ok("version");
    let control = fs::read_to_string(&script).expect("the control file should be readable");

    let mut version = String::from(DEFAULT_VERSION);
    let mut lines: Vec<String> = Vec::new();
    let mut exit = 0;
    let mut sleep = 0;
    let mut flood = 0;
    for entry in control.lines() {
        let Some((key, value)) = entry.split_once('=') else {
            continue;
        };
        match key {
            "version" => version = value.to_owned(),
            "stderr" => lines.push(value.to_owned()),
            "exit" => exit = value.parse().expect("exit must be a number"),
            "sleep_ms" => sleep = value.parse().expect("sleep_ms must be a number"),
            "flood" => flood = value.parse().expect("flood must be a number"),
            _ => {}
        }
    }

    if version_probe {
        report(&[version], 0, 0);
    }
    record_invocation(&record);
    if sleep > 0 {
        std::thread::sleep(Duration::from_millis(sleep));
    }
    report(&lines, flood, exit);
}

/// Record what the child was given, so that a case can assert on it.
fn record_invocation(record: &std::path::Path) {
    let cwd = std::env::current_dir().expect("the fake should have a working directory");
    fs::write(record.join("cwd"), cwd.as_os_str().as_encoded_bytes())
        .expect("the working directory should be recordable");
    let mut names: Vec<String> = std::env::vars()
        .map(|(name, _)| name)
        .filter(|name| ![SCRIPT_VARIABLE, RECORD_VARIABLE, MODE_VARIABLE].contains(&name.as_str()))
        .collect();
    names.sort();
    fs::write(record.join("environment"), names.join("\n"))
        .expect("the environment should be recordable");
    if let Ok(document) = fs::read("document.xml") {
        fs::write(record.join("document"), document).expect("the document should be recordable");
    }
}

/// Write the chosen standard error, flood it, and exit with the chosen code.
fn report(lines: &[String], flood: usize, exit: i32) -> ! {
    let mut stderr = std::io::stderr().lock();
    for line in lines {
        let _ = writeln!(stderr, "{line}");
    }
    if flood > 0 {
        let chunk = "flood ".repeat(1024);
        let mut written = 0;
        while written < flood {
            let _ = write!(stderr, "{chunk}");
            written += chunk.len();
        }
    }
    let _ = stderr.flush();
    std::process::exit(exit);
}
