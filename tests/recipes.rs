use std::fs;
use std::path::Path;
use std::process::{Command, Output};

use tempfile::TempDir;

/// Runs the built `ricette` binary with `args` in a fresh, empty temporary directory.
fn run_in_empty_dir(args: &[&str]) -> (Output, TempDir) {
    let dir = TempDir::new().expect("create temporary directory");
    let output = run_in(dir.path(), args);
    (output, dir)
}

fn run_in(dir: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_ricette"))
        .args(args)
        .current_dir(dir)
        .output()
        .expect("run ricette binary")
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

fn entries_in(dir: &Path) -> Vec<String> {
    let mut names: Vec<String> = fs::read_dir(dir)
        .expect("read directory")
        .map(|entry| {
            entry
                .expect("read entry")
                .file_name()
                .to_string_lossy()
                .into_owned()
        })
        .collect();
    names.sort();
    names
}

#[test]
fn list_prints_the_five_recipe_names_in_catalog_order() {
    let (output, _dir) = run_in_empty_dir(&["recipes", "list"]);

    assert_eq!(exit_code(&output), 0);
    assert_eq!(
        stdout(&output),
        "Tomato Pasta\nMushroom Risotto\nCaprese Salad\nVegetable Soup\nCheese Omelette\n"
    );
}

#[test]
fn search_tomato_prints_the_three_recipes_that_use_it_in_catalog_order() {
    let (output, _dir) = run_in_empty_dir(&["recipes", "search", "tomato"]);

    assert_eq!(exit_code(&output), 0);
    assert_eq!(
        stdout(&output),
        "Tomato Pasta\nCaprese Salad\nVegetable Soup\n"
    );
}

#[test]
fn search_is_ascii_case_insensitive() {
    let (output, _dir) = run_in_empty_dir(&["recipes", "search", "TOMATO"]);

    assert_eq!(exit_code(&output), 0);
    assert_eq!(
        stdout(&output),
        "Tomato Pasta\nCaprese Salad\nVegetable Soup\n"
    );
}

#[test]
fn search_potato_prints_only_vegetable_soup() {
    let (output, _dir) = run_in_empty_dir(&["recipes", "search", "potato"]);

    assert_eq!(exit_code(&output), 0);
    assert_eq!(stdout(&output), "Vegetable Soup\n");
}

#[test]
fn search_celery_prints_only_vegetable_soup() {
    let (output, _dir) = run_in_empty_dir(&["recipes", "search", "celery"]);

    assert_eq!(exit_code(&output), 0);
    assert_eq!(stdout(&output), "Vegetable Soup\n");
}

#[test]
fn search_pasta_prints_only_tomato_pasta() {
    let (output, _dir) = run_in_empty_dir(&["recipes", "search", "pasta"]);

    assert_eq!(exit_code(&output), 0);
    assert_eq!(stdout(&output), "Tomato Pasta\n");
}

#[test]
fn search_matches_whole_ingredient_names_so_oil_matches_nothing() {
    let (output, _dir) = run_in_empty_dir(&["recipes", "search", "oil"]);

    assert_eq!(exit_code(&output), 0);
    assert_eq!(stdout(&output), "No recipes found.\n");
}

#[test]
fn search_prints_recipe_names_only_not_ingredients() {
    let (output, _dir) = run_in_empty_dir(&["recipes", "search", "tomato"]);
    let text = stdout(&output);

    for ingredient in [
        "pasta",
        "garlic",
        "olive oil",
        "mozzarella",
        "basil",
        "carrot",
        "celery",
    ] {
        assert!(
            !text.contains(ingredient),
            "search output must not contain ingredient {ingredient:?}, got {text:?}"
        );
    }
}

#[test]
fn list_creates_no_files_in_the_working_directory() {
    let (output, dir) = run_in_empty_dir(&["recipes", "list"]);

    assert_eq!(exit_code(&output), 0);
    assert_eq!(entries_in(dir.path()), Vec::<String>::new());
}

#[test]
fn search_creates_no_files_in_the_working_directory() {
    let (output, dir) = run_in_empty_dir(&["recipes", "search", "tomato"]);

    assert_eq!(exit_code(&output), 0);
    assert_eq!(entries_in(dir.path()), Vec::<String>::new());
}

#[test]
fn search_without_an_ingredient_is_a_usage_error_on_stderr() {
    let (output, _dir) = run_in_empty_dir(&["recipes", "search"]);

    assert_ne!(exit_code(&output), 0);
    assert_eq!(exit_code(&output), 2);
    assert!(
        stderr(&output).contains("Usage"),
        "stderr was {:?}",
        stderr(&output)
    );
    assert_eq!(stdout(&output), "");
}

#[test]
fn recipes_without_a_subcommand_is_a_usage_error_on_stderr() {
    let (output, _dir) = run_in_empty_dir(&["recipes"]);

    assert_ne!(exit_code(&output), 0);
    assert_eq!(exit_code(&output), 2);
    assert!(
        stderr(&output).contains("Usage"),
        "stderr was {:?}",
        stderr(&output)
    );
    assert_eq!(stdout(&output), "");
}

#[test]
fn running_from_a_directory_with_files_leaves_them_untouched() {
    let dir = TempDir::new().expect("create temporary directory");
    fs::write(dir.path().join("notes.txt"), "keep me").expect("write fixture");

    let output = run_in(dir.path(), &["recipes", "search", "egg"]);

    assert_eq!(exit_code(&output), 0);
    assert_eq!(stdout(&output), "Cheese Omelette\n");
    assert_eq!(entries_in(dir.path()), vec!["notes.txt".to_string()]);
}

#[test]
fn show_tomato_pasta_prints_its_ingredients_in_catalog_order() {
    let (output, _dir) = run_in_empty_dir(&["recipes", "show", "Tomato Pasta"]);

    assert_eq!(exit_code(&output), 0);
    assert_eq!(
        stdout(&output),
        "pasta 200 g\ntomato 400 g\ngarlic 2 cloves\nolive oil 2 tbsp\n"
    );
    assert_eq!(stderr(&output), "");
}

#[test]
fn show_prints_every_other_recipe_with_its_quantities_in_catalog_order() {
    let cases = [
        (
            "Mushroom Risotto",
            "rice 300 g\nmushroom 250 g\nonion 1 piece\nbutter 50 g\nparmesan 80 g\n",
        ),
        (
            "Caprese Salad",
            "tomato 300 g\nmozzarella 250 g\nbasil 10 g\nolive oil 1 tbsp\n",
        ),
        (
            "Vegetable Soup",
            "carrot 200 g\npotato 300 g\nonion 1 piece\ncelery 150 g\ntomato 400 g\n",
        ),
        (
            "Cheese Omelette",
            "egg 3 pieces\ncheese 50 g\nbutter 10 g\n",
        ),
    ];
    for (name, expected) in cases {
        let (output, _dir) = run_in_empty_dir(&["recipes", "show", name]);

        assert_eq!(exit_code(&output), 0, "exit code for {name}");
        assert_eq!(stdout(&output), expected, "output for {name}");
    }
}

#[test]
fn show_matches_the_name_ignoring_ascii_case() {
    let (lower, _dir) = run_in_empty_dir(&["recipes", "show", "tomato pasta"]);
    let (exact, _other) = run_in_empty_dir(&["recipes", "show", "Tomato Pasta"]);

    assert_eq!(exit_code(&lower), 0);
    assert_eq!(stdout(&lower), stdout(&exact));
}

#[test]
fn show_an_unknown_recipe_fails_with_a_message_on_stderr() {
    let (output, _dir) = run_in_empty_dir(&["recipes", "show", "Pizza"]);

    assert_eq!(exit_code(&output), 1);
    assert_eq!(stderr(&output), "Unknown recipe: Pizza\n");
    assert_eq!(stdout(&output), "");
}

#[test]
fn show_an_unknown_recipe_creates_no_file() {
    let (output, dir) = run_in_empty_dir(&["recipes", "show", "Pizza"]);

    assert_eq!(exit_code(&output), 1);
    assert_eq!(entries_in(dir.path()), Vec::<String>::new());
}

#[test]
fn show_an_unknown_name_close_to_one_recipe_suggests_it_on_a_second_stderr_line() {
    let (output, _dir) = run_in_empty_dir(&["recipes", "show", "Tomatto Pasta"]);

    assert_eq!(exit_code(&output), 1);
    assert_eq!(
        stderr(&output),
        "Unknown recipe: Tomatto Pasta\nDid you mean: Tomato Pasta?\n"
    );
    assert_eq!(stdout(&output), "");
}

#[test]
fn show_suggestion_ignores_ascii_case() {
    let (output, _dir) = run_in_empty_dir(&["recipes", "show", "CAPRESE SALDA"]);

    assert_eq!(exit_code(&output), 1);
    assert_eq!(
        stderr(&output),
        "Unknown recipe: CAPRESE SALDA\nDid you mean: Caprese Salad?\n"
    );
    assert_eq!(stdout(&output), "");
}

#[test]
fn show_an_unknown_name_far_from_every_recipe_prints_no_suggestion() {
    let (output, _dir) = run_in_empty_dir(&["recipes", "show", "Tomato"]);

    assert_eq!(exit_code(&output), 1);
    assert_eq!(stderr(&output), "Unknown recipe: Tomato\n");
    assert_eq!(stdout(&output), "");
}

#[test]
fn show_without_a_name_is_a_usage_error_on_stderr() {
    let (output, _dir) = run_in_empty_dir(&["recipes", "show"]);

    assert_eq!(exit_code(&output), 2);
    assert!(
        stderr(&output).contains("Usage"),
        "stderr was {:?}",
        stderr(&output)
    );
    assert_eq!(stdout(&output), "");
}

#[test]
fn show_creates_no_files_in_the_working_directory() {
    let (output, dir) = run_in_empty_dir(&["recipes", "show", "Tomato Pasta"]);

    assert_eq!(exit_code(&output), 0);
    assert_eq!(entries_in(dir.path()), Vec::<String>::new());
}

#[test]
fn show_succeeds_without_reading_a_malformed_list_file() {
    let dir = TempDir::new().expect("create temporary directory");
    fs::write(dir.path().join("ricette-list.json"), "{ this is not json").expect("write fixture");

    let output = run_in(dir.path(), &["recipes", "show", "Tomato Pasta"]);

    assert_eq!(exit_code(&output), 0);
    assert_eq!(
        stdout(&output),
        "pasta 200 g\ntomato 400 g\ngarlic 2 cloves\nolive oil 2 tbsp\n"
    );
    assert_eq!(
        fs::read_to_string(dir.path().join("ricette-list.json")).expect("read fixture"),
        "{ this is not json"
    );
}

#[test]
fn show_with_servings_2_doubles_every_quantity_of_tomato_pasta() {
    let (output, _dir) = run_in_empty_dir(&["recipes", "show", "Tomato Pasta", "--servings", "2"]);

    assert_eq!(exit_code(&output), 0);
    assert_eq!(
        stdout(&output),
        "pasta 400 g\ntomato 800 g\ngarlic 4 cloves\nolive oil 4 tbsp\n"
    );
    assert_eq!(stderr(&output), "");
}

#[test]
fn show_with_servings_1_prints_exactly_the_same_as_without_the_flag() {
    let (without, _dir) = run_in_empty_dir(&["recipes", "show", "Tomato Pasta"]);
    let (with, _other) = run_in_empty_dir(&["recipes", "show", "Tomato Pasta", "--servings", "1"]);

    assert_eq!(exit_code(&with), 0);
    assert_eq!(stdout(&with), stdout(&without));
    assert_eq!(stderr(&with), "");
}

#[test]
fn show_with_servings_2_prints_twice_the_catalog_quantities_for_every_recipe() {
    let cases = [
        (
            "Tomato Pasta",
            "pasta 400 g\ntomato 800 g\ngarlic 4 cloves\nolive oil 4 tbsp\n",
        ),
        (
            "Mushroom Risotto",
            "rice 600 g\nmushroom 500 g\nonion 2 piece\nbutter 100 g\nparmesan 160 g\n",
        ),
        (
            "Caprese Salad",
            "tomato 600 g\nmozzarella 500 g\nbasil 20 g\nolive oil 2 tbsp\n",
        ),
        (
            "Vegetable Soup",
            "carrot 400 g\npotato 600 g\nonion 2 piece\ncelery 300 g\ntomato 800 g\n",
        ),
        (
            "Cheese Omelette",
            "egg 6 pieces\ncheese 100 g\nbutter 20 g\n",
        ),
    ];
    for (name, expected) in cases {
        let (output, _dir) = run_in_empty_dir(&["recipes", "show", name, "--servings", "2"]);

        assert_eq!(exit_code(&output), 0, "exit code for {name}");
        assert_eq!(stdout(&output), expected, "output for {name}");
        assert_eq!(stderr(&output), "", "stderr for {name}");
    }
}

#[test]
fn show_with_an_invalid_servings_value_is_a_usage_error_with_empty_stdout() {
    for value in ["0", "-1", "two"] {
        let (output, _dir) =
            run_in_empty_dir(&["recipes", "show", "Tomato Pasta", "--servings", value]);

        assert_eq!(exit_code(&output), 2, "exit code for --servings {value}");
        // Clap's usage errors all end with this hint, whether the value is out of range or not a number.
        assert!(
            stderr(&output).starts_with("error: ")
                && stderr(&output).contains("For more information, try '--help'."),
            "stderr for --servings {value} was {:?}",
            stderr(&output)
        );
        assert_eq!(stdout(&output), "", "stdout for --servings {value}");
    }
}

#[test]
fn show_with_servings_that_overflow_fails_with_no_stdout_and_creates_no_file() {
    let (output, dir) = run_in_empty_dir(&[
        "recipes",
        "show",
        "Tomato Pasta",
        "--servings",
        "18446744073709551615",
    ]);

    assert_eq!(exit_code(&output), 1);
    assert_eq!(
        stderr(&output),
        "error: the quantity of pasta in g is too large\n"
    );
    assert_eq!(stdout(&output), "");
    assert_eq!(entries_in(dir.path()), Vec::<String>::new());
}

#[test]
fn show_with_servings_does_not_read_or_write_a_malformed_list_file() {
    let dir = TempDir::new().expect("create temporary directory");
    fs::write(dir.path().join("ricette-list.json"), "{ this is not json").expect("write fixture");

    let output = run_in(
        dir.path(),
        &["recipes", "show", "Tomato Pasta", "--servings", "2"],
    );

    assert_eq!(exit_code(&output), 0);
    assert_eq!(
        fs::read_to_string(dir.path().join("ricette-list.json")).expect("read fixture"),
        "{ this is not json"
    );
    assert_eq!(
        entries_in(dir.path()),
        vec!["ricette-list.json".to_string()]
    );
}
