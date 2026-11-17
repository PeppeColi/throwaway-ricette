# Glossary

The domain vocabulary of `ricette`. Use these terms in code, tests and documentation.

- **Item**: the name of a food or product the cook wants to buy, such as "milk".
- **Shopping list**: the ordered list of items to buy, kept in `ricette-list.json`; an entry can be marked bought.
- **Shopping list entry**: one line of the shopping list, holding one item and, when known, its quantity and unit; entries with the same item and unit are summed, while plain entries are never merged.
- **Bought**: the state of a shopping list entry whose item has already been bought; `list show` marks it with `[bought]` and `list clear-bought` removes it.
- **Quantity**: how much of an item is needed, as a positive whole number (for example 2).
- **Unit**: the measure a quantity is counted in, such as grams, millilitres or pieces.
- **Recipe**: a set of instructions for cooking a dish, with the ingredients it needs for one serving.
- **Ingredient**: an item a recipe needs, with the quantity and unit required for one serving.
- **Servings**: the number of portions a recipe is cooked for; the catalog gives the ingredients for one serving.
