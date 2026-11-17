//! Integration tests for `ricette list add-recipe <name>`.
//!
//! Each test runs the built binary in a fresh temporary working directory and
//! checks stdout, stderr, the exit code and the `ricette-list.json` file.

use std::fs;
use std::path::Path;
use std::process::{Command, Output};

use serde_json::{Value, json};
use tempfile::TempDir;

const LIST_FILE: &str = "ricette-list.json";

/// One ingredient of a recipe: `(item, quantity, unit)`.
type Ingredient = (&'static str, u64, &'static str);

/// One recipe of the data contract: its name and its ingredients.
type RecipeData = (&'static str, &'static [Ingredient]);

/// The recipe data contract: every recipe with its ingredients, in recipe order.
const RECIPES: &[RecipeData] = &[
    (
        "Tomato Pasta",
        &[
            ("pasta", 200, "g"),
            ("tomato", 400, "g"),
            ("garlic", 2, "cloves"),
            ("olive oil", 2, "tbsp"),
        ],
    ),
    (
        "Mushroom Risotto",
        &[
            ("rice", 300, "g"),
            ("mushroom", 250, "g"),
            ("onion", 1, "piece"),
            ("butter", 50, "g"),
            ("parmesan", 80, "g"),
        ],
    ),
    (
        "Caprese Salad",
        &[
            ("tomato", 300, "g"),
            ("mozzarella", 250, "g"),
            ("basil", 10, "g"),
            ("olive oil", 1, "tbsp"),
        ],
    ),
    (
        "Vegetable Soup",
        &[
            ("carrot", 200, "g"),
            ("potato", 300, "g"),
            ("onion", 1, "piece"),
            ("celery", 150, "g"),
            ("tomato", 400, "g"),
        ],
    ),
    (
        "Cheese Omelette",
        &[
            ("egg", 3, "pieces"),
            ("cheese", 50, "g"),
            ("butter", 10, "g"),
        ],
    ),
];

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

fn entry(item: &str, quantity: u64, unit: &str) -> Value {
    json!({ "item": item, "quantity": quantity, "unit": unit })
}

fn plain(item: &str) -> Value {
    json!({ "item": item })
}

/// Parses `ricette-list.json` in `dir` and returns its entries in order.
fn read_entries(dir: &Path) -> Vec<Value> {
    let text = fs::read_to_string(dir.join(LIST_FILE)).expect("list file exists");
    let value: Value = serde_json::from_str(&text).expect("list file is valid JSON");
    value["entries"]
        .as_array()
        .expect("`entries` is an array")
        .clone()
}

/// The `Added: ...` lines a recipe prints, built from the data contract.
fn added_lines(ingredients: &[Ingredient]) -> String {
    ingredients
        .iter()
        .map(|(item, quantity, unit)| format!("Added: {item} {quantity} {unit}\n"))
        .collect()
}

fn recipe(name: &str) -> &'static [Ingredient] {
    RECIPES
        .iter()
        .find(|(recipe_name, _)| *recipe_name == name)
        .map(|(_, ingredients)| *ingredients)
        .expect("recipe is in the data contract")
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
fn add_recipe_tomato_pasta_prints_four_lines_and_writes_four_entries() {
    let sandbox = Sandbox::new();

    let output = sandbox.run(&["list", "add-recipe", "Tomato Pasta"]);

    assert_eq!(exit_code(&output), 0);
    assert_eq!(
        stdout(&output),
        "Added: pasta 200 g\nAdded: tomato 400 g\nAdded: garlic 2 cloves\nAdded: olive oil 2 tbsp\n"
    );
    assert_eq!(
        read_entries(sandbox.path()),
        vec![
            entry("pasta", 200, "g"),
            entry("tomato", 400, "g"),
            entry("garlic", 2, "cloves"),
            entry("olive oil", 2, "tbsp"),
        ]
    );
}

#[test]
fn caprese_after_tomato_pasta_sums_shared_ingredients_and_prints_recipe_amounts() {
    let sandbox = Sandbox::new();
    sandbox.run(&["list", "add-recipe", "Tomato Pasta"]);

    let output = sandbox.run(&["list", "add-recipe", "Caprese Salad"]);

    assert_eq!(exit_code(&output), 0);
    assert_eq!(
        stdout(&output),
        "Added: tomato 300 g\nAdded: mozzarella 250 g\nAdded: basil 10 g\nAdded: olive oil 1 tbsp\n"
    );
    assert_eq!(
        read_entries(sandbox.path()),
        vec![
            entry("pasta", 200, "g"),
            entry("tomato", 700, "g"),
            entry("garlic", 2, "cloves"),
            entry("olive oil", 3, "tbsp"),
            entry("mozzarella", 250, "g"),
            entry("basil", 10, "g"),
        ]
    );

    let shown = sandbox.run(&["list", "show"]);
    assert_eq!(exit_code(&shown), 0);
    assert_eq!(
        stdout(&shown),
        "pasta 200 g\ntomato 700 g\ngarlic 2 cloves\nolive oil 3 tbsp\nmozzarella 250 g\nbasil 10 g\n"
    );
}

#[test]
fn each_of_the_five_recipes_adds_exactly_its_ingredients_and_quantities() {
    for (name, ingredients) in RECIPES {
        let sandbox = Sandbox::new();

        let output = sandbox.run(&["list", "add-recipe", *name]);

        assert_eq!(exit_code(&output), 0, "exit code for {name}");
        assert_eq!(
            stdout(&output),
            added_lines(ingredients),
            "stdout for {name}"
        );
        let expected: Vec<Value> = ingredients
            .iter()
            .map(|(item, quantity, unit)| entry(item, *quantity, unit))
            .collect();
        assert_eq!(read_entries(sandbox.path()), expected, "entries for {name}");
    }
}

#[test]
fn recipe_name_is_matched_ascii_case_insensitively_over_the_whole_name() {
    let sandbox = Sandbox::new();

    let output = sandbox.run(&["list", "add-recipe", "tomato pasta"]);

    assert_eq!(exit_code(&output), 0);
    assert_eq!(stdout(&output), added_lines(recipe("Tomato Pasta")));
    assert_eq!(read_entries(sandbox.path()).len(), 4);
}

#[test]
fn adding_the_same_recipe_twice_doubles_quantities_and_prints_recipe_amounts() {
    let sandbox = Sandbox::new();
    let first = sandbox.run(&["list", "add-recipe", "Tomato Pasta"]);
    assert_eq!(exit_code(&first), 0);

    let second = sandbox.run(&["list", "add-recipe", "Tomato Pasta"]);

    assert_eq!(exit_code(&second), 0);
    assert_eq!(stdout(&second), added_lines(recipe("Tomato Pasta")));
    assert_eq!(
        read_entries(sandbox.path()),
        vec![
            entry("pasta", 400, "g"),
            entry("tomato", 800, "g"),
            entry("garlic", 4, "cloves"),
            entry("olive oil", 4, "tbsp"),
        ]
    );
}

#[test]
fn plain_entry_is_not_merged_with_a_quantified_recipe_ingredient() {
    let sandbox = Sandbox::new();
    let added = sandbox.run(&["list", "add", "tomato"]);
    assert_eq!(exit_code(&added), 0);

    let output = sandbox.run(&["list", "add-recipe", "Tomato Pasta"]);

    assert_eq!(exit_code(&output), 0);
    assert_eq!(
        read_entries(sandbox.path()),
        vec![
            plain("tomato"),
            entry("pasta", 200, "g"),
            entry("tomato", 400, "g"),
            entry("garlic", 2, "cloves"),
            entry("olive oil", 2, "tbsp"),
        ]
    );
}

#[test]
fn unknown_recipe_exits_1_with_message_on_stderr_and_creates_no_file() {
    let sandbox = Sandbox::new();

    let output = sandbox.run(&["list", "add-recipe", "Pizza"]);

    assert_eq!(exit_code(&output), 1);
    assert_eq!(stderr(&output), "Unknown recipe: Pizza\n");
    assert_eq!(stdout(&output), "");
    assert!(
        !sandbox.path().join(LIST_FILE).exists(),
        "no list file should be created"
    );
}

#[test]
fn unknown_recipe_leaves_an_existing_list_unchanged() {
    let sandbox = Sandbox::new();
    sandbox.run(&["list", "add", "milk"]);
    let before = fs::read(sandbox.path().join(LIST_FILE)).expect("list file exists");

    let output = sandbox.run(&["list", "add-recipe", "Pizza"]);

    assert_eq!(exit_code(&output), 1);
    assert_eq!(stderr(&output), "Unknown recipe: Pizza\n");
    assert_eq!(
        fs::read(sandbox.path().join(LIST_FILE)).expect("list file exists"),
        before
    );
}

#[test]
fn add_recipe_with_a_malformed_list_fails_and_leaves_the_file_unchanged() {
    let sandbox = Sandbox::new();
    let contents = b"{ this is not json";
    sandbox.write_list(contents);

    let output = sandbox.run(&["list", "add-recipe", "Tomato Pasta"]);

    assert_eq!(exit_code(&output), 1);
    assert!(!stderr(&output).is_empty(), "stderr should report an error");
    assert_eq!(stdout(&output), "", "nothing should be printed as added");
    assert_eq!(
        fs::read(sandbox.path().join(LIST_FILE)).expect("list file exists"),
        contents
    );
}

#[test]
fn add_recipe_whose_write_fails_leaves_the_existing_list_unchanged() {
    let sandbox = Sandbox::new();
    sandbox.run(&["list", "add", "milk"]);
    let before = fs::read(sandbox.path().join(LIST_FILE)).expect("list file exists");
    // A directory where the temporary file would be written makes the write fail.
    fs::create_dir(sandbox.path().join(format!("{LIST_FILE}.tmp"))).expect("create blocker");

    let output = sandbox.run(&["list", "add-recipe", "Tomato Pasta"]);

    assert_eq!(exit_code(&output), 1);
    assert!(!stderr(&output).is_empty(), "stderr should report an error");
    assert_eq!(
        fs::read(sandbox.path().join(LIST_FILE)).expect("list file exists"),
        before
    );
}

#[test]
fn recipes_search_tomato_keeps_its_names_only_output() {
    let sandbox = Sandbox::new();

    let output = sandbox.run(&["recipes", "search", "tomato"]);

    assert_eq!(exit_code(&output), 0);
    assert_eq!(
        stdout(&output),
        "Tomato Pasta\nCaprese Salad\nVegetable Soup\n"
    );
}

#[test]
fn recipes_list_keeps_its_names_only_output() {
    let sandbox = Sandbox::new();

    let output = sandbox.run(&["recipes", "list"]);

    assert_eq!(exit_code(&output), 0);
    assert_eq!(
        stdout(&output),
        "Tomato Pasta\nMushroom Risotto\nCaprese Salad\nVegetable Soup\nCheese Omelette\n"
    );
}
