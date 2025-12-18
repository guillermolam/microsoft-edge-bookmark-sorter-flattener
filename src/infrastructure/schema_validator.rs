use anyhow::{anyhow, Result};
use jsonschema::{Draft, JSONSchema};
use once_cell::sync::Lazy;
use serde_json::Value;

static BOOKMARKS_SCHEMA: Lazy<JSONSchema> = Lazy::new(|| {
    let schema_content = include_str!("../schemas/bookmarks_schema.json");
    let schema: Value = serde_json::from_str(schema_content).expect("Invalid bookmarks schema");
    JSONSchema::options()
        .with_draft(Draft::Draft7)
        .compile(&schema)
        .expect("Failed to compile bookmarks schema")
});

static FOLDER_SCHEMA: Lazy<JSONSchema> = Lazy::new(|| {
    let schema_content = include_str!("../schemas/folder_schema.json");
    let schema: Value = serde_json::from_str(schema_content).expect("Invalid folder schema");
    JSONSchema::options()
        .with_draft(Draft::Draft7)
        .compile(&schema)
        .expect("Failed to compile folder schema")
});

static URL_SCHEMA: Lazy<JSONSchema> = Lazy::new(|| {
    let schema_content = include_str!("../schemas/url_schema.json");
    let schema: Value = serde_json::from_str(schema_content).expect("Invalid URL schema");
    JSONSchema::options()
        .with_draft(Draft::Draft7)
        .compile(&schema)
        .expect("Failed to compile URL schema")
});

/// Validate the entire bookmarks file against the bookmarks schema
pub fn validate_bookmarks_file(bookmarks: &Value) -> Result<()> {
    match BOOKMARKS_SCHEMA.validate(bookmarks) {
        Ok(()) => Ok(()),
        Err(errors) => {
            let error_list: Vec<String> = errors.map(|e| e.to_string()).collect();
            Err(anyhow!(
                "Bookmarks file validation failed:\n{}",
                error_list.join("\n")
            ))
        }
    }
}

/// Validate a folder item against the folder schema
pub fn validate_folder_item(folder: &Value) -> Result<()> {
    match FOLDER_SCHEMA.validate(folder) {
        Ok(()) => Ok(()),
        Err(errors) => {
            let error_list: Vec<String> = errors.map(|e| e.to_string()).collect();
            Err(anyhow!(
                "Folder item validation failed:\n{}",
                error_list.join("\n")
            ))
        }
    }
}

/// Validate a URL item against the URL schema
pub fn validate_url_item(url_item: &Value) -> Result<()> {
    match URL_SCHEMA.validate(url_item) {
        Ok(()) => Ok(()),
        Err(errors) => {
            let error_list: Vec<String> = errors.map(|e| e.to_string()).collect();
            Err(anyhow!(
                "URL item validation failed:\n{}",
                error_list.join("\n")
            ))
        }
    }
}

/// Recursively validate all items in the bookmarks structure
pub fn validate_all_bookmark_items(bookmarks: &Value) -> Result<()> {
    if let Some(roots) = bookmarks.get("roots") {
        if let Some(roots_obj) = roots.as_object() {
            for (_root_name, root_folder) in roots_obj {
                validate_bookmark_tree(root_folder)?;
            }
        }
    }

    Ok(())
}

/// Recursively validate a bookmark tree (folder with children)
fn validate_bookmark_tree(node: &Value) -> Result<()> {
    // Validate the current node
    if let Some(node_type) = node.get("type") {
        if let Some(type_str) = node_type.as_str() {
            match type_str {
                "folder" => {
                    validate_folder_item(node)?;
                    // Recursively validate children
                    if let Some(children) = node.get("children") {
                        if let Some(children_array) = children.as_array() {
                            for child in children_array {
                                validate_bookmark_tree(child)?;
                            }
                        }
                    }
                }
                "url" => {
                    validate_url_item(node)?;
                }
                _ => {
                    // Unknown node types are permitted by higher-level business
                    // validation; schema-validator should not reject them here.
                    // Skip schema validation for unknown node types.
                    return Ok(());
                }
            }
        }
    }

    Ok(())
}
