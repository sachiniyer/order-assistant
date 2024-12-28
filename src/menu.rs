use serde::{Deserialize, Serialize};
use std::fs;

use crate::error::AppResult;
use crate::order::OrderItem;

/// Represents a single item on the menu
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MenuItem {
    /// Name of the menu item
    #[serde(rename = "itemName")]
    pub item_name: String,
    /// Category/type of the item
    #[serde(rename = "itemType")]
    pub item_type: String,
    /// Description of the item
    pub description: String,
    /// Available customization options
    pub options: std::collections::HashMap<String, OptionConfig>,
}

/// Configuration for a customization option
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct OptionConfig {
    /// Whether and how the option is required
    pub required: RequirementConfig,
    /// Minimum number of choices required
    pub minimum: i32,
    /// Maximum number of choices allowed
    pub maximum: i32,
    /// Available choices for this option
    pub choices: std::collections::HashMap<String, Choice>,
}

/// Requirement configuration for an option
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(untagged)]
pub enum RequirementConfig {
    /// Simple boolean requirement
    Simple(bool),
    /// Requirement dependent on another option
    Dependent { option: String, value: String },
}

/// Price configuration for an option choice
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Choice {
    /// Additional price for this choice
    pub price: f64,
}

/// Complete menu configuration
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Menu {
    /// List of available menu items
    pub items: Vec<MenuItem>,
}

/// Status of an item's validation against menu requirements
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ItemStatus {
    /// Item is missing required options
    Incomplete(String),
    /// Item meets all requirements
    Complete(String),
    /// Item has invalid options
    Invalid(String),
}

impl Menu {
    /// Creates a new Menu instance from the menu file.
    ///
    /// # Returns
    /// * `AppResult<Self>` - The loaded menu or an error
    pub fn new() -> AppResult<Self> {
        let content = fs::read_to_string(
            std::env::var("MENU_FILE").unwrap_or_else(|_| "static/menu.json".to_string()),
        )?;
        let items: Vec<MenuItem> = serde_json::from_str(&content)?;
        Ok(Menu { items })
    }

    /// Validates an order item against the menu requirements.
    ///
    /// # Arguments
    /// * `item` - The order item to validate
    ///
    /// # Returns
    /// * `AppResult<ItemStatus>` - The validation status of the item
    pub fn validate_item(&self, item: &OrderItem) -> AppResult<ItemStatus> {
        // NOTE(dev): This function essentially provides hints to GPT on what is needs to be changed
        //            The wording could be improved to prompt GPT better
        let menu_item = self.items.iter().find(|i| i.item_name == item.item_name);
        if item.option_keys.len() != item.option_values.len() {
            return Ok(ItemStatus::Invalid(
                "Option keys and values do not match".to_string(),
            ));
        }
        for (option_key, option_values) in
            Iterator::zip(item.option_keys.iter(), item.option_values.iter())
        {
            if menu_item.is_none() {
                return Ok(ItemStatus::Invalid(format!(
                    "Item does not exist: {}",
                    item.item_name
                )));
            }
            let option = menu_item.unwrap().options.get(option_key);
            if option.is_none() {
                return Ok(ItemStatus::Invalid(format!(
                    "Option does not exist: {}",
                    option_key
                )));
            }
            let option = option.unwrap();
            for value in option_values {
                if !option.choices.contains_key(value) {
                    return Ok(ItemStatus::Invalid(format!(
                        "Invalid choice for option {}: {}",
                        option_key, value
                    )));
                }
            }
            if option_values.len() < option.minimum as usize {
                return Ok(ItemStatus::Incomplete("Too few options".to_string()));
            }
            if option_values.len() > option.maximum as usize {
                return Ok(ItemStatus::Invalid("Too many options".to_string()));
            }
        }

        // NOTE(dev): Validate that all required options are present
        for (option_name, option_config) in menu_item.unwrap().options.iter() {
            match &option_config.required {
                RequirementConfig::Simple(true) => {
                    if !item.option_keys.contains(&option_name) {
                        return Ok(ItemStatus::Incomplete(format!(
                            "Required option missing {}",
                            option_name
                        )));
                    }
                }
                RequirementConfig::Dependent { option, value } => {
                    let item_option_index = item.option_keys.iter().position(|x| x == option_name);
                    match item_option_index {
                        None => {
                            // NOTE(dev): If the option is not present, we need to check if the dependent option is present
                            let dependent_option_index =
                                item.option_keys.iter().position(|x| x == option);
                            // NOTE(dev): If the dependent option is not present, it is incomplete
                            if dependent_option_index.is_none() {
                                return Ok(ItemStatus::Incomplete(format!(
                                    "Dependent option missing {}",
                                    option
                                )));
                            };
                            // NOTE(dev): If the dependent option is present, we need to check the value
                            let dependent_option_value =
                                item.option_values.get(dependent_option_index.unwrap()).expect(
                                    "The dependent option value should exist if the dependent option exists",
                                );
                            // NOTE(dev): If the dependent option contains the dependent value, the item is incomplete
                            if dependent_option_value.contains(value) {
                                return Ok(ItemStatus::Incomplete(format!(
                                    "Dependent option missing {}",
                                    option_name
                                )));
                            }
                        }
                        // NOTE(dev): If the option is present, we don't need to check the dependent option
                        _ => {}
                    }
                }
                _ => {}
            }
        }
        Ok(ItemStatus::Complete("Item is valid".to_string()))
    }
}
