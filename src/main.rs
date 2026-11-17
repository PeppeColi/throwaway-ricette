use std::fs;
use std::io::ErrorKind;
use std::process::ExitCode;

use clap::{Parser, Subcommand};
use serde::{Deserialize, Serialize};

/// The shopping list lives in this file, relative to the working directory.
const LIST_FILE: &str = "ricette-list.json";

/// Cooking helpers for the terminal.
#[derive(Parser)]
#[command(name = "ricette", version, about)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Keep and print the shopping list.
    List {
        #[command(subcommand)]
        action: ListAction,
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
        Command::List { action } => match action {
            ListAction::Add { item, qty, unit } => add(&item, qty, unit.as_deref()),
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
