use std::fs;
use std::io::{ErrorKind, Write};
use std::process::ExitCode;

use clap::{Parser, Subcommand};
use serde::{Deserialize, Serialize};

/// The shopping list lives in this file, relative to the working directory.
const LIST_FILE: &str = "ricette-list.json";

/// A recipe in the built-in catalog.
struct Recipe {
    name: &'static str,
    ingredients: &'static [Ingredient],
}

/// An item a recipe needs, with the quantity and unit required.
struct Ingredient {
    item: &'static str,
    quantity: u64,
    unit: &'static str,
}

/// The fixed catalog, embedded in the binary. Declaration order is display order.
const CATALOG: &[Recipe] = &[
    Recipe {
        name: "Tomato Pasta",
        ingredients: &[
            Ingredient {
                item: "pasta",
                quantity: 200,
                unit: "g",
            },
            Ingredient {
                item: "tomato",
                quantity: 400,
                unit: "g",
            },
            Ingredient {
                item: "garlic",
                quantity: 2,
                unit: "cloves",
            },
            Ingredient {
                item: "olive oil",
                quantity: 2,
                unit: "tbsp",
            },
        ],
    },
    Recipe {
        name: "Mushroom Risotto",
        ingredients: &[
            Ingredient {
                item: "rice",
                quantity: 300,
                unit: "g",
            },
            Ingredient {
                item: "mushroom",
                quantity: 250,
                unit: "g",
            },
            Ingredient {
                item: "onion",
                quantity: 1,
                unit: "piece",
            },
            Ingredient {
                item: "butter",
                quantity: 50,
                unit: "g",
            },
            Ingredient {
                item: "parmesan",
                quantity: 80,
                unit: "g",
            },
        ],
    },
    Recipe {
        name: "Caprese Salad",
        ingredients: &[
            Ingredient {
                item: "tomato",
                quantity: 300,
                unit: "g",
            },
            Ingredient {
                item: "mozzarella",
                quantity: 250,
                unit: "g",
            },
            Ingredient {
                item: "basil",
                quantity: 10,
                unit: "g",
            },
            Ingredient {
                item: "olive oil",
                quantity: 1,
                unit: "tbsp",
            },
        ],
    },
    Recipe {
        name: "Vegetable Soup",
        ingredients: &[
            Ingredient {
                item: "carrot",
                quantity: 200,
                unit: "g",
            },
            Ingredient {
                item: "potato",
                quantity: 300,
                unit: "g",
            },
            Ingredient {
                item: "onion",
                quantity: 1,
                unit: "piece",
            },
            Ingredient {
                item: "celery",
                quantity: 150,
                unit: "g",
            },
            Ingredient {
                item: "tomato",
                quantity: 400,
                unit: "g",
            },
        ],
    },
    Recipe {
        name: "Cheese Omelette",
        ingredients: &[
            Ingredient {
                item: "egg",
                quantity: 3,
                unit: "pieces",
            },
            Ingredient {
                item: "cheese",
                quantity: 50,
                unit: "g",
            },
            Ingredient {
                item: "butter",
                quantity: 10,
                unit: "g",
            },
        ],
    },
];

/// Cooking helpers for the terminal.
#[derive(Parser)]
#[command(name = "ricette", version, about)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Work with the built-in recipes.
    Recipes {
        #[command(subcommand)]
        action: RecipesAction,
    },
    /// Keep and print the shopping list.
    List {
        #[command(subcommand)]
        action: ListAction,
    },
}

#[derive(Subcommand)]
enum RecipesAction {
    /// Print the names of all recipes.
    List,
    /// Print the names of the recipes that use an ingredient.
    Search {
        /// The ingredient to look for, compared ignoring ASCII case.
        ingredient: String,
    },
    /// Print the ingredients of a recipe, one per line, with quantity and unit.
    Show {
        /// The recipe name, compared ignoring ASCII case; quote names with spaces.
        name: String,
    },
}

#[derive(Subcommand)]
enum ListAction {
    /// Append an item to the shopping list, optionally with an amount.
    Add {
        /// The item to add; it must not be empty.
        #[arg(value_parser = non_empty)]
        item: String,
        /// How many units of the item to add; a positive integer. Needs `--unit`.
        #[arg(long, requires = "unit", value_parser = clap::value_parser!(u64).range(1..))]
        qty: Option<u64>,
        /// The unit the quantity is counted in; it must not be empty. Needs `--qty`.
        #[arg(long, requires = "qty", value_parser = non_empty)]
        unit: Option<String>,
    },
    /// Add every ingredient of a recipe to the shopping list, with its quantities.
    AddRecipe {
        /// The recipe name, compared ignoring ASCII case; quote names with spaces.
        name: String,
    },
    /// Print the shopping list, one item per line.
    Show,
    /// Print how many entries the shopping list has.
    Count,
    /// Print the item of the most recently added entry of the shopping list.
    Last,
    /// Empty the shopping list.
    Clear,
}

/// The contents of `ricette-list.json`.
#[derive(Debug, Default, Serialize, Deserialize)]
struct ShoppingList {
    entries: Vec<ShoppingListEntry>,
}

/// One item of the shopping list. An entry with both `quantity` and `unit` is
/// quantified; an entry without them is plain.
#[derive(Debug, Serialize, Deserialize)]
struct ShoppingListEntry {
    item: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    quantity: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    unit: Option<String>,
}

impl ShoppingListEntry {
    /// The entry as shown to the user: `<item> <quantity> <unit>` when quantified,
    /// `<item>` when plain.
    fn display(&self) -> String {
        match (self.quantity, &self.unit) {
            (Some(quantity), Some(unit)) => format!("{} {quantity} {unit}", self.item),
            _ => self.item.clone(),
        }
    }
}

fn non_empty(value: &str) -> Result<String, String> {
    if value.is_empty() {
        Err("the item must not be empty".to_string())
    } else {
        Ok(value.to_string())
    }
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    let result = match cli.command {
        Command::Recipes { action } => match action {
            RecipesAction::List => {
                list_recipes();
                Ok(())
            }
            RecipesAction::Search { ingredient } => {
                search_recipes(&ingredient);
                Ok(())
            }
            RecipesAction::Show { name } => {
                let Some(recipe) = find_recipe(&name) else {
                    eprintln!("Unknown recipe: {name}");
                    return ExitCode::from(1);
                };
                show_recipe(recipe)
            }
        },
        Command::List { action } => match action {
            ListAction::Add { item, qty, unit } => add(&item, qty, unit.as_deref()),
            ListAction::AddRecipe { name } => {
                let Some(recipe) = find_recipe(&name) else {
                    eprintln!("Unknown recipe: {name}");
                    return ExitCode::from(1);
                };
                add_recipe(recipe)
            }
            ListAction::Show => show(),
            ListAction::Count => count(),
            ListAction::Last => last(),
            ListAction::Clear => clear(),
        },
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("error: {message}");
            ExitCode::from(1)
        }
    }
}

fn list_recipes() {
    for recipe in CATALOG {
        println!("{}", recipe.name);
    }
}

fn search_recipes(ingredient: &str) {
    let mut found = false;
    for recipe in CATALOG {
        if recipe
            .ingredients
            .iter()
            .any(|entry| entry.item.eq_ignore_ascii_case(ingredient))
        {
            println!("{}", recipe.name);
            found = true;
        }
    }
    if !found {
        println!("No recipes found.");
    }
}

/// Prints the ingredients of `recipe` for one serving. Reads and writes no files.
fn show_recipe(recipe: &Recipe) -> Result<(), String> {
    // Scaled before the first line is printed, so a failure prints nothing.
    let ingredients = scaled_ingredients(recipe, 1)?;
    for ingredient in &ingredients {
        println!(
            "{} {} {}",
            ingredient.item, ingredient.quantity, ingredient.unit
        );
    }
    Ok(())
}

fn add(item: &str, qty: Option<u64>, unit: Option<&str>) -> Result<(), String> {
    let mut list = load()?;
    match (qty, unit) {
        (Some(quantity), Some(unit)) => {
            add_quantified(&mut list, item, quantity, unit)?;
            save(&list)?;
            println!("Added: {item} {quantity} {unit}");
        }
        // clap requires `--qty` and `--unit` together, so this is the plain case.
        _ => {
            list.entries.push(ShoppingListEntry {
                item: item.to_string(),
                quantity: None,
                unit: None,
            });
            save(&list)?;
            println!("Added: {item}");
        }
    }
    Ok(())
}

/// The catalog recipe whose name equals `name`, ignoring ASCII case.
fn find_recipe(name: &str) -> Option<&'static Recipe> {
    CATALOG
        .iter()
        .find(|recipe| recipe.name.eq_ignore_ascii_case(name))
}

/// Adds every ingredient of `recipe` to the list, in recipe order, with the same
/// merge rule as `add`. The list is read once and written once, so a failure
/// leaves the file unchanged and prints nothing.
fn add_recipe(recipe: &Recipe) -> Result<(), String> {
    let ingredients = scaled_ingredients(recipe, 1)?;
    let mut list = load()?;
    for ingredient in &ingredients {
        add_quantified(
            &mut list,
            ingredient.item,
            ingredient.quantity,
            ingredient.unit,
        )?;
    }
    save(&list)?;
    for ingredient in &ingredients {
        println!(
            "Added: {} {} {}",
            ingredient.item, ingredient.quantity, ingredient.unit
        );
    }
    Ok(())
}

/// The ingredients of `recipe` in recipe order, each quantity multiplied by
/// `servings`. Fails with the first ingredient whose scaled quantity does not
/// fit in a `u64`.
fn scaled_ingredients(recipe: &Recipe, servings: u64) -> Result<Vec<Ingredient>, String> {
    recipe
        .ingredients
        .iter()
        .map(|ingredient| {
            let quantity = ingredient.quantity.checked_mul(servings).ok_or_else(|| {
                format!(
                    "the quantity of {} in {} is too large",
                    ingredient.item, ingredient.unit
                )
            })?;
            Ok(Ingredient {
                item: ingredient.item,
                quantity,
                unit: ingredient.unit,
            })
        })
        .collect()
}

/// Adds `quantity` of `item` in `unit`: the first entry with the same item and
/// unit gets the sum, keeping its position; otherwise a new entry is appended.
fn add_quantified(
    list: &mut ShoppingList,
    item: &str,
    quantity: u64,
    unit: &str,
) -> Result<(), String> {
    let existing = list
        .entries
        .iter_mut()
        .find(|entry| entry.item == item && entry.unit.as_deref() == Some(unit));
    match existing {
        Some(entry) => {
            let total = entry
                .quantity
                .unwrap_or(0)
                .checked_add(quantity)
                .ok_or_else(|| format!("the quantity of {item} in {unit} is too large"))?;
            entry.quantity = Some(total);
        }
        None => list.entries.push(ShoppingListEntry {
            item: item.to_string(),
            quantity: Some(quantity),
            unit: Some(unit.to_string()),
        }),
    }
    Ok(())
}

fn show() -> Result<(), String> {
    let list = load()?;
    if list.entries.is_empty() {
        println!("The shopping list is empty.");
    }
    for entry in &list.entries {
        println!("{}", entry.display());
    }
    Ok(())
}

/// Prints the number of entries, counting duplicates. Reads the list only, so
/// the file is never written.
fn count() -> Result<(), String> {
    let list = load()?;
    let entries = list.entries.len();
    let noun = if entries == 1 { "entry" } else { "entries" };
    println!("{entries} {noun}");
    Ok(())
}

/// Prints the item of the last entry in the list, which is the most recently
/// added one. Reads the list only, so the file is never written.
fn last() -> Result<(), String> {
    let list = load()?;
    match list.entries.last() {
        Some(entry) => println!("{}", entry.item),
        None => println!("The shopping list is empty."),
    }
    Ok(())
}

fn clear() -> Result<(), String> {
    // Read first: a malformed file must be reported and left unchanged.
    load()?;
    save(&ShoppingList::default())?;
    println!("Shopping list cleared.");
    Ok(())
}

/// Reads the list. A missing file is an empty list; anything else that cannot
/// be read or parsed is an error.
fn load() -> Result<ShoppingList, String> {
    let unreadable = |detail: &dyn std::fmt::Display| {
        format!("the shopping list cannot be read from {LIST_FILE}: {detail}")
    };
    match fs::read_to_string(LIST_FILE) {
        Ok(text) => serde_json::from_str(&text).map_err(|e| unreadable(&e)),
        Err(e) if e.kind() == ErrorKind::NotFound => Ok(ShoppingList::default()),
        Err(e) => Err(unreadable(&e)),
    }
}

/// Replaces the list file. The new contents go to a temporary file next to it
/// first, so a failed write leaves the existing list unchanged.
fn save(list: &ShoppingList) -> Result<(), String> {
    let mut text = serde_json::to_string_pretty(list)
        .map_err(|e| format!("cannot serialize the shopping list: {e}"))?;
    text.push('\n');
    // One temporary file per process, created exclusively, so two saves in
    // the same directory never write to or rename each other's file.
    let temp = format!("{LIST_FILE}.{}.tmp", std::process::id());
    let result = write_new(&temp, text.as_bytes())
        .map_err(|e| format!("cannot write {temp}: {e}"))
        .and_then(|()| {
            fs::rename(&temp, LIST_FILE).map_err(|e| format!("cannot replace {LIST_FILE}: {e}"))
        });
    if result.is_err() {
        let _ = fs::remove_file(&temp);
    }
    result
}

/// Writes `contents` to a file that must not exist yet.
fn write_new(path: &str, contents: &[u8]) -> std::io::Result<()> {
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)?;
    file.write_all(contents)?;
    file.sync_all()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The item, quantity and unit of each ingredient, in order.
    fn summary(ingredients: &[Ingredient]) -> Vec<(&str, u64, &str)> {
        ingredients
            .iter()
            .map(|ingredient| (ingredient.item, ingredient.quantity, ingredient.unit))
            .collect()
    }

    #[test]
    fn find_recipe_returns_each_catalog_recipe_by_name() {
        for name in [
            "Tomato Pasta",
            "Mushroom Risotto",
            "Caprese Salad",
            "Vegetable Soup",
            "Cheese Omelette",
        ] {
            let recipe = find_recipe(name).expect("the catalog has this recipe");
            assert_eq!(recipe.name, name);
        }
    }

    #[test]
    fn find_recipe_ignores_ascii_case() {
        assert_eq!(
            find_recipe("tomato pasta").map(|r| r.name),
            Some("Tomato Pasta")
        );
        assert_eq!(
            find_recipe("CAPRESE SALAD").map(|r| r.name),
            Some("Caprese Salad")
        );
        assert_eq!(
            find_recipe("cHeEsE oMeLeTtE").map(|r| r.name),
            Some("Cheese Omelette")
        );
    }

    #[test]
    fn find_recipe_returns_none_for_unknown_names() {
        assert!(find_recipe("Pizza").is_none());
        assert!(find_recipe("").is_none());
        assert!(
            find_recipe("Tomato").is_none(),
            "a part of a name is not a match"
        );
        assert!(find_recipe("Tomato Pasta!").is_none());
    }

    #[test]
    fn scaled_ingredients_at_one_serving_match_the_catalog() {
        let expected = [
            (
                "Tomato Pasta",
                vec![
                    ("pasta", 200, "g"),
                    ("tomato", 400, "g"),
                    ("garlic", 2, "cloves"),
                    ("olive oil", 2, "tbsp"),
                ],
            ),
            (
                "Mushroom Risotto",
                vec![
                    ("rice", 300, "g"),
                    ("mushroom", 250, "g"),
                    ("onion", 1, "piece"),
                    ("butter", 50, "g"),
                    ("parmesan", 80, "g"),
                ],
            ),
            (
                "Caprese Salad",
                vec![
                    ("tomato", 300, "g"),
                    ("mozzarella", 250, "g"),
                    ("basil", 10, "g"),
                    ("olive oil", 1, "tbsp"),
                ],
            ),
            (
                "Vegetable Soup",
                vec![
                    ("carrot", 200, "g"),
                    ("potato", 300, "g"),
                    ("onion", 1, "piece"),
                    ("celery", 150, "g"),
                    ("tomato", 400, "g"),
                ],
            ),
            (
                "Cheese Omelette",
                vec![
                    ("egg", 3, "pieces"),
                    ("cheese", 50, "g"),
                    ("butter", 10, "g"),
                ],
            ),
        ];
        for (name, ingredients) in expected {
            let recipe = find_recipe(name).expect("the catalog has this recipe");
            let scaled = scaled_ingredients(recipe, 1).expect("one serving does not overflow");
            assert_eq!(summary(&scaled), ingredients, "{name} at one serving");
        }
    }

    #[test]
    fn scaled_ingredients_at_two_servings_double_every_quantity() {
        let expected = [
            (
                "Tomato Pasta",
                vec![
                    ("pasta", 400, "g"),
                    ("tomato", 800, "g"),
                    ("garlic", 4, "cloves"),
                    ("olive oil", 4, "tbsp"),
                ],
            ),
            (
                "Mushroom Risotto",
                vec![
                    ("rice", 600, "g"),
                    ("mushroom", 500, "g"),
                    ("onion", 2, "piece"),
                    ("butter", 100, "g"),
                    ("parmesan", 160, "g"),
                ],
            ),
            (
                "Caprese Salad",
                vec![
                    ("tomato", 600, "g"),
                    ("mozzarella", 500, "g"),
                    ("basil", 20, "g"),
                    ("olive oil", 2, "tbsp"),
                ],
            ),
            (
                "Vegetable Soup",
                vec![
                    ("carrot", 400, "g"),
                    ("potato", 600, "g"),
                    ("onion", 2, "piece"),
                    ("celery", 300, "g"),
                    ("tomato", 800, "g"),
                ],
            ),
            (
                "Cheese Omelette",
                vec![
                    ("egg", 6, "pieces"),
                    ("cheese", 100, "g"),
                    ("butter", 20, "g"),
                ],
            ),
        ];
        for (name, ingredients) in expected {
            let recipe = find_recipe(name).expect("the catalog has this recipe");
            let scaled = scaled_ingredients(recipe, 2).expect("two servings do not overflow");
            assert_eq!(summary(&scaled), ingredients, "{name} at two servings");
        }
    }

    #[test]
    fn scaled_ingredients_reports_the_first_quantity_that_overflows() {
        const RECIPE: Recipe = Recipe {
            name: "Overflow",
            ingredients: &[
                Ingredient {
                    item: "pasta",
                    quantity: 200,
                    unit: "g",
                },
                Ingredient {
                    item: "tomato",
                    quantity: 400,
                    unit: "g",
                },
            ],
        };
        let error = scaled_ingredients(&RECIPE, u64::MAX)
            .err()
            .expect("200 times u64::MAX does not fit in u64");
        assert_eq!(error, "the quantity of pasta in g is too large");
    }
}
