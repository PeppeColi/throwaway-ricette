use std::fs;
use std::io::ErrorKind;
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
        Command::Recipes { action } => {
            match action {
                RecipesAction::List => list_recipes(),
                RecipesAction::Search { ingredient } => search_recipes(&ingredient),
            }
            Ok(())
        }
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
    let mut list = load()?;
    for ingredient in recipe.ingredients {
        add_quantified(
            &mut list,
            ingredient.item,
            ingredient.quantity,
            ingredient.unit,
        )?;
    }
    save(&list)?;
    for ingredient in recipe.ingredients {
        println!(
            "Added: {} {} {}",
            ingredient.item, ingredient.quantity, ingredient.unit
        );
    }
    Ok(())
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

fn save(list: &ShoppingList) -> Result<(), String> {
    let mut text = serde_json::to_string_pretty(list)
        .map_err(|e| format!("cannot serialize the shopping list: {e}"))?;
    text.push('\n');
    fs::write(LIST_FILE, text).map_err(|e| format!("cannot write {LIST_FILE}: {e}"))
}
