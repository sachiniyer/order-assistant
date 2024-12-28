use serde::{Deserialize, Serialize};
use std::fs;
use tracing::{debug, info};

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
        info!("Loading menu configuration");
        let menu_path =
            std::env::var("MENU_FILE").unwrap_or_else(|_| "static/menu.json".to_string());
        debug!("Reading menu from: {}", menu_path);
        let content = fs::read_to_string(menu_path)?;
        let items: Vec<MenuItem> = serde_json::from_str(&content)?;
        debug!("Loaded {} menu items", items.len());
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
        debug!(
            "Starting validation for item: {} (ID: {})",
            item.item_name, item.id
        );

        if item.option_keys.len() != item.option_values.len() {
            info!(
                "Invalid item: Option keys and values do not match for {} (ID: {}). Keys: {}, Values: {}",
                item.item_name,
                item.id,
                item.option_keys.len(),
                item.option_values.len()
            );
            return Ok(ItemStatus::Invalid(
                "Option keys and values do not match".to_string(),
            ));
        }

        let menu_item = self.items.iter().find(|i| i.item_name == item.item_name);
        debug!("Found menu item definition: {}", menu_item.is_some());

        for (option_key, option_values) in
            Iterator::zip(item.option_keys.iter(), item.option_values.iter())
        {
            if menu_item.is_none() {
                info!(
                    "Item not found in menu: {} (ID: {})",
                    item.item_name, item.id
                );
                return Ok(ItemStatus::Invalid(format!(
                    "Item does not exist: {}",
                    item.item_name
                )));
            }
            let option = menu_item.unwrap().options.get(option_key);
            debug!(
                "Validating option '{}' for item {} (ID: {}). Option exists: {}",
                option_key,
                item.item_name,
                item.id,
                option.is_some()
            );

            if option.is_none() {
                info!(
                    "Invalid option '{}' for item {} (ID: {})",
                    option_key, item.item_name, item.id
                );
                return Ok(ItemStatus::Invalid(format!(
                    "Option does not exist: {}",
                    option_key
                )));
            }
            let option = option.unwrap();

            for value in option_values {
                debug!(
                    "Checking value '{}' for option '{}' in item {} (ID: {})",
                    value, option_key, item.item_name, item.id
                );
                if !option.choices.contains_key(value) {
                    info!(
                        "Invalid choice '{}' for option '{}' in item {} (ID: {})",
                        value, option_key, item.item_name, item.id
                    );
                    return Ok(ItemStatus::Invalid(format!(
                        "Invalid choice for option {}: {}",
                        option_key, value
                    )));
                }
            }

            debug!(
                "Checking option count for '{}'. Min: {}, Max: {}, Current: {}",
                option_key,
                option.minimum,
                option.maximum,
                option_values.len()
            );

            if option_values.len() < option.minimum as usize {
                info!(
                    "Too few options for {} (ID: {}). Required: {}, Found: {}",
                    item.item_name,
                    item.id,
                    option.minimum,
                    option_values.len()
                );
                return Ok(ItemStatus::Incomplete("Too few options".to_string()));
            }
            if option_values.len() > option.maximum as usize {
                info!(
                    "Too many options for {} (ID: {}). Maximum: {}, Found: {}",
                    item.item_name,
                    item.id,
                    option.maximum,
                    option_values.len()
                );
                return Ok(ItemStatus::Invalid("Too many options".to_string()));
            }
        }

        debug!(
            "Validating required options for item {} (ID: {})",
            item.item_name, item.id
        );
        for (option_name, option_config) in menu_item.unwrap().options.iter() {
            match &option_config.required {
                RequirementConfig::Simple(true) => {
                    debug!(
                        "Checking required option '{}' for item {} (ID: {})",
                        option_name, item.item_name, item.id
                    );
                    if !item.option_keys.contains(option_name) {
                        info!(
                            "Missing required option '{}' for item {} (ID: {})",
                            option_name, item.item_name, item.id
                        );
                        return Ok(ItemStatus::Incomplete(format!(
                            "Required option missing {}",
                            option_name
                        )));
                    }
                }
                RequirementConfig::Dependent { option, value } => {
                    debug!(
                        "Checking dependent option '{}' (depends on '{}' = '{}') for item {} (ID: {})",
                        option_name, option, value, item.item_name, item.id
                    );
                    let item_option_index = item.option_keys.iter().position(|x| x == option_name);
                    #[allow(clippy::single_match)]
                    match item_option_index {
                        None => {
                            let dependent_option_index =
                                item.option_keys.iter().position(|x| x == option);
                            if dependent_option_index.is_none() {
                                info!(
                                    "Missing dependent option '{}' for item {} (ID: {})",
                                    option, item.item_name, item.id
                                );
                                return Ok(ItemStatus::Incomplete(format!(
                                    "Dependent option missing {}",
                                    option
                                )));
                            }

                            let dependent_option_value = item.option_values
                                .get(dependent_option_index.unwrap())
                                .expect("The dependent option value should exist if the dependent option exists");

                            debug!(
                                "Checking dependent value '{}' against current values {:?}",
                                value, dependent_option_value
                            );

                            if dependent_option_value.contains(value) {
                                info!(
                                    "Missing required dependent option '{}' for item {} (ID: {})",
                                    option_name, item.item_name, item.id
                                );
                                return Ok(ItemStatus::Incomplete(format!(
                                    "Dependent option missing {}",
                                    option_name
                                )));
                            }
                        }
                        _ => {}
                    }
                }
                _ => {}
            }
        }

        debug!(
            "Validation successful for item {} (ID: {})",
            item.item_name, item.id
        );
        Ok(ItemStatus::Complete("Item is valid".to_string()))
    }
}
