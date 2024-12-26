use crate::error::AppResult;
use serde::{Deserialize, Serialize};
use std::fs;

// This struct matches the actual JSON structure
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MenuItem {
    #[serde(rename = "itemName")]
    pub item_name: String,
    #[serde(rename = "itemType")]
    pub item_type: String,
    pub description: String,
    pub options: std::collections::HashMap<String, OptionConfig>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct OptionConfig {
    pub required: bool,
    pub minimum: i32,
    pub maximum: i32,
    pub choices: std::collections::HashMap<String, Choice>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Choice {
    pub price: f64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Menu {
    pub items: Vec<MenuItem>,
}

impl Menu {
    pub fn load() -> AppResult<Self> {
        let content = fs::read_to_string(
            std::env::var("MENU_FILE").unwrap_or_else(|_| "static/menu.json".to_string()),
        )?;
        let menu: Menu = serde_json::from_str(&content)?;
        Ok(menu)
    }
}
