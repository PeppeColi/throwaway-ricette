//! Contract tests for `ricette recipes show --servings` and `ricette list add-recipe --servings`.
//!
//! Both commands scale the same catalog. Each test runs the built binary in fresh
//! temporary directories and checks that, for every recipe at 1, 2 and 3 servings,
//! the lines `recipes show` prints are the lines `list add-recipe` prints and writes,
//! in the same order, together with the summing, default, error and regression rules.

use std::fs;
use std::process::{Command, Output};

use serde_json::{Value, json};
use tempfile::TempDir;

const LIST_FILE: &str = "ricette-list.json";

/// One ingredient of a recipe: `(item, quantity, unit)`.
type Ingredient = (&'static str, u64, &'static str);

/// One recipe of the data contract: its name and its ingredients.
type RecipeData = (&'static str, &'static [Ingredient]);

/// The recipe data contract: every recipe with its ingredients for one serving, in
/// catalog order.
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

/// The servings counts that every recipe is checked at.
const SERVINGS: [u64; 3] = [1, 2, 3];

/// A fresh working directory, plus a fresh home directory that `HOME` points at, so a
/// test can see that nothing is written outside the working directory.
struct Sandbox {
    dir: TempDir,
    home: TempDir,
}

impl Sandbox {
    fn new() -> Self {
        Self {
            dir: TempDir::new().expect("create working directory"),
            home: TempDir::new().expect("create home directory"),
        }
    }

    /// Runs `ricette` with `args` in the working directory.
    fn run(&self, args: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_ricette"))
            .args(args)
            .current_dir(self.dir.path())
            .env("HOME", self.home.path())
            .output()
            .expect("failed to run the ricette binary")
    }

    fn write_list(&self, contents: &[u8]) {
        fs::write(self.dir.path().join(LIST_FILE), contents).expect("write the list file");
    }

    /// The bytes of the list file, or `None` when it does not exist.
    fn list_bytes(&self) -> Option<Vec<u8>> {
        fs::read(self.dir.path().join(LIST_FILE)).ok()
    }

    /// The entries of the list file, in order. The file must exist.
    fn entries(&self) -> Vec<Value> {
        let text = fs::read_to_string(self.dir.path().join(LIST_FILE)).expect("list file exists");
        let value: Value = serde_json::from_str(&text).expect("list file is valid JSON");
        value["entries"]
            .as_array()
            .expect("`entries` is an array")
            .clone()
    }

    /// The names of the files in the working directory, sorted.
    fn file_names(&self) -> Vec<String> {
        let mut names: Vec<String> = fs::read_dir(self.dir.path())
            .expect("read the working directory")
            .map(|entry| {
                entry
                    .expect("directory entry")
                    .file_name()
                    .to_string_lossy()
                    .into_owned()
            })
            .collect();
        names.sort();
        names
    }
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

fn recipe(name: &str) -> &'static [Ingredient] {
    RECIPES
        .iter()
        .find(|(recipe_name, _)| *recipe_name == name)
        .map(|(_, ingredients)| *ingredients)
        .unwrap_or_else(|| panic!("{name} is in the data contract"))
}

/// The ingredients in recipe order, each quantity multiplied by `servings`.
fn scaled(ingredients: &[Ingredient], servings: u64) -> Vec<Ingredient> {
    ingredients
        .iter()
        .map(|&(item, quantity, unit)| (item, quantity * servings, unit))
        .collect()
}

/// The lines `recipes show` prints: `<item> <quantity> <unit>`, one per ingredient.
fn show_lines(ingredients: &[Ingredient]) -> String {
    ingredients
        .iter()
        .map(|(item, quantity, unit)| format!("{item} {quantity} {unit}\n"))
        .collect()
}

/// The lines `list add-recipe` prints: `Added: <item> <quantity> <unit>`, one per ingredient.
fn added_lines(ingredients: &[Ingredient]) -> String {
    ingredients
        .iter()
        .map(|(item, quantity, unit)| format!("Added: {item} {quantity} {unit}\n"))
        .collect()
}

/// The `Added: ` lines with the prefix removed, so they can be compared with `show_lines`.
fn without_added_prefix(added: &str) -> String {
    added
        .lines()
        .map(|line| format!("{}\n", line.strip_prefix("Added: ").unwrap_or(line)))
        .collect()
}

fn entry(item: &str, quantity: u64, unit: &str) -> Value {
    json!({ "item": item, "quantity": quantity, "unit": unit })
}

fn plain(item: &str) -> Value {
    json!({ "item": item })
}

/// The entries `list add-recipe` writes on an empty list for `ingredients`.
fn entries_of(ingredients: &[Ingredient]) -> Vec<Value> {
    ingredients
        .iter()
        .map(|&(item, quantity, unit)| entry(item, quantity, unit))
        .collect()
}

/// The line `list show` prints for an entry.
fn display(entry: &Value) -> String {
    let item = entry["item"].as_str().expect("item is a string");
    match (entry.get("quantity"), entry.get("unit")) {
        (Some(quantity), Some(unit)) => {
            format!(
                "{item} {quantity} {}",
                unit.as_str().expect("unit is a string")
            )
        }
        _ => item.to_string(),
    }
}

/// The output of `list show` for `entries`: one line per entry.
fn list_show_output(entries: &[Value]) -> String {
    entries
        .iter()
        .map(|entry| format!("{}\n", display(entry)))
        .collect()
}

/// The entries the list must hold after `add-recipe` adds `ingredients` to `existing`:
/// the first entry with the same item and unit takes the sum and keeps its place;
/// otherwise a new entry is appended. Plain entries never match.
fn merged(existing: &[Value], ingredients: &[Ingredient]) -> Vec<Value> {
    let mut entries = existing.to_vec();
    for &(item, quantity, unit) in ingredients {
        let position = entries
            .iter()
            .position(|entry| entry["item"] == item && entry["unit"] == unit);
        match position {
            Some(index) => {
                let total = entries[index]["quantity"]
                    .as_u64()
                    .expect("a quantified entry has a quantity")
                    + quantity;
                entries[index]["quantity"] = json!(total);
            }
            None => entries.push(entry(item, quantity, unit)),
        }
    }
    entries
}

/// Fails unless `recipes show` and the `list add-recipe` run that followed it agree:
/// the printed `Added:` lines and the written entries carry the same item, quantity
/// and unit as the `show` lines, in the same order.
fn assert_agree(shown: &str, added: &str, entries: &[Value], context: &str) {
    assert_eq!(
        without_added_prefix(added),
        shown,
        "{context}: add-recipe prints what show prints"
    );
    assert_eq!(
        list_show_output(entries),
        shown,
        "{context}: the written entries are what show prints"
    );
}

#[test]
fn show_prints_the_scaled_lines_of_every_recipe_at_1_2_and_3_servings() {
    for &(name, ingredients) in RECIPES {
        for servings in SERVINGS {
            let sandbox = Sandbox::new();
            let count = servings.to_string();
            let output = sandbox.run(&["recipes", "show", name, "--servings", &count]);
            let context = format!("recipes show {name:?} --servings {servings}");

            assert_eq!(exit_code(&output), 0, "{context}: exit code");
            assert_eq!(
                stdout(&output),
                show_lines(&scaled(ingredients, servings)),
                "{context}"
            );
            assert_eq!(stderr(&output), "", "{context}: stderr");
            assert!(
                sandbox.file_names().is_empty(),
                "{context}: creates no files"
            );
        }
    }
}

#[test]
fn add_recipe_prints_and_writes_the_same_scaled_lines_as_show_for_every_recipe() {
    for &(name, ingredients) in RECIPES {
        for servings in SERVINGS {
            let count = servings.to_string();
            let context = format!("{name:?} at {servings} servings");
            let scaled = scaled(ingredients, servings);

            let shown_run = Sandbox::new();
            let shown = shown_run.run(&["recipes", "show", name, "--servings", &count]);
            assert_eq!(exit_code(&shown), 0, "{context}: show exit code");

            let added_run = Sandbox::new();
            let added = added_run.run(&["list", "add-recipe", name, "--servings", &count]);
            assert_eq!(exit_code(&added), 0, "{context}: add-recipe exit code");
            assert_eq!(stderr(&added), "", "{context}: add-recipe stderr");
            assert_eq!(
                stdout(&added),
                added_lines(&scaled),
                "{context}: add-recipe output"
            );

            let entries = added_run.entries();
            assert_eq!(entries, entries_of(&scaled), "{context}: written entries");
            assert_agree(&stdout(&shown), &stdout(&added), &entries, &context);
        }
    }
}

#[test]
fn caprese_salad_at_2_servings_after_tomato_pasta_prints_the_documented_totals() {
    let sandbox = Sandbox::new();
    let first = sandbox.run(&["list", "add-recipe", "Tomato Pasta"]);
    assert_eq!(exit_code(&first), 0);

    let output = sandbox.run(&["list", "add-recipe", "Caprese Salad", "--servings", "2"]);

    assert_eq!(exit_code(&output), 0);
    assert_eq!(
        stdout(&output),
        "Added: tomato 600 g\n\
         Added: mozzarella 500 g\n\
         Added: basil 20 g\n\
         Added: olive oil 2 tbsp\n"
    );
    let shown = sandbox.run(&["list", "show"]);
    assert_eq!(
        stdout(&shown),
        "pasta 200 g\n\
         tomato 1000 g\n\
         garlic 2 cloves\n\
         olive oil 4 tbsp\n\
         mozzarella 500 g\n\
         basil 20 g\n"
    );
}

#[test]
fn add_recipe_sums_scaled_amounts_into_existing_entries_for_every_base_recipe_and_servings() {
    for &(base, _) in RECIPES {
        for &(name, ingredients) in RECIPES {
            for servings in SERVINGS {
                let sandbox = Sandbox::new();
                let count = servings.to_string();
                let context = format!("{name:?} at {servings} servings after {base:?}");

                let first = sandbox.run(&["list", "add-recipe", base]);
                assert_eq!(exit_code(&first), 0, "{context}: base exit code");
                let before = sandbox.entries();

                let output = sandbox.run(&["list", "add-recipe", name, "--servings", &count]);
                assert_eq!(exit_code(&output), 0, "{context}: exit code");
                let scaled = scaled(ingredients, servings);
                assert_eq!(
                    stdout(&output),
                    added_lines(&scaled),
                    "{context}: printed lines"
                );

                let expected = merged(&before, &scaled);
                assert_eq!(sandbox.entries(), expected, "{context}: written entries");

                let shown = sandbox.run(&["list", "show"]);
                assert_eq!(
                    stdout(&shown),
                    list_show_output(&expected),
                    "{context}: list show totals"
                );
            }
        }
    }
}

#[test]
fn a_plain_entry_is_never_merged_with_a_scaled_quantified_ingredient() {
    let sandbox = Sandbox::new();
    let plain_add = sandbox.run(&["list", "add", "tomato"]);
    assert_eq!(exit_code(&plain_add), 0);

    let output = sandbox.run(&["list", "add-recipe", "Tomato Pasta", "--servings", "2"]);

    assert_eq!(exit_code(&output), 0);
    assert_eq!(
        sandbox.entries(),
        vec![
            plain("tomato"),
            entry("pasta", 400, "g"),
            entry("tomato", 800, "g"),
            entry("garlic", 4, "cloves"),
            entry("olive oil", 4, "tbsp"),
        ]
    );
    let shown = sandbox.run(&["list", "show"]);
    assert_eq!(
        stdout(&shown),
        "tomato\npasta 400 g\ntomato 800 g\ngarlic 4 cloves\nolive oil 4 tbsp\n"
    );
}

#[test]
fn without_servings_both_commands_behave_exactly_like_servings_1() {
    for &(name, _) in RECIPES {
        let context = format!("{name:?}");

        let show_default = Sandbox::new().run(&["recipes", "show", name]);
        let show_one = Sandbox::new().run(&["recipes", "show", name, "--servings", "1"]);
        assert_eq!(
            exit_code(&show_default),
            exit_code(&show_one),
            "{context}: show exit"
        );
        assert_eq!(
            stdout(&show_default),
            stdout(&show_one),
            "{context}: show stdout"
        );
        assert_eq!(
            stderr(&show_default),
            stderr(&show_one),
            "{context}: show stderr"
        );

        let default_run = Sandbox::new();
        let add_default = default_run.run(&["list", "add-recipe", name]);
        let one_run = Sandbox::new();
        let add_one = one_run.run(&["list", "add-recipe", name, "--servings", "1"]);
        assert_eq!(
            exit_code(&add_default),
            exit_code(&add_one),
            "{context}: add exit"
        );
        assert_eq!(
            stdout(&add_default),
            stdout(&add_one),
            "{context}: add stdout"
        );
        assert_eq!(
            stderr(&add_default),
            stderr(&add_one),
            "{context}: add stderr"
        );
        assert_eq!(
            default_run.list_bytes(),
            one_run.list_bytes(),
            "{context}: list file bytes"
        );
    }
}

#[test]
fn unknown_recipe_fails_the_same_way_with_or_without_servings_and_changes_nothing() {
    let commands: [&[&str]; 4] = [
        &["recipes", "show", "Pizza"],
        &["recipes", "show", "Pizza", "--servings", "2"],
        &["list", "add-recipe", "Pizza"],
        &["list", "add-recipe", "Pizza", "--servings", "2"],
    ];
    for args in commands {
        let context = format!("{args:?}");

        let sandbox = Sandbox::new();
        let output = sandbox.run(args);
        assert_eq!(exit_code(&output), 1, "{context}: exit code");
        assert_eq!(
            stderr(&output),
            "Unknown recipe: Pizza\n",
            "{context}: stderr"
        );
        assert_eq!(stdout(&output), "", "{context}: stdout");
        assert!(
            sandbox.file_names().is_empty(),
            "{context}: creates no file"
        );

        let existing = b"{\n  \"entries\": []\n}\n";
        let with_list = Sandbox::new();
        with_list.write_list(existing);
        let output = with_list.run(args);
        assert_eq!(exit_code(&output), 1, "{context}: exit code with a list");
        assert_eq!(stdout(&output), "", "{context}: stdout with a list");
        assert_eq!(
            with_list.list_bytes().as_deref(),
            Some(&existing[..]),
            "{context}: list unchanged"
        );
    }
}

#[test]
fn invalid_servings_are_usage_errors_that_print_nothing_and_change_nothing() {
    let existing = b"{\n  \"entries\": [\n    {\n      \"item\": \"tomato\"\n    }\n  ]\n}\n";
    for value in ["0", "-1", "two"] {
        for args in [
            vec!["recipes", "show", "Tomato Pasta", "--servings", value],
            vec!["list", "add-recipe", "Tomato Pasta", "--servings", value],
        ] {
            let context = format!("{args:?}");

            let sandbox = Sandbox::new();
            let output = sandbox.run(&args);
            assert_eq!(exit_code(&output), 2, "{context}: exit code");
            assert_eq!(stdout(&output), "", "{context}: stdout");
            // Clap's usage errors, for an out-of-range value and a non-number alike,
            // end with this hint.
            assert!(
                stderr(&output).contains("For more information, try '--help'."),
                "{context}: usage error on stderr, got: {}",
                stderr(&output)
            );
            assert!(
                sandbox.file_names().is_empty(),
                "{context}: creates no file"
            );

            let with_list = Sandbox::new();
            with_list.write_list(existing);
            let output = with_list.run(&args);
            assert_eq!(exit_code(&output), 2, "{context}: exit code with a list");
            assert_eq!(stdout(&output), "", "{context}: stdout with a list");
            assert_eq!(
                with_list.list_bytes().as_deref(),
                Some(&existing[..]),
                "{context}: list unchanged"
            );
        }
    }
}

#[test]
fn scaling_overflow_fails_with_the_exact_message_and_changes_nothing() {
    let max = u64::MAX.to_string();
    let existing = b"{\n  \"entries\": []\n}\n";

    let show = Sandbox::new();
    let output = show.run(&["recipes", "show", "Tomato Pasta", "--servings", &max]);
    assert_eq!(exit_code(&output), 1, "show exit code");
    assert_eq!(stdout(&output), "", "show stdout");
    assert_eq!(
        stderr(&output),
        "error: the quantity of pasta in g is too large\n",
        "show stderr"
    );
    assert!(show.file_names().is_empty(), "show creates no file");

    let add = Sandbox::new();
    add.write_list(existing);
    let output = add.run(&["list", "add-recipe", "Tomato Pasta", "--servings", &max]);
    assert_eq!(exit_code(&output), 1, "add-recipe exit code");
    assert_eq!(stdout(&output), "", "add-recipe stdout");
    assert_eq!(
        stderr(&output),
        "error: the quantity of pasta in g is too large\n",
        "add-recipe stderr"
    );
    assert_eq!(
        add.list_bytes().as_deref(),
        Some(&existing[..]),
        "add-recipe list unchanged"
    );

    let missing = Sandbox::new();
    let output = missing.run(&["list", "add-recipe", "Tomato Pasta", "--servings", &max]);
    assert_eq!(exit_code(&output), 1, "add-recipe exit code without a list");
    assert!(
        missing.file_names().is_empty(),
        "add-recipe creates no file"
    );
}

#[test]
fn add_recipe_with_servings_fails_on_a_malformed_list_and_leaves_it_byte_for_byte() {
    let malformed = b"{ this is not json";
    let sandbox = Sandbox::new();
    sandbox.write_list(malformed);

    let output = sandbox.run(&["list", "add-recipe", "Tomato Pasta", "--servings", "2"]);

    assert_eq!(exit_code(&output), 1);
    assert_eq!(stdout(&output), "");
    assert!(
        stderr(&output).contains("the shopping list cannot be read"),
        "stderr reports the unreadable list, got: {}",
        stderr(&output)
    );
    assert_eq!(sandbox.list_bytes().as_deref(), Some(&malformed[..]));
}

#[test]
fn recipes_list_and_search_keep_their_names_only_output() {
    let sandbox = Sandbox::new();
    let list = sandbox.run(&["recipes", "list"]);
    assert_eq!(exit_code(&list), 0);
    assert_eq!(
        stdout(&list),
        "Tomato Pasta\nMushroom Risotto\nCaprese Salad\nVegetable Soup\nCheese Omelette\n"
    );

    let search = sandbox.run(&["recipes", "search", "tomato"]);
    assert_eq!(exit_code(&search), 0);
    assert_eq!(
        stdout(&search),
        "Tomato Pasta\nCaprese Salad\nVegetable Soup\n"
    );
    assert!(
        sandbox.file_names().is_empty(),
        "list and search create no files"
    );
}

#[test]
fn add_recipe_without_servings_keeps_its_previous_output_and_entries() {
    let sandbox = Sandbox::new();
    let output = sandbox.run(&["list", "add-recipe", "Tomato Pasta"]);

    assert_eq!(exit_code(&output), 0);
    assert_eq!(
        stdout(&output),
        "Added: pasta 200 g\nAdded: tomato 400 g\nAdded: garlic 2 cloves\nAdded: olive oil 2 tbsp\n"
    );
    assert_eq!(sandbox.entries(), entries_of(recipe("Tomato Pasta")));
}

#[test]
fn recipes_show_list_and_search_create_no_files_and_ignore_a_malformed_list() {
    let malformed = b"not a list";
    let sandbox = Sandbox::new();
    sandbox.write_list(malformed);

    let show = sandbox.run(&["recipes", "show", "Tomato Pasta"]);
    assert_eq!(
        exit_code(&show),
        0,
        "show succeeds despite a malformed list"
    );
    assert_eq!(stdout(&show), show_lines(recipe("Tomato Pasta")));

    let list = sandbox.run(&["recipes", "list"]);
    assert_eq!(
        exit_code(&list),
        0,
        "list succeeds despite a malformed list"
    );

    let search = sandbox.run(&["recipes", "search", "tomato"]);
    assert_eq!(
        exit_code(&search),
        0,
        "search succeeds despite a malformed list"
    );

    assert_eq!(sandbox.list_bytes().as_deref(), Some(&malformed[..]));
    assert_eq!(sandbox.file_names(), vec![LIST_FILE.to_string()]);
}

#[test]
fn an_empty_working_directory_stays_empty_after_the_read_only_commands() {
    let sandbox = Sandbox::new();
    for args in [
        &["recipes", "show", "Caprese Salad", "--servings", "3"][..],
        &["recipes", "list"][..],
        &["recipes", "search", "onion"][..],
    ] {
        let output = sandbox.run(args);
        assert_eq!(exit_code(&output), 0, "{args:?}");
    }
    assert!(sandbox.file_names().is_empty());
}
