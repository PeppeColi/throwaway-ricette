//! Integration tests for the bought state of shopping list entries:
//! `ricette list bought`, the `[bought]` mark in `list show`, and how `add`,
//! `add-recipe`, `count`, `last` and `clear` treat bought entries.
//!
//! Each test runs the built binary in a fresh temporary working directory and
//! checks stdout, stderr, the exit code and the `ricette-list.json` file.

use std::fs;
use std::path::Path;
use std::process::{Command, Output};

use serde_json::{Value, json};
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

/// A sandbox with `pasta 200 g`, `tomato 400 g` and `garlic 2 cloves`.
fn pasta_tomato_garlic() -> Sandbox {
    let sandbox = Sandbox::new();
    sandbox.ok(&["list", "add", "pasta", "--qty", "200", "--unit", "g"]);
    sandbox.ok(&["list", "add", "tomato", "--qty", "400", "--unit", "g"]);
    sandbox.ok(&["list", "add", "garlic", "--qty", "2", "--unit", "cloves"]);
    sandbox
}

// --- data compatibility ---

#[test]
fn a_legacy_list_is_shown_without_marks() {
    let sandbox = Sandbox::new();
    sandbox.write_list(LEGACY_LIST.as_bytes());

    let output = sandbox.run(&["list", "show"]);

    assert_eq!(exit_code(&output), 0);
    assert_eq!(stdout(&output), "pasta 200 g\nmilk\n");
}

#[test]
fn bought_on_a_legacy_list_writes_the_field_only_on_the_bought_entry() {
    let sandbox = Sandbox::new();
    sandbox.write_list(LEGACY_LIST.as_bytes());

    let output = sandbox.run(&["list", "bought", "pasta"]);

    assert_eq!(exit_code(&output), 0);
    assert_eq!(stdout(&output), "Bought: pasta\n");
    assert_eq!(
        sandbox.entries(),
        vec![
            json!({"item": "pasta", "quantity": 200, "unit": "g", "bought": true}),
            json!({"item": "milk"}),
        ]
    );
    assert_eq!(sandbox.show(), "pasta 200 g [bought]\nmilk\n");
}

#[test]
fn add_on_a_legacy_list_works_and_keeps_entries_without_the_field() {
    let sandbox = Sandbox::new();
    sandbox.write_list(LEGACY_LIST.as_bytes());

    let output = sandbox.run(&["list", "add", "eggs"]);

    assert_eq!(exit_code(&output), 0);
    assert_eq!(
        sandbox.entries(),
        vec![
            json!({"item": "pasta", "quantity": 200, "unit": "g"}),
            json!({"item": "milk"}),
            json!({"item": "eggs"}),
        ]
    );
}

#[test]
fn add_recipe_on_a_legacy_list_works_and_writes_no_bought_field() {
    let sandbox = Sandbox::new();
    sandbox.write_list(LEGACY_LIST.as_bytes());

    let output = sandbox.run(&["list", "add-recipe", "Tomato Pasta"]);

    assert_eq!(exit_code(&output), 0);
    let entries = sandbox.entries();
    assert_eq!(
        entries[0],
        json!({"item": "pasta", "quantity": 400, "unit": "g"})
    );
    assert_eq!(entries[1], json!({"item": "milk"}));
    assert!(
        entries.iter().all(|entry| entry.get("bought").is_none()),
        "no entry is bought: {entries:?}"
    );
}

#[test]
fn an_explicit_false_bought_parses_as_unbought_and_is_dropped_on_save() {
    let sandbox = Sandbox::new();
    sandbox.write_list(br#"{"entries": [{"item": "milk", "bought": false}]}"#);

    assert_eq!(sandbox.show(), "milk\n");
    sandbox.ok(&["list", "add", "eggs"]);

    assert_eq!(
        sandbox.entries(),
        vec![json!({"item": "milk"}), json!({"item": "eggs"})]
    );
}

// --- list bought ---

#[test]
fn bought_marks_a_plain_entry_and_show_prints_the_mark() {
    let sandbox = milk_and_eggs();

    let output = sandbox.run(&["list", "bought", "milk"]);

    assert_eq!(exit_code(&output), 0);
    assert_eq!(stdout(&output), "Bought: milk\n");
    assert_eq!(stderr(&output), "");
    assert_eq!(sandbox.show(), "milk [bought]\neggs\n");
    assert_eq!(
        sandbox.entries(),
        vec![
            json!({"item": "milk", "bought": true}),
            json!({"item": "eggs"}),
        ]
    );
}

#[test]
fn bought_marks_a_quantified_entry_and_show_keeps_the_order() {
    let sandbox = pasta_tomato_garlic();

    sandbox.ok(&["list", "bought", "tomato"]);

    assert_eq!(
        sandbox.show(),
        "pasta 200 g\ntomato 400 g [bought]\ngarlic 2 cloves\n"
    );
}

#[test]
fn bought_marks_every_entry_with_the_item_plain_and_quantified() {
    let sandbox = Sandbox::new();
    sandbox.ok(&["list", "add", "tomato"]);
    sandbox.ok(&["list", "add", "tomato", "--qty", "400", "--unit", "g"]);
    sandbox.ok(&["list", "add", "eggs"]);
    sandbox.ok(&["list", "add", "tomato", "--qty", "2", "--unit", "pieces"]);

    let output = sandbox.run(&["list", "bought", "tomato"]);

    assert_eq!(exit_code(&output), 0);
    assert_eq!(stdout(&output), "Bought: tomato\n");
    assert_eq!(
        sandbox.show(),
        "tomato [bought]\ntomato 400 g [bought]\neggs\ntomato 2 pieces [bought]\n"
    );
}

#[test]
fn bought_matching_is_case_sensitive_and_leaves_the_file_unchanged() {
    let sandbox = milk_and_eggs();
    let before = sandbox.read_list();

    let output = sandbox.run(&["list", "bought", "MILK"]);

    assert_eq!(exit_code(&output), 1);
    assert_eq!(stderr(&output), "No entry for: MILK\n");
    assert_eq!(stdout(&output), "");
    assert_eq!(sandbox.read_list(), before);
}

#[test]
fn bought_does_not_match_part_of_an_item() {
    let sandbox = Sandbox::new();
    sandbox.ok(&["list", "add", "olive oil"]);
    let before = sandbox.read_list();

    let output = sandbox.run(&["list", "bought", "olive"]);

    assert_eq!(exit_code(&output), 1);
    assert_eq!(stderr(&output), "No entry for: olive\n");
    assert_eq!(sandbox.read_list(), before);
}

#[test]
fn bought_without_a_list_file_fails_and_creates_no_file() {
    let sandbox = Sandbox::new();

    let output = sandbox.run(&["list", "bought", "milk"]);

    assert_eq!(exit_code(&output), 1);
    assert_eq!(stderr(&output), "No entry for: milk\n");
    assert_eq!(stdout(&output), "");
    assert!(sandbox.files().is_empty(), "files: {:?}", sandbox.files());
}

#[test]
fn bought_on_an_unmatched_item_leaves_the_file_unchanged() {
    let sandbox = milk_and_eggs();
    let before = sandbox.read_list();

    let output = sandbox.run(&["list", "bought", "bread"]);

    assert_eq!(exit_code(&output), 1);
    assert_eq!(stderr(&output), "No entry for: bread\n");
    assert_eq!(stdout(&output), "");
    assert_eq!(sandbox.read_list(), before);
    assert_eq!(sandbox.files(), vec![LIST_FILE.to_string()]);
}

#[test]
fn bought_is_idempotent_and_leaves_no_temporary_file() {
    let sandbox = milk_and_eggs();
    sandbox.ok(&["list", "bought", "milk"]);
    let after_first = sandbox.read_list();

    let output = sandbox.run(&["list", "bought", "milk"]);

    assert_eq!(exit_code(&output), 0);
    assert_eq!(stdout(&output), "Bought: milk\n");
    assert_eq!(sandbox.read_list(), after_first);
    assert_eq!(sandbox.files(), vec![LIST_FILE.to_string()]);
}

#[test]
fn bought_with_an_empty_item_is_a_usage_error() {
    let sandbox = Sandbox::new();

    let output = sandbox.run(&["list", "bought", ""]);

    assert_eq!(exit_code(&output), 2);
    assert!(!stderr(&output).is_empty(), "stderr shows a usage error");
    assert_eq!(stdout(&output), "");
    assert!(sandbox.files().is_empty());
}

#[test]
fn bought_with_a_malformed_list_fails_and_leaves_the_file_unchanged() {
    let sandbox = Sandbox::new();
    let malformed = b"{\"entries\": [{\"item\": \"pasta\"";
    sandbox.write_list(malformed);

    let output = sandbox.run(&["list", "bought", "pasta"]);

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

// --- list show ---

#[test]
fn show_is_unchanged_for_a_list_with_no_bought_entries() {
    let sandbox = pasta_tomato_garlic();
    sandbox.ok(&["list", "add", "milk"]);

    assert_eq!(
        sandbox.show(),
        "pasta 200 g\ntomato 400 g\ngarlic 2 cloves\nmilk\n"
    );
    assert!(
        sandbox
            .entries()
            .iter()
            .all(|entry| entry.get("bought").is_none())
    );
}

#[test]
fn show_of_an_empty_list_is_unchanged() {
    let sandbox = Sandbox::new();

    assert_eq!(sandbox.show(), "The shopping list is empty.\n");
}

// --- adding to a bought entry ---

#[test]
fn add_summing_into_a_bought_entry_unmarks_it() {
    let sandbox = pasta_tomato_garlic();
    sandbox.ok(&["list", "bought", "tomato"]);

    let output = sandbox.run(&["list", "add", "tomato", "--qty", "100", "--unit", "g"]);

    assert_eq!(exit_code(&output), 0);
    assert_eq!(
        sandbox.show(),
        "pasta 200 g\ntomato 500 g\ngarlic 2 cloves\n"
    );
    assert_eq!(
        sandbox.entries()[1],
        json!({"item": "tomato", "quantity": 500, "unit": "g"})
    );
}

#[test]
fn add_in_another_unit_leaves_the_bought_entry_marked() {
    let sandbox = pasta_tomato_garlic();
    sandbox.ok(&["list", "bought", "tomato"]);

    sandbox.ok(&["list", "add", "tomato", "--qty", "2", "--unit", "pieces"]);

    assert_eq!(
        sandbox.show(),
        "pasta 200 g\ntomato 400 g [bought]\ngarlic 2 cloves\ntomato 2 pieces\n"
    );
}

#[test]
fn add_recipe_summing_into_a_bought_ingredient_unmarks_it() {
    let sandbox = Sandbox::new();
    sandbox.ok(&["list", "add-recipe", "Tomato Pasta"]);
    sandbox.ok(&["list", "bought", "tomato"]);
    sandbox.ok(&["list", "bought", "garlic"]);

    let output = sandbox.run(&["list", "add-recipe", "Caprese Salad"]);

    assert_eq!(exit_code(&output), 0);
    assert_eq!(
        sandbox.show(),
        "pasta 200 g\ntomato 700 g\ngarlic 2 cloves [bought]\nolive oil 3 tbsp\n\
         mozzarella 250 g\nbasil 10 g\n"
    );
}

#[test]
fn adding_a_bought_plain_item_again_appends_a_new_entry() {
    let sandbox = Sandbox::new();
    sandbox.ok(&["list", "add", "tomato"]);
    sandbox.ok(&["list", "bought", "tomato"]);

    let output = sandbox.run(&["list", "add", "tomato"]);

    assert_eq!(exit_code(&output), 0);
    assert_eq!(stdout(&output), "Added: tomato\n");
    assert_eq!(sandbox.show(), "tomato [bought]\ntomato\n");
    assert_eq!(
        sandbox.entries(),
        vec![
            json!({"item": "tomato", "bought": true}),
            json!({"item": "tomato"}),
        ]
    );
}

// --- count, last and clear ---

#[test]
fn count_includes_bought_entries() {
    let sandbox = milk_and_eggs();
    sandbox.ok(&["list", "bought", "milk"]);

    let output = sandbox.run(&["list", "count"]);

    assert_eq!(exit_code(&output), 0);
    assert_eq!(stdout(&output), "2 entries\n");
}

#[test]
fn last_prints_the_last_entry_even_when_bought() {
    let sandbox = milk_and_eggs();
    sandbox.ok(&["list", "bought", "eggs"]);

    let output = sandbox.run(&["list", "last"]);

    assert_eq!(exit_code(&output), 0);
    assert_eq!(stdout(&output), "eggs\n");
}

#[test]
fn clear_removes_bought_entries_too() {
    let sandbox = milk_and_eggs();
    sandbox.ok(&["list", "bought", "milk"]);

    let output = sandbox.run(&["list", "clear"]);

    assert_eq!(exit_code(&output), 0);
    assert_eq!(stdout(&output), "Shopping list cleared.\n");
    assert!(sandbox.entries().is_empty());
    assert_eq!(sandbox.show(), "The shopping list is empty.\n");
}

#[test]
fn list_without_a_subcommand_is_still_a_usage_error() {
    let sandbox = Sandbox::new();

    let output = sandbox.run(&["list"]);

    assert_eq!(exit_code(&output), 2);
    assert!(sandbox.files().is_empty());
}
