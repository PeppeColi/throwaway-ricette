# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).
The agent adds one line under "Unreleased" in each pull request; when you release, move those
lines under a new heading with the version and the date.

## [Unreleased]

### Added

- `ricette recipes list` and `ricette recipes search <ingredient>` show the built-in catalog of five recipes.
- `ricette list add`, `ricette list show` and `ricette list clear` keep a shopping list in `ricette-list.json` in the current directory.
- `ricette list add <item> --qty <n> --unit <u>` records an amount; adding the same item in the same unit again sums into one line, and `ricette list show` prints amounts next to items.
- `ricette list count` prints how many entries the shopping list has, as `<n> entries` (`1 entry` for one).
- `ricette list last` prints the item of the most recently added shopping list entry, or `The shopping list is empty.`
- `ricette list add-recipe "<name>"` adds every ingredient of a recipe to the shopping list with its quantity and unit; an unknown recipe name exits 1 and leaves the list unchanged.
- `ricette recipes show "<name>"` prints the ingredients of a recipe, one per line as `<item> <quantity> <unit>`; an unknown recipe name exits 1.
- `ricette recipes show "<name>" --servings <n>` multiplies every quantity by `n`, a positive whole number that defaults to 1.
- `ricette list add-recipe "<name>" --servings <n>` adds the recipe scaled by `n` to the shopping list, summing with amounts already listed, and prints the scaled amounts.
- `ricette list bought "<item>"` marks every shopping list entry with that item as bought, and `ricette list show` ends their lines with `[bought]`; adding an amount to a bought entry unmarks it, and existing `ricette-list.json` files keep working.
