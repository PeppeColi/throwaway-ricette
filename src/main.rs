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
    /// Append an item to the shopping list.
    Add {
        /// The item to add; it must not be empty.
        #[arg(value_parser = non_empty)]
        item: String,
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

/// One item of the shopping list. Later work may add optional fields.
#[derive(Debug, Serialize, Deserialize)]
struct ShoppingListEntry {
    item: String,
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
            ListAction::Add { item } => add(&item),
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

fn add(item: &str) -> Result<(), String> {
    let mut list = load()?;
    list.entries.push(ShoppingListEntry {
        item: item.to_string(),
    });
    save(&list)?;
    println!("Added: {item}");
    Ok(())
}

fn show() -> Result<(), String> {
    let list = load()?;
    if list.entries.is_empty() {
        println!("The shopping list is empty.");
    }
    for entry in &list.entries {
        println!("{}", entry.item);
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
