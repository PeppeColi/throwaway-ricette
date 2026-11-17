use clap::{Parser, Subcommand};

/// A recipe in the built-in catalog.
struct Recipe {
    name: &'static str,
    ingredients: &'static [&'static str],
}

/// The fixed catalog, embedded in the binary. Declaration order is display order.
const CATALOG: &[Recipe] = &[
    Recipe {
        name: "Tomato Pasta",
        ingredients: &["pasta", "tomato", "garlic", "olive oil"],
    },
    Recipe {
        name: "Mushroom Risotto",
        ingredients: &["rice", "mushroom", "onion", "butter", "parmesan"],
    },
    Recipe {
        name: "Caprese Salad",
        ingredients: &["tomato", "mozzarella", "basil", "olive oil"],
    },
    Recipe {
        name: "Vegetable Soup",
        ingredients: &["carrot", "potato", "onion", "celery", "tomato"],
    },
    Recipe {
        name: "Cheese Omelette",
        ingredients: &["egg", "cheese", "butter"],
    },
];

#[derive(Parser)]
#[command(name = "ricette", about = "Look up built-in recipes")]
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

fn main() {
    let cli = Cli::parse();
    match cli.command {
        Command::Recipes { action } => match action {
            RecipesAction::List => list_recipes(),
            RecipesAction::Search { ingredient } => search_recipes(&ingredient),
        },
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
            .any(|name| name.eq_ignore_ascii_case(ingredient))
        {
            println!("{}", recipe.name);
            found = true;
        }
    }
    if !found {
        println!("No recipes found.");
    }
}
