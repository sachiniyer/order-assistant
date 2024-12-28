use serde::{Deserialize, Serialize};
use std::fs;

use crate::error::AppResult;
use crate::order::OrderItem;

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
    pub required: RequirementConfig,
    pub minimum: i32,
    pub maximum: i32,
    pub choices: std::collections::HashMap<String, Choice>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(untagged)]
pub enum RequirementConfig {
    Simple(bool),
    Dependent { option: String, value: String },
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Choice {
    pub price: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Menu {
    pub items: Vec<MenuItem>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ItemStatus {
    Incomplete,
    Complete,
    Invalid,
}

impl Menu {
    pub fn new() -> AppResult<Self> {
        let content = fs::read_to_string(
            std::env::var("MENU_FILE").unwrap_or_else(|_| "static/menu.json".to_string()),
        )?;
        let items: Vec<MenuItem> = serde_json::from_str(&content)?;
        Ok(Menu { items })
    }

    pub fn validate_item(&self, item: &OrderItem) -> AppResult<ItemStatus> {
        // TODO(siyer): Could validate the price here as well, but that is skipped for now
        //              Additionally, could provide more hints on why the item is invalid.
        //              This function just provides hints to GPT on what is missing/needs to be changed
        let menu_item = self.items.iter().find(|i| i.item_name == item.item_name);
        // NOTE(dev): Validate that the options exist on the item, and that the values are valid
        for (option_key, option_values) in
            Iterator::zip(item.option_keys.iter(), item.option_values.iter())
        {
            let option = menu_item.unwrap().options.get(option_key);
            if option.is_none() {
                return Ok(ItemStatus::Invalid);
            }
            let option = option.unwrap();
            for value in option_values {
                if !option.choices.contains_key(value) {
                    return Ok(ItemStatus::Invalid);
                }
            }
            if option_values.len() < option.minimum as usize
                || option_values.len() > option.maximum as usize
            {
                return Ok(ItemStatus::Incomplete);
            }
        }

        // NOTE(dev): Validate that all required options are present
        for (option_name, option_config) in menu_item.unwrap().options.iter() {
            match &option_config.required {
                RequirementConfig::Simple(true) => {
                    if !item.option_keys.contains(&option_name) {
                        return Ok(ItemStatus::Incomplete);
                    }
                }
                RequirementConfig::Dependent { option, value } => {
                    let item_index = item.option_keys.iter().position(|x| x == option);
                    match item_index {
                        Some(index) => {
                            if !item.option_values.get(index).unwrap().contains(&value) {
                                return Ok(ItemStatus::Incomplete);
                            }
                        }
                        None => {
                            return Ok(ItemStatus::Incomplete);
                        }
                    }
                }
                _ => {}
            }
        }
        Ok(ItemStatus::Complete)
    }
}
