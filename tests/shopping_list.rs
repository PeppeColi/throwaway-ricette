//! Integration tests for `ricette list add|show|clear|count`.
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

/// Parses `ricette-list.json` in `dir` and returns its entries in order, as JSON.
fn read_entries(dir: &Path) -> Vec<Value> {
    let text = fs::read_to_string(dir.join(LIST_FILE)).expect("list file exists");
    let value: Value = serde_json::from_str(&text).expect("list file is valid JSON");
    value["entries"]
        .as_array()
        .expect("`entries` is an array")
        .clone()
}

/// Parses `ricette-list.json` in `dir` and returns its items in order.
fn read_items(dir: &Path) -> Vec<String> {
    read_entries(dir)
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

fn entry(item: &str, quantity: u64, unit: &str) -> Value {
    serde_json::json!({"item": item, "quantity": quantity, "unit": unit})
}

fn plain(item: &str) -> Value {
    serde_json::json!({"item": item})
}

#[test]
fn quantified_add_prints_the_amount_and_writes_one_entry() {
    let sandbox = Sandbox::new();

    let output = sandbox.run(&["list", "add", "flour", "--qty", "2", "--unit", "kg"]);

    assert_eq!(exit_code(&output), 0);
    assert_eq!(stdout(&output), "Added: flour 2 kg\n");
    assert_eq!(read_entries(sandbox.path()), vec![entry("flour", 2, "kg")]);
}

#[test]
fn adding_the_same_item_and_unit_sums_into_the_first_entry() {
    let sandbox = Sandbox::new();
    sandbox.run(&["list", "add", "flour", "--qty", "2", "--unit", "kg"]);

    let output = sandbox.run(&["list", "add", "flour", "--qty", "1", "--unit", "kg"]);

    assert_eq!(exit_code(&output), 0);
    assert_eq!(stdout(&output), "Added: flour 1 kg\n");
    assert_eq!(read_entries(sandbox.path()), vec![entry("flour", 3, "kg")]);
}

#[test]
fn same_item_with_a_different_unit_appends_a_new_entry_and_show_lists_both() {
    let sandbox = Sandbox::new();
    sandbox.run(&["list", "add", "flour", "--qty", "2", "--unit", "kg"]);
    sandbox.run(&["list", "add", "flour", "--qty", "1", "--unit", "kg"]);

    let output = sandbox.run(&["list", "add", "flour", "--qty", "500", "--unit", "g"]);
    assert_eq!(exit_code(&output), 0);
    assert_eq!(stdout(&output), "Added: flour 500 g\n");
    assert_eq!(
        read_entries(sandbox.path()),
        vec![entry("flour", 3, "kg"), entry("flour", 500, "g")]
    );

    let shown = sandbox.run(&["list", "show"]);
    assert_eq!(exit_code(&shown), 0);
    assert_eq!(stdout(&shown), "flour 3 kg\nflour 500 g\n");
}

#[test]
fn plain_add_still_prints_only_the_item_and_never_merges() {
    let sandbox = Sandbox::new();

    let first = sandbox.run(&["list", "add", "milk"]);
    let second = sandbox.run(&["list", "add", "milk"]);

    assert_eq!(exit_code(&first), 0);
    assert_eq!(stdout(&first), "Added: milk\n");
    assert_eq!(exit_code(&second), 0);
    assert_eq!(stdout(&second), "Added: milk\n");
    assert_eq!(
        read_entries(sandbox.path()),
        vec![plain("milk"), plain("milk")]
    );
}

#[test]
fn plain_add_does_not_merge_with_a_quantified_entry_of_the_same_item() {
    let sandbox = Sandbox::new();
    sandbox.run(&["list", "add", "milk", "--qty", "1", "--unit", "l"]);

    sandbox.run(&["list", "add", "milk"]);

    assert_eq!(
        read_entries(sandbox.path()),
        vec![entry("milk", 1, "l"), plain("milk")]
    );
}

#[test]
fn show_prints_quantified_and_plain_entries_in_insertion_order() {
    let sandbox = Sandbox::new();
    sandbox.run(&["list", "add", "flour", "--qty", "2", "--unit", "kg"]);
    sandbox.run(&["list", "add", "milk"]);
    sandbox.run(&["list", "add", "eggs", "--qty", "6", "--unit", "pieces"]);

    let output = sandbox.run(&["list", "show"]);

    assert_eq!(exit_code(&output), 0);
    assert_eq!(stdout(&output), "flour 2 kg\nmilk\neggs 6 pieces\n");
}

#[test]
fn summing_keeps_the_position_of_the_first_matching_entry() {
    let sandbox = Sandbox::new();
    sandbox.run(&["list", "add", "flour", "--qty", "2", "--unit", "kg"]);
    sandbox.run(&["list", "add", "milk"]);

    sandbox.run(&["list", "add", "flour", "--qty", "1", "--unit", "kg"]);

    let shown = sandbox.run(&["list", "show"]);
    assert_eq!(stdout(&shown), "flour 3 kg\nmilk\n");
    assert_eq!(
        read_entries(sandbox.path()),
        vec![entry("flour", 3, "kg"), plain("milk")]
    );
}

#[test]
fn item_case_difference_never_merges_quantified_entries() {
    let sandbox = Sandbox::new();
    sandbox.run(&["list", "add", "milk", "--qty", "1", "--unit", "l"]);

    let output = sandbox.run(&["list", "add", "Milk", "--qty", "1", "--unit", "l"]);

    assert_eq!(exit_code(&output), 0);
    assert_eq!(
        read_entries(sandbox.path()),
        vec![entry("milk", 1, "l"), entry("Milk", 1, "l")]
    );
}

#[test]
fn unit_case_difference_never_merges() {
    let sandbox = Sandbox::new();
    sandbox.run(&["list", "add", "flour", "--qty", "1", "--unit", "kg"]);

    let output = sandbox.run(&["list", "add", "flour", "--qty", "1", "--unit", "KG"]);

    assert_eq!(exit_code(&output), 0);
    assert_eq!(
        read_entries(sandbox.path()),
        vec![entry("flour", 1, "kg"), entry("flour", 1, "KG")]
    );
}

#[test]
fn legacy_plain_entries_are_read_and_a_quantified_add_keeps_the_file_valid() {
    let sandbox = Sandbox::new();
    sandbox.write_list(br#"{"entries":[{"item":"milk"}]}"#);

    let shown = sandbox.run(&["list", "show"]);
    assert_eq!(exit_code(&shown), 0);
    assert_eq!(stdout(&shown), "milk\n");

    let added = sandbox.run(&["list", "add", "milk", "--qty", "1", "--unit", "l"]);
    assert_eq!(exit_code(&added), 0);
    assert_eq!(stdout(&added), "Added: milk 1 l\n");
    assert_eq!(
        read_entries(sandbox.path()),
        vec![plain("milk"), entry("milk", 1, "l")]
    );
    let shown_again = sandbox.run(&["list", "show"]);
    assert_eq!(stdout(&shown_again), "milk\nmilk 1 l\n");
}

#[test]
fn quantity_without_unit_is_a_usage_error_and_leaves_the_file_untouched() {
    let sandbox = Sandbox::new();
    sandbox.run(&["list", "add", "flour", "--qty", "2", "--unit", "kg"]);
    let before = fs::read(sandbox.path().join(LIST_FILE)).unwrap();

    let output = sandbox.run(&["list", "add", "flour", "--qty", "2"]);

    assert_eq!(exit_code(&output), 2);
    assert!(
        !stderr(&output).is_empty(),
        "stderr should show a usage error"
    );
    assert_eq!(fs::read(sandbox.path().join(LIST_FILE)).unwrap(), before);
}

#[test]
fn unit_without_quantity_is_a_usage_error_and_creates_no_file() {
    let sandbox = Sandbox::new();

    let output = sandbox.run(&["list", "add", "flour", "--unit", "kg"]);

    assert_eq!(exit_code(&output), 2);
    assert!(
        !stderr(&output).is_empty(),
        "stderr should show a usage error"
    );
    assert!(!sandbox.path().join(LIST_FILE).exists());
}

#[test]
fn invalid_quantity_or_unit_is_a_usage_error_and_leaves_the_file_untouched() {
    let invalid: [&[&str]; 4] = [
        &["list", "add", "flour", "--qty", "0", "--unit", "kg"],
        &["list", "add", "flour", "--qty", "-1", "--unit", "kg"],
        &["list", "add", "flour", "--qty", "2", "--unit", ""],
        &["list", "add", "flour", "--qty", "abc", "--unit", "kg"],
    ];
    for args in invalid {
        let sandbox = Sandbox::new();
        sandbox.run(&["list", "add", "milk"]);
        let before = fs::read(sandbox.path().join(LIST_FILE)).unwrap();

        let output = sandbox.run(args);

        assert_eq!(exit_code(&output), 2, "arguments {args:?}");
        assert!(
            !stderr(&output).is_empty(),
            "stderr should show a usage error for {args:?}"
        );
        assert_eq!(
            fs::read(sandbox.path().join(LIST_FILE)).unwrap(),
            before,
            "the file must be unchanged for {args:?}"
        );
    }
}

#[test]
fn clear_removes_plain_and_quantified_entries() {
    let sandbox = Sandbox::new();
    sandbox.run(&["list", "add", "flour", "--qty", "2", "--unit", "kg"]);
    sandbox.run(&["list", "add", "milk"]);

    let cleared = sandbox.run(&["list", "clear"]);
    assert_eq!(exit_code(&cleared), 0);
    assert_eq!(stdout(&cleared), "Shopping list cleared.\n");
    assert_eq!(read_entries(sandbox.path()), Vec::<Value>::new());

    let shown = sandbox.run(&["list", "show"]);
    assert_eq!(exit_code(&shown), 0);
    assert_eq!(stdout(&shown), "The shopping list is empty.\n");
}

#[test]
fn count_without_a_list_file_prints_zero_entries_and_creates_no_file() {
    let sandbox = Sandbox::new();

    let output = sandbox.run(&["list", "count"]);

    assert_eq!(exit_code(&output), 0);
    assert_eq!(stdout(&output), "0 entries\n");
    assert!(!sandbox.path().join(LIST_FILE).exists());
}

#[test]
fn count_after_one_add_prints_one_entry_in_the_singular() {
    let sandbox = Sandbox::new();
    sandbox.run(&["list", "add", "milk"]);

    let output = sandbox.run(&["list", "count"]);

    assert_eq!(exit_code(&output), 0);
    assert_eq!(stdout(&output), "1 entry\n");
}

#[test]
fn count_counts_duplicate_items_as_separate_entries() {
    let sandbox = Sandbox::new();
    sandbox.run(&["list", "add", "milk"]);
    sandbox.run(&["list", "add", "milk"]);
    sandbox.run(&["list", "add", "eggs"]);

    let output = sandbox.run(&["list", "count"]);

    assert_eq!(exit_code(&output), 0);
    assert_eq!(stdout(&output), "3 entries\n");
}

#[test]
fn count_with_a_malformed_list_fails_and_leaves_the_file_unchanged() {
    let sandbox = Sandbox::new();
    let malformed = b"{\"entries\": [{\"item\": \"milk\"";
    sandbox.write_list(malformed);

    let output = sandbox.run(&["list", "count"]);

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
