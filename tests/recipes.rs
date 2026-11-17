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
