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
    let errors = BOOKMARKS_SCHEMA.validate(bookmarks);
    let error_list: Vec<String> = errors.map(|e| e.to_string()).collect();

    if !error_list.is_empty() {
        return Err(anyhow!(
            "Bookmarks file validation failed:\n{}",
            error_list.join("\n")
        ));
    }

    Ok(())
}

/// Validate a folder item against the folder schema
pub fn validate_folder_item(folder: &Value) -> Result<()> {
    let errors = FOLDER_SCHEMA.validate(folder);
    let error_list: Vec<String> = errors.map(|e| e.to_string()).collect();

    if !error_list.is_empty() {
        return Err(anyhow!(
            "Folder item validation failed:\n{}",
            error_list.join("\n")
        ));
    }

    Ok(())
}

/// Validate a URL item against the URL schema
pub fn validate_url_item(url_item: &Value) -> Result<()> {
    let errors = URL_SCHEMA.validate(url_item);
    let error_list: Vec<String> = errors.map(|e| e.to_string()).collect();

    if !error_list.is_empty() {
        return Err(anyhow!(
            "URL item validation failed:\n{}",
            error_list.join("\n")
        ));
    }

    Ok(())
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
                _ => return Err(anyhow!("Unknown node type: {}", type_str)),
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_validate_valid_url_item() {
        let valid_url = json!({
            "type": "url",
            "name": "Google",
            "url": "https://google.com",
            "date_added": "13200000000000001",
            "guid": "550e8400-e29b-41d4-a716-446655440000",
            "id": "1001",
            "show_icon": false,
            "source": "sync",
            "visit_count": 10
        });

        assert!(validate_url_item(&valid_url).is_ok());
    }

    #[test]
    fn test_validate_invalid_url_item() {
        let invalid_url = json!({
            "type": "url",
            "name": "Google",
            "url": "not-a-url",
            "date_added": "invalid-date",
            "guid": "invalid-guid",
            "id": "1001"
        });

        assert!(validate_url_item(&invalid_url).is_err());
    }

    #[test]
    fn test_validate_valid_folder_item() {
        let valid_folder = json!({
            "type": "folder",
            "name": "Work",
            "date_added": "13200000000000000",
            "guid": "550e8400-e29b-41d4-a716-446655440001",
            "id": "1",
            "source": "sync",
            "children": []
        });

        assert!(validate_folder_item(&valid_folder).is_ok());
    }

    #[test]
    fn test_validate_invalid_folder_item() {
        let invalid_folder = json!({
            "type": "folder",
            "name": "Work",
            "date_added": "invalid-date",
            "guid": "invalid-guid",
            "id": "1"
        });

        assert!(validate_folder_item(&invalid_folder).is_err());
    }
}
use anyhow::Result;
use jsonschema::{Draft, JSONSchema};
use serde_json::Value;

pub fn validate_bookmarks_file(v: &Value) -> Result<()> {
    let schema_text = include_str!("../schemas/bookmarks_schema.json");
    let schema_json: Value = serde_json::from_str(schema_text)?;
    let compiled = JSONSchema::options()
        .with_draft(Draft::Draft7)
        .compile(&schema_json)?;
    if let Err(errors) = compiled.validate(v) {
        let msg = errors.map(|e| e.to_string()).collect::<Vec<_>>().join("; ");
        return Err(anyhow::anyhow!(
            "bookmarks schema validation failed: {}",
            msg
        ));
    }
    Ok(())
}

pub fn validate_all_bookmark_items(v: &Value) -> Result<()> {
    let folder_text = include_str!("../schemas/folder_schema.json");
    let url_text = include_str!("../schemas/url_schema.json");

    let folder_schema: Value = serde_json::from_str(folder_text)?;
    let url_schema: Value = serde_json::from_str(url_text)?;

    let compiled_folder = JSONSchema::options()
        .with_draft(Draft::Draft7)
        .compile(&folder_schema)?;
    let compiled_url = JSONSchema::options()
        .with_draft(Draft::Draft7)
        .compile(&url_schema)?;

    // Traverse roots and validate each node iteratively.
    if let Some(roots) = v.get("roots") {
        if let Some(map) = roots.as_object() {
            let mut stack: Vec<&Value> = map.values().collect();
            while let Some(node) = stack.pop() {
                if let Some(obj) = node.as_object() {
                    if let Some(t) = obj.get("type").and_then(|t| t.as_str()) {
                        match t {
                            "folder" => {
                                if let Err(errors) = compiled_folder.validate(node) {
                                    let msg = errors
                                        .map(|e| e.to_string())
                                        .collect::<Vec<_>>()
                                        .join("; ");
                                    return Err(anyhow::anyhow!(
                                        "folder schema validation failed: {}",
                                        msg
                                    ));
                                }
                                if let Some(children) =
                                    obj.get("children").and_then(|c| c.as_array())
                                {
                                    for child in children.iter() {
                                        stack.push(child);
                                    }
                                }
                            }
                            "url" => {
                                if let Err(errors) = compiled_url.validate(node) {
                                    let msg = errors
                                        .map(|e| e.to_string())
                                        .collect::<Vec<_>>()
                                        .join("; ");
                                    return Err(anyhow::anyhow!(
                                        "url schema validation failed: {}",
                                        msg
                                    ));
                                }
                            }
                            _ => {
                                // Unknown types are allowed for business rules; no schema validation
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(())
}
use anyhow::{anyhow, Result};
use jsonschema::{Draft, JSONSchema};
use once_cell::sync::Lazy;
use serde_json::Value;
use std::fs;

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
    let errors = BOOKMARKS_SCHEMA.validate(bookmarks);
    let error_list: Vec<String> = errors.map(|e| e.to_string()).collect();

    if !error_list.is_empty() {
        return Err(anyhow!(
            "Bookmarks file validation failed:\n{}",
            error_list.join("\n")
        ));
    }

    Ok(())
}

/// Validate a folder item against the folder schema
pub fn validate_folder_item(folder: &Value) -> Result<()> {
    let errors = FOLDER_SCHEMA.validate(folder);
    let error_list: Vec<String> = errors.map(|e| e.to_string()).collect();

    if !error_list.is_empty() {
        return Err(anyhow!(
            "Folder item validation failed:\n{}",
            error_list.join("\n")
        ));
    }

    Ok(())
}

/// Validate a URL item against the URL schema
pub fn validate_url_item(url_item: &Value) -> Result<()> {
    let errors = URL_SCHEMA.validate(url_item);
    let error_list: Vec<String> = errors.map(|e| e.to_string()).collect();

    if !error_list.is_empty() {
        return Err(anyhow!(
            "URL item validation failed:\n{}",
            error_list.join("\n")
        ));
    }

    Ok(())
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
                _ => return Err(anyhow!("Unknown node type: {}", type_str)),
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_validate_valid_url_item() {
        let valid_url = json!({
            "type": "url",
            "name": "Google",
            "url": "https://google.com",
            "date_added": "13200000000000001",
            "guid": "550e8400-e29b-41d4-a716-446655440000",
            "id": "1001",
            "show_icon": false,
            "source": "sync",
            "visit_count": 10
        });

        assert!(validate_url_item(&valid_url).is_ok());
    }

    #[test]
    fn test_validate_invalid_url_item() {
        let invalid_url = json!({
            "type": "url",
            "name": "Google",
            "url": "not-a-url",
            "date_added": "invalid-date",
            "guid": "invalid-guid",
            "id": "1001"
        });

        assert!(validate_url_item(&invalid_url).is_err());
    }

    #[test]
    fn test_validate_valid_folder_item() {
        let valid_folder = json!({
            "type": "folder",
            "name": "Work",
            "date_added": "13200000000000000",
            "guid": "550e8400-e29b-41d4-a716-446655440001",
            "id": "1",
            "source": "sync",
            "children": []
        });

        assert!(validate_folder_item(&valid_folder).is_ok());
    }

    #[test]
    fn test_validate_invalid_folder_item() {
        let invalid_folder = json!({
            "type": "folder",
            "name": "Work",
            "date_added": "invalid-date",
            "guid": "invalid-guid",
            "id": "1"
        });

        assert!(validate_folder_item(&invalid_folder).is_err());
    }
}
