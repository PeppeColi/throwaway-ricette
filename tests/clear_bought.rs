//! Integration tests for `ricette list clear-bought`: it removes every bought
//! entry of the shopping list and keeps the others in their original order.
//!
//! Each test runs the built binary in a fresh temporary working directory and
//! checks stdout, stderr, the exit code and the `ricette-list.json` file.

use std::fs;
use std::path::Path;
use std::process::{Command, Output};

use serde_json::Value;
use tempfile::TempDir;

const LIST_FILE: &str = "ricette-list.json";

/// A `ricette-list.json` written by the previous version, without `bought`.
const LEGACY_LIST: &str = r#"{
  "entries": [
    { "item": "pasta", "quantity": 200, "unit": "g" },
    { "item": "milk" }
  ]
}
"#;

fn stdout(output: &Output) -> String {
    String::from_utf8(output.stdout.clone()).expect("stdout is UTF-8")
}

fn stderr(output: &Output) -> String {
    String::from_utf8(output.stderr.clone()).expect("stderr is UTF-8")
}

fn exit_code(output: &Output) -> i32 {
    output.status.code().expect("process exited with a code")
}

struct Sandbox {
    dir: TempDir,
    home: TempDir,
}

impl Sandbox {
    fn new() -> Self {
        Sandbox {
            dir: tempfile::tempdir().expect("temporary working directory"),
            home: tempfile::tempdir().expect("temporary home directory"),
        }
    }

    fn path(&self) -> &Path {
        self.dir.path()
    }

    fn list_path(&self) -> std::path::PathBuf {
        self.path().join(LIST_FILE)
    }

    fn run(&self, args: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_ricette"))
            .args(args)
            .current_dir(self.path())
            .env("HOME", self.home.path())
            .output()
            .expect("failed to run the ricette binary")
    }

    /// Runs `args` and asserts that the command succeeded.
    fn ok(&self, args: &[&str]) -> Output {
        let output = self.run(args);
        assert_eq!(
            exit_code(&output),
            0,
            "{args:?} failed: {}",
            stderr(&output)
        );
        output
    }

    fn write_list(&self, contents: &[u8]) {
        fs::write(self.list_path(), contents).expect("write list file");
    }

    fn read_list(&self) -> Vec<u8> {
        fs::read(self.list_path()).expect("list file exists")
    }

    /// The entries of `ricette-list.json`, in order, as JSON.
    fn entries(&self) -> Vec<Value> {
        let value: Value =
            serde_json::from_slice(&self.read_list()).expect("list file is valid JSON");
        value["entries"]
            .as_array()
            .expect("`entries` is an array")
            .clone()
    }

    fn show(&self) -> String {
        stdout(&self.ok(&["list", "show"]))
    }

    /// The names of the files in the working directory, sorted.
    fn files(&self) -> Vec<String> {
        let mut names: Vec<String> = fs::read_dir(self.path())
            .expect("read working directory")
            .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        names.sort();
        names
    }
}

/// A sandbox with plain `milk` and `eggs`.
fn milk_and_eggs() -> Sandbox {
    let sandbox = Sandbox::new();
    sandbox.ok(&["list", "add", "milk"]);
    sandbox.ok(&["list", "add", "eggs"]);
    sandbox
}

/// A sandbox with plain `milk`, `eggs`, `bread` and `pasta`.
fn four_plain_items() -> Sandbox {
    let sandbox = Sandbox::new();
    for item in ["milk", "eggs", "bread", "pasta"] {
        sandbox.ok(&["list", "add", item]);
    }
    sandbox
}

// --- removing bought entries ---

#[test]
fn clear_bought_removes_one_bought_entry_and_keeps_the_rest() {
    let sandbox = milk_and_eggs();
    sandbox.ok(&["list", "add", "bread"]);
    sandbox.ok(&["list", "bought", "milk"]);
    sandbox.ok(&["list", "bought", "eggs"]);

    let output = sandbox.run(&["list", "clear-bought"]);

    assert_eq!(exit_code(&output), 0);
    assert_eq!(stdout(&output), "Removed 2 bought entries.\n");
    assert_eq!(stderr(&output), "");
    assert_eq!(sandbox.show(), "bread\n");
}

#[test]
fn clear_bought_reports_a_single_bought_entry_in_the_singular() {
    let sandbox = milk_and_eggs();
    sandbox.ok(&["list", "bought", "milk"]);

    let output = sandbox.run(&["list", "clear-bought"]);

    assert_eq!(exit_code(&output), 0);
    assert_eq!(stdout(&output), "Removed 1 bought entry.\n");
    assert_eq!(sandbox.show(), "eggs\n");
}

#[test]
fn clear_bought_keeps_unbought_entries_in_their_original_position() {
    let sandbox = four_plain_items();
    sandbox.ok(&["list", "bought", "milk"]);
    sandbox.ok(&["list", "bought", "bread"]);

    let output = sandbox.run(&["list", "clear-bought"]);

    assert_eq!(exit_code(&output), 0);
    assert_eq!(stdout(&output), "Removed 2 bought entries.\n");
    assert_eq!(
        sandbox.entries(),
        vec![
            serde_json::json!({ "item": "eggs" }),
            serde_json::json!({ "item": "pasta" }),
        ]
    );
}

#[test]
fn clear_bought_removes_quantified_bought_entries_and_keeps_others_intact() {
    let sandbox = Sandbox::new();
    sandbox.ok(&["list", "add", "pasta", "--qty", "200", "--unit", "g"]);
    sandbox.ok(&["list", "add", "tomato", "--qty", "400", "--unit", "g"]);
    sandbox.ok(&["list", "bought", "pasta"]);

    let output = sandbox.run(&["list", "clear-bought"]);

    assert_eq!(exit_code(&output), 0);
    assert_eq!(stdout(&output), "Removed 1 bought entry.\n");
    assert_eq!(
        sandbox.entries(),
        vec![serde_json::json!({ "item": "tomato", "quantity": 400, "unit": "g" })]
    );
}

#[test]
fn clear_bought_when_all_entries_are_bought_empties_the_list() {
    let sandbox = milk_and_eggs();
    sandbox.ok(&["list", "bought", "milk"]);
    sandbox.ok(&["list", "bought", "eggs"]);

    let output = sandbox.run(&["list", "clear-bought"]);

    assert_eq!(exit_code(&output), 0);
    assert_eq!(stdout(&output), "Removed 2 bought entries.\n");
    assert!(sandbox.entries().is_empty());
    assert_eq!(sandbox.show(), "The shopping list is empty.\n");
}

// --- nothing to remove ---

#[test]
fn clear_bought_with_no_bought_entries_leaves_the_file_unchanged() {
    let sandbox = milk_and_eggs();
    let before = sandbox.read_list();

    let output = sandbox.run(&["list", "clear-bought"]);

    assert_eq!(exit_code(&output), 0);
    assert_eq!(stdout(&output), "No bought entries.\n");
    assert_eq!(sandbox.read_list(), before);
    assert_eq!(sandbox.files(), vec![LIST_FILE.to_string()]);
}

#[test]
fn clear_bought_without_a_list_file_creates_no_file() {
    let sandbox = Sandbox::new();

    let output = sandbox.run(&["list", "clear-bought"]);

    assert_eq!(exit_code(&output), 0);
    assert_eq!(stdout(&output), "No bought entries.\n");
    assert_eq!(stderr(&output), "");
    assert!(sandbox.files().is_empty());
}

#[test]
fn clear_bought_on_an_empty_list_leaves_the_file_unchanged() {
    let sandbox = Sandbox::new();
    sandbox.write_list(b"{\"entries\": []}\n");

    let output = sandbox.run(&["list", "clear-bought"]);

    assert_eq!(exit_code(&output), 0);
    assert_eq!(stdout(&output), "No bought entries.\n");
    assert_eq!(sandbox.read_list(), b"{\"entries\": []}\n");
}

// --- data compatibility ---

#[test]
fn clear_bought_on_a_legacy_list_without_bought_fields_changes_nothing() {
    let sandbox = Sandbox::new();
    sandbox.write_list(LEGACY_LIST.as_bytes());

    let output = sandbox.run(&["list", "clear-bought"]);

    assert_eq!(exit_code(&output), 0);
    assert_eq!(stdout(&output), "No bought entries.\n");
    assert_eq!(sandbox.read_list(), LEGACY_LIST.as_bytes());
}

#[test]
fn clear_bought_on_a_legacy_list_removes_only_the_entries_marked_bought() {
    let sandbox = Sandbox::new();
    sandbox.write_list(LEGACY_LIST.as_bytes());
    sandbox.ok(&["list", "bought", "pasta"]);

    let output = sandbox.run(&["list", "clear-bought"]);

    assert_eq!(exit_code(&output), 0);
    assert_eq!(stdout(&output), "Removed 1 bought entry.\n");
    assert_eq!(
        sandbox.entries(),
        vec![serde_json::json!({ "item": "milk" })]
    );
    assert_eq!(sandbox.show(), "milk\n");
}

// --- malformed list ---

#[test]
fn clear_bought_with_a_malformed_list_fails_and_leaves_the_file_unchanged() {
    let sandbox = Sandbox::new();
    let malformed = b"{\"entries\": [{\"item\": \"pasta\"";
    sandbox.write_list(malformed);

    let output = sandbox.run(&["list", "clear-bought"]);

    assert_eq!(exit_code(&output), 1);
    assert!(
        stderr(&output).contains("cannot be read"),
        "stderr should report the shopping list cannot be read, got: {}",
        stderr(&output)
    );
    assert_eq!(stdout(&output), "");
    assert_eq!(sandbox.read_list(), malformed);
    assert_eq!(sandbox.files(), vec![LIST_FILE.to_string()]);
}

// --- temporary files ---

#[test]
fn clear_bought_leaves_no_temporary_file_behind() {
    let sandbox = four_plain_items();
    sandbox.ok(&["list", "bought", "milk"]);

    let output = sandbox.run(&["list", "clear-bought"]);

    assert_eq!(exit_code(&output), 0);
    assert_eq!(sandbox.files(), vec![LIST_FILE.to_string()]);
}

// --- other commands ---

#[test]
fn list_bought_and_show_keep_working_after_clear_bought() {
    let sandbox = milk_and_eggs();
    sandbox.ok(&["list", "bought", "eggs"]);

    let output = sandbox.run(&["list", "clear-bought"]);

    assert_eq!(exit_code(&output), 0);
    assert_eq!(sandbox.show(), "milk\n");
    let marked = sandbox.run(&["list", "bought", "milk"]);
    assert_eq!(exit_code(&marked), 0);
    assert_eq!(stdout(&marked), "Bought: milk\n");
    assert_eq!(sandbox.show(), "milk [bought]\n");
}
