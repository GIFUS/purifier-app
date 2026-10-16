use crate::cleanup::categories;
use crate::cleanup::model::CategoryDef;
use crate::common::error::Result;

#[tauri::command]
pub fn list_categories() -> Result<Vec<CategoryDef>> {
    Ok(categories::all().into_iter().copied().collect())
}

#[tauri::command]
pub fn list_groups() -> Result<Vec<(String, String)>> {
    Ok(categories::GROUPS
        .iter()
        .map(|(id, label)| ((*id).to_string(), (*label).to_string()))
        .collect())
}
