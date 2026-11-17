//! Integration tests for `ricette list add|show|clear`.
//!
//! Each test runs the built binary in a fresh temporary working directory and
//! checks stdout, stderr, the exit code and the `ricette-list.json` file.

use std::fs;
use std::path::Path;
use std::process::{Command, Output};

use serde_json::Value;
use tempfile::TempDir;

const LIST_FILE: &str = "ricette-list.json";

/// Runs `ricette` with `args` in `dir`. `HOME` points at `home`, so a test can
/// check that nothing is written there.
fn run(dir: &Path, home: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_ricette"))
        .args(args)
        .current_dir(dir)
        .env("HOME", home)
        .output()
        .expect("failed to run the ricette binary")
}

fn stdout(output: &Output) -> String {
    String::from_utf8(output.stdout.clone()).expect("stdout is UTF-8")
}

fn stderr(output: &Output) -> String {
    String::from_utf8(output.stderr.clone()).expect("stderr is UTF-8")
}

fn exit_code(output: &Output) -> i32 {
    output.status.code().expect("process exited with a code")
}

/// Parses `ricette-list.json` in `dir` and returns its items in order.
fn read_items(dir: &Path) -> Vec<String> {
    let text = fs::read_to_string(dir.join(LIST_FILE)).expect("list file exists");
    let value: Value = serde_json::from_str(&text).expect("list file is valid JSON");
    let entries = value["entries"].as_array().expect("`entries` is an array");
    entries
        .iter()
        .map(|entry| {
            entry["item"]
                .as_str()
                .expect("each entry has a string `item`")
                .to_string()
        })
        .collect()
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

    fn run(&self, args: &[&str]) -> Output {
        run(self.path(), self.home.path(), args)
    }

    fn write_list(&self, contents: &[u8]) {
        fs::write(self.path().join(LIST_FILE), contents).expect("write list file");
    }
}

#[test]
fn add_in_empty_directory_prints_confirmation_and_creates_one_entry() {
    let sandbox = Sandbox::new();

    let output = sandbox.run(&["list", "add", "milk"]);

    assert_eq!(exit_code(&output), 0);
    assert_eq!(stdout(&output), "Added: milk\n");
    assert_eq!(read_items(sandbox.path()), vec!["milk".to_string()]);
}

#[test]
fn show_prints_items_in_insertion_order_one_per_line() {
    let sandbox = Sandbox::new();
    sandbox.run(&["list", "add", "milk"]);
    sandbox.run(&["list", "add", "eggs"]);

    let output = sandbox.run(&["list", "show"]);

    assert_eq!(exit_code(&output), 0);
    assert_eq!(stdout(&output), "milk\neggs\n");
}

#[test]
fn adding_the_same_item_twice_keeps_both_entries() {
    let sandbox = Sandbox::new();
    sandbox.run(&["list", "add", "milk"]);
    sandbox.run(&["list", "add", "eggs"]);

    let output = sandbox.run(&["list", "add", "milk"]);
    assert_eq!(exit_code(&output), 0);
    assert_eq!(stdout(&output), "Added: milk\n");
    assert_eq!(
        read_items(sandbox.path()),
        vec!["milk".to_string(), "eggs".to_string(), "milk".to_string()]
    );

    let shown = sandbox.run(&["list", "show"]);
    assert_eq!(stdout(&shown), "milk\neggs\nmilk\n");
}

#[test]
fn item_comparison_is_case_sensitive() {
    let sandbox = Sandbox::new();
    sandbox.run(&["list", "add", "Milk"]);
    sandbox.run(&["list", "add", "milk"]);

    assert_eq!(
        read_items(sandbox.path()),
        vec!["Milk".to_string(), "milk".to_string()]
    );
}

#[test]
fn show_without_a_list_file_says_the_list_is_empty() {
    let sandbox = Sandbox::new();

    let output = sandbox.run(&["list", "show"]);

    assert_eq!(exit_code(&output), 0);
    assert_eq!(stdout(&output), "The shopping list is empty.\n");
    assert!(!sandbox.path().join(LIST_FILE).exists());
}

#[test]
fn clear_empties_the_list_and_show_then_says_it_is_empty() {
    let sandbox = Sandbox::new();
    sandbox.run(&["list", "add", "milk"]);

    let cleared = sandbox.run(&["list", "clear"]);
    assert_eq!(exit_code(&cleared), 0);
    assert_eq!(stdout(&cleared), "Shopping list cleared.\n");
    assert_eq!(read_items(sandbox.path()), Vec::<String>::new());

    let shown = sandbox.run(&["list", "show"]);
    assert_eq!(exit_code(&shown), 0);
    assert_eq!(stdout(&shown), "The shopping list is empty.\n");
}

#[test]
fn clear_without_a_list_file_creates_an_empty_list() {
    let sandbox = Sandbox::new();

    let output = sandbox.run(&["list", "clear"]);

    assert_eq!(exit_code(&output), 0);
    assert_eq!(stdout(&output), "Shopping list cleared.\n");
    assert_eq!(read_items(sandbox.path()), Vec::<String>::new());
}

#[test]
fn list_file_is_read_and_written_in_the_working_directory_only() {
    let sandbox = Sandbox::new();

    sandbox.run(&["list", "add", "milk"]);

    assert!(sandbox.path().join(LIST_FILE).is_file());
    assert!(
        !sandbox.home.path().join(LIST_FILE).exists(),
        "the list must not be written to the home directory"
    );
}

#[test]
fn show_with_a_malformed_list_fails_and_leaves_the_file_unchanged() {
    let sandbox = Sandbox::new();
    let malformed = b"{\"entries\": [{\"item\": \"milk\"";
    sandbox.write_list(malformed);

    let output = sandbox.run(&["list", "show"]);

    assert_eq!(exit_code(&output), 1);
    assert!(
        stderr(&output).contains("cannot be read"),
        "stderr should report the shopping list cannot be read, got: {}",
        stderr(&output)
    );
    assert_eq!(stdout(&output), "");
    assert_eq!(fs::read(sandbox.path().join(LIST_FILE)).unwrap(), malformed);
}

#[test]
fn add_with_a_malformed_list_fails_and_leaves_the_file_unchanged() {
    let sandbox = Sandbox::new();
    let malformed = b"not json at all";
    sandbox.write_list(malformed);

    let output = sandbox.run(&["list", "add", "milk"]);

    assert_eq!(exit_code(&output), 1);
    assert!(!stderr(&output).is_empty(), "stderr should report an error");
    assert_eq!(fs::read(sandbox.path().join(LIST_FILE)).unwrap(), malformed);
}

#[test]
fn clear_with_a_malformed_list_fails_and_leaves_the_file_unchanged() {
    let sandbox = Sandbox::new();
    let malformed = b"{\"entries\": 42}";
    sandbox.write_list(malformed);

    let output = sandbox.run(&["list", "clear"]);

    assert_eq!(exit_code(&output), 1);
    assert!(!stderr(&output).is_empty(), "stderr should report an error");
    assert_eq!(fs::read(sandbox.path().join(LIST_FILE)).unwrap(), malformed);
}

#[test]
fn adding_an_empty_item_is_a_usage_error_and_creates_no_file() {
    let sandbox = Sandbox::new();

    let output = sandbox.run(&["list", "add", ""]);

    assert_eq!(exit_code(&output), 2);
    assert!(
        !stderr(&output).is_empty(),
        "stderr should show a usage error"
    );
    assert!(!sandbox.path().join(LIST_FILE).exists());
}

#[test]
fn adding_an_empty_item_leaves_an_existing_list_unchanged() {
    let sandbox = Sandbox::new();
    sandbox.run(&["list", "add", "milk"]);
    let before = fs::read(sandbox.path().join(LIST_FILE)).unwrap();

    let output = sandbox.run(&["list", "add", ""]);

    assert_eq!(exit_code(&output), 2);
    assert_eq!(fs::read(sandbox.path().join(LIST_FILE)).unwrap(), before);
}

#[test]
fn glossary_defines_the_domain_terms() {
    let glossary = fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("GLOSSARY.md"))
        .expect("GLOSSARY.md exists at the repository root");

    for term in [
        "item",
        "shopping list",
        "shopping list entry",
        "quantity",
        "unit",
        "recipe",
        "ingredient",
    ] {
        let prefix = format!("- **{term}**:");
        let definition = glossary
            .lines()
            .map(str::trim)
            .find(|line| line.to_lowercase().starts_with(&prefix))
            .unwrap_or_else(|| panic!("GLOSSARY.md has no line defining `{term}`"))
            .to_lowercase();
        let text = definition[prefix.len()..].trim();
        assert!(!text.is_empty(), "the definition of `{term}` is empty");
    }
}
