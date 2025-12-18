use serde_json::json;
use anyhow::Result;

use microsoft_edge_bookmark_sorter_flattener::infrastructure::schema_validator::{
    validate_folder_item, validate_url_item,
};

#[test]
fn validate_valid_url_item_integration() -> Result<()> {
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

    validate_url_item(&valid_url)?;
    Ok(())
}

#[test]
fn validate_invalid_url_item_integration() {
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
fn validate_valid_folder_item_integration() -> Result<()> {
    let valid_folder = json!({
        "type": "folder",
        "name": "Work",
        "date_added": "13200000000000000",
        "guid": "550e8400-e29b-41d4-a716-446655440001",
        "id": "1",
        "source": "sync",
        "children": []
    });

    validate_folder_item(&valid_folder)?;
    Ok(())
}

#[test]
fn validate_invalid_folder_item_integration() {
    let invalid_folder = json!({
        "type": "folder",
        "name": "Work",
        "date_added": "invalid-date",
        "guid": "invalid-guid",
        "id": "1"
    });

    assert!(validate_folder_item(&invalid_folder).is_err());
}
