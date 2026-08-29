use macroquad_toolkit::data_loader::parse_json_labeled;
use serde::Deserialize;
use std::sync::OnceLock;
use tarrowyn_protocol::{MaterialStock, ProfessionKind};

#[derive(Debug, Deserialize)]
struct RecipesManifest {
    recipes: Vec<RecipeManifest>,
}

static RECIPE_CATALOG: OnceLock<Vec<RecipeManifest>> = OnceLock::new();

#[derive(Debug, Deserialize)]
struct RecipeManifest {
    id: String,
    name: String,
    profession: ProfessionKind,
    service: String,
    materials: MaterialStock,
    tools_required: u32,
    reward_gold: u32,
    benefit: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RecipeTemplate {
    pub(crate) id: String,
    pub(crate) name: String,
    pub(crate) profession: ProfessionKind,
    pub(crate) service: String,
    pub(crate) materials: MaterialStock,
    pub(crate) tools_required: u32,
    pub(crate) reward_gold: u32,
    pub(crate) benefit: String,
}

pub(super) fn validate() -> Result<(), String> {
    let recipes: RecipesManifest = parse_json_labeled(
        "recipes.json",
        macroquad_toolkit::include_json_str!("../../../assets/data/recipes.json"),
    )
    .map_err(|error| format!("recipes JSON is invalid: {error}"))?;
    validate_recipes(&recipes)
}

pub(crate) fn recipe_template(recipe_id: &str) -> RecipeTemplate {
    let recipes = RECIPE_CATALOG.get_or_init(|| {
        let recipes: RecipesManifest = parse_json_labeled(
            "recipes.json",
            macroquad_toolkit::include_json_str!("../../../assets/data/recipes.json"),
        )
        .expect("recipes content JSON must be valid");
        validate_recipes(&recipes).expect("recipes content must satisfy its schema");
        recipes.recipes
    });
    let recipe = recipes
        .iter()
        .find(|recipe| recipe.id == recipe_id)
        .expect("validated recipe catalog must contain the requested recipe");
    RecipeTemplate {
        id: recipe.id.clone(),
        name: recipe.name.clone(),
        profession: recipe.profession,
        service: recipe.service.clone(),
        materials: recipe.materials,
        tools_required: recipe.tools_required,
        reward_gold: recipe.reward_gold,
        benefit: recipe.benefit.clone(),
    }
}

fn validate_recipes(recipes: &RecipesManifest) -> Result<(), String> {
    super::validate_id_list(
        "recipe",
        recipes
            .recipes
            .iter()
            .map(|recipe| recipe.id.as_str())
            .collect(),
    )?;
    if recipes.recipes.is_empty()
        || recipes.recipes.iter().any(|recipe| {
            recipe.name.trim().is_empty()
                || recipe.service.trim().is_empty()
                || recipe.materials.tools != 0
                || recipe
                    .materials
                    .wood
                    .saturating_add(recipe.materials.iron)
                    .saturating_add(recipe.materials.cloth)
                    .saturating_add(recipe.materials.bandages)
                    == 0
                || recipe.tools_required == 0
                || recipe.reward_gold == 0
                || recipe.benefit.trim().is_empty()
        })
    {
        return Err(
            "recipes need IDs, names, professions, material costs, tools, rewards, and benefits"
                .to_owned(),
        );
    }
    if !recipes.recipes.iter().any(|recipe| {
        recipe.id == "field-tool-repair" && recipe.profession == ProfessionKind::Carpenter
    }) {
        return Err("recipes are missing the launch field-tool repair recipe".to_owned());
    }
    Ok(())
}
