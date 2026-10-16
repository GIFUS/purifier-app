pub mod applications;
pub mod system;
pub mod user;

use crate::cleanup::model::CategoryDef;
use crate::common::error::{AppError, Result};

pub use crate::cleanup::locate::env;
pub use crate::cleanup::locate::fixed;
pub use crate::cleanup::model::{CleanupMode, Danger, Locator};

pub const GROUPS: &[(&str, &str)] = &[
    ("system", "System"),
    ("user", "User"),
    ("applications", "Applications"),
];

pub fn all() -> Vec<&'static CategoryDef> {
    system::SYSTEM
        .iter()
        .chain(user::USER)
        .chain(applications::APPLICATIONS)
        .collect()
}

pub fn lookup(id: &str) -> Result<&'static CategoryDef> {
    all()
        .into_iter()
        .find(|c| c.id == id)
        .ok_or_else(|| AppError::UnknownCategory(id.to_string()))
}

pub fn select(ids: Option<&[String]>) -> Result<Vec<&'static CategoryDef>> {
    match ids {
        None => Ok(all()),
        Some([]) => Ok(Vec::new()),
        Some(ids) => ids.iter().map(|id| lookup(id)).collect(),
    }
}
