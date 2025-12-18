use std::cmp::Ordering;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NodeKind {
    Folder,
    Url,
    Other(String),
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct NodeId(pub String);

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct NodeMeta {
    pub guid: Option<String>,
    pub id: Option<String>,
    pub date_added: Option<String>,
    pub date_modified: Option<String>,
    pub date_last_used: Option<String>,
    pub visit_count: Option<i64>,
}

#[derive(Debug, Clone)]
pub struct BookmarkNode {
    pub kind: NodeKind,
    pub name: Option<String>,
    pub url: Option<String>,
    pub identity: Option<NodeId>,
    pub meta: NodeMeta,
}

impl BookmarkNode {
    pub fn normalized_folder_name(&self) -> Option<String> {
        let name = self.name.as_ref()?;
        Some(name.trim().to_lowercase())
    }

    pub fn compare_outermost_winner(&self, other: &Self) -> Ordering {
        // This comparator is only meaningful when the caller provides depth / path ordering.
        // Kept here as a placeholder for domain-level tie-break rules; usecase supplies depth.
        self.meta
            .date_added
            .cmp(&other.meta.date_added)
            .then_with(|| self.meta.id.cmp(&other.meta.id))
            .then_with(|| self.meta.guid.cmp(&other.meta.guid))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalized_folder_name_trims_and_lowercases() {
        let node = BookmarkNode {
            kind: NodeKind::Folder,
            name: Some("  Foo BAR  ".to_string()),
            url: None,
            identity: None,
            meta: NodeMeta::default(),
        };

        assert_eq!(node.normalized_folder_name().as_deref(), Some("foo bar"));

        let node = BookmarkNode {
            name: None,
            ..node
        };
        assert_eq!(node.normalized_folder_name(), None);
    }

    #[test]
    fn compare_outermost_winner_uses_date_added_then_id_then_guid() {
        let base = BookmarkNode {
            kind: NodeKind::Folder,
            name: Some("x".to_string()),
            url: None,
            identity: None,
            meta: NodeMeta::default(),
        };

        let a = BookmarkNode {
            meta: NodeMeta {
                date_added: Some("1".to_string()),
                id: Some("2".to_string()),
                guid: Some("b".to_string()),
                ..NodeMeta::default()
            },
            ..base.clone()
        };

        let b = BookmarkNode {
            meta: NodeMeta {
                date_added: Some("2".to_string()),
                id: Some("1".to_string()),
                guid: Some("a".to_string()),
                ..NodeMeta::default()
            },
            ..base.clone()
        };

        assert_eq!(a.compare_outermost_winner(&b), Ordering::Less);
        assert_eq!(b.compare_outermost_winner(&a), Ordering::Greater);

        let c = BookmarkNode {
            meta: NodeMeta {
                date_added: Some("2".to_string()),
                id: Some("2".to_string()),
                guid: Some("a".to_string()),
                ..NodeMeta::default()
            },
            ..base.clone()
        };
        let d = BookmarkNode {
            meta: NodeMeta {
                date_added: Some("2".to_string()),
                id: Some("3".to_string()),
                guid: Some("a".to_string()),
                ..NodeMeta::default()
            },
            ..base
        };

        assert_eq!(c.compare_outermost_winner(&d), Ordering::Less);

        let e = BookmarkNode {
            meta: NodeMeta {
                date_added: Some("2".to_string()),
                id: Some("3".to_string()),
                guid: Some("a".to_string()),
                ..NodeMeta::default()
            },
            ..d.clone()
        };
        let f = BookmarkNode {
            meta: NodeMeta {
                date_added: Some("2".to_string()),
                id: Some("3".to_string()),
                guid: Some("b".to_string()),
                ..NodeMeta::default()
            },
            ..d
        };

        assert_eq!(e.compare_outermost_winner(&f), Ordering::Less);
    }
}
