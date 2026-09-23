use app::model::{Category, CategoryKind, EmojiChange};
use app::services::CreateCategory;
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

#[derive(Debug, Serialize, ToSchema)]
pub struct CategoryResponse {
    pub id: Uuid,
    #[schema(example = "mercado")]
    pub name: String,
    #[schema(value_type = String, example = "expense")]
    pub kind: CategoryKind,
    #[schema(example = "🛒")]
    pub emoji: Option<String>,
    pub archived: bool,
    /// Part of the basic cost of living (expense categories only).
    pub essential: bool,
}

impl From<Category> for CategoryResponse {
    fn from(category: Category) -> Self {
        let Category { id, name, kind, emoji, archived, essential } = category;
        Self { id: id.0, name, kind, emoji, archived, essential }
    }
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateCategoryBody {
    #[schema(example = "farmácia")]
    pub name: String,
    #[schema(value_type = String, example = "expense")]
    pub kind: CategoryKind,
    pub emoji: Option<String>,
    /// Defaults to false; only expense categories can be essential.
    #[serde(default)]
    pub essential: bool,
}

impl From<CreateCategoryBody> for CreateCategory {
    fn from(body: CreateCategoryBody) -> Self {
        Self { name: body.name, kind: body.kind, emoji: body.emoji, essential: body.essential }
    }
}

/// Fields left out keep their value; `emoji: null` clears the emoji.
#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateCategoryBody {
    pub essential: Option<bool>,
    #[schema(example = "feira")]
    pub name: Option<String>,
    // The three states a PATCH needs: missing keeps the emoji, `null`
    // clears it, a string sets it. `emoji_change` names them.
    #[allow(clippy::option_option)]
    #[serde(default, deserialize_with = "crate::dto::given_field")]
    pub emoji: Option<Option<String>>,
}

impl UpdateCategoryBody {
    /// The change the body asks for: missing keeps, `null` clears.
    pub fn emoji_change(&self) -> EmojiChange {
        match &self.emoji {
            None => EmojiChange::Keep,
            Some(None) => EmojiChange::Clear,
            Some(Some(emoji)) => EmojiChange::Set(emoji.clone()),
        }
    }
}

#[derive(Debug, Deserialize, IntoParams)]
pub struct ListCategoriesQuery {
    /// `expense` or `income`; omit for both.
    #[param(value_type = Option<String>)]
    pub kind: Option<CategoryKind>,
}
