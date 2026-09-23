use std::str::FromStr;

use domain::EntryKind;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use super::CategoryId;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CategoryKind {
    Expense,
    Income,
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error("unknown category kind {0:?}: expected expense or income")]
pub struct UnknownCategoryKind(pub String);

impl CategoryKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            CategoryKind::Expense => "expense",
            CategoryKind::Income => "income",
        }
    }

    /// Category kind an entry of `kind` must use, if it takes a category.
    pub const fn required_for(kind: EntryKind) -> Option<CategoryKind> {
        match kind {
            EntryKind::Income => Some(CategoryKind::Income),
            EntryKind::Expense
            | EntryKind::Refund
            | EntryKind::CardInstallment
            | EntryKind::CardCredit => Some(CategoryKind::Expense),
            _ => None,
        }
    }
}

impl FromStr for CategoryKind {
    type Err = UnknownCategoryKind;
    fn from_str(name: &str) -> Result<Self, Self::Err> {
        match name {
            "expense" => Ok(CategoryKind::Expense),
            "income" => Ok(CategoryKind::Income),
            other => Err(UnknownCategoryKind(other.to_owned())),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Category {
    pub id: CategoryId,
    pub name: String,
    pub kind: CategoryKind,
    pub emoji: Option<String>,
    pub archived: bool,
    /// Part of the basic cost of living; expense categories only.
    pub essential: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NewCategory {
    pub name: String,
    pub kind: CategoryKind,
    pub emoji: Option<String>,
    pub essential: bool,
}

/// What happens to a category's emoji when it is edited.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum EmojiChange {
    #[default]
    Keep,
    Clear,
    Set(String),
}

/// What `/editar` may change on a category; `None` keeps the name.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CategoryEdit {
    pub name: Option<String>,
    pub emoji: EmojiChange,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn entry_kinds_map_to_category_kinds() {
        assert_eq!(CategoryKind::required_for(EntryKind::Income), Some(CategoryKind::Income));
        assert_eq!(CategoryKind::required_for(EntryKind::Refund), Some(CategoryKind::Expense));
        assert_eq!(CategoryKind::required_for(EntryKind::Transfer), None);
    }

    #[test]
    fn parses_names() {
        assert_eq!("income".parse::<CategoryKind>(), Ok(CategoryKind::Income));
        assert!(
            "salary".parse::<CategoryKind>().unwrap_err().to_string().contains("expense or income")
        );
    }
}
