use crate::domain::traits::UrlCanonicalizer;
use crate::infrastructure::serde_json_adapter::{BookmarkNodeDto, BookmarksFileDto};
use crate::usecase::normalize::arena::{Arena, Handle};
use std::collections::BTreeMap;

pub fn rebuild_dto_from_arena(
    mut base: BookmarksFileDto,
    mut arena: Arena,
    canonicalizer: &dyn UrlCanonicalizer,
) -> BookmarksFileDto {
    let mut built: Vec<Option<BookmarkNodeDto>> = vec![None; arena.nodes.len()];

    for root in arena.root_container.values().copied() {
        let order = postorder_handles(&arena, root);
        for h in order {
            if arena.nodes[h.0].deleted {
                continue;
            }

            let node = std::mem::take(&mut arena.nodes[h.0]);
            let mut kids: Vec<BookmarkNodeDto> = Vec::new();
            for ch in node.children.iter() {
                if let Some(k) = built[ch.0].take() {
                    kids.push(k);
                }
            }

            // Deterministic child order.
            kids.sort_by_cached_key(|a| sort_key(a, canonicalizer));

            let dto = BookmarkNodeDto {
                node_type: node.node_type.clone(),
                name: node.name.clone(),
                url: node.url.clone(),
                children: kids,
                date_added: node.date_added.clone(),
                date_modified: node.date_modified.clone(),
                date_last_used: node.date_last_used.clone(),
                visit_count: node.visit_count,
                guid: node.guid.clone(),
                id: node.id.clone(),
                source: node.source.clone(),
                show_icon: node.show_icon,
                // Preserve unknown extra fields from input nodes, but we will strip
                // internal merge metadata (e.g. `x_merge_meta`) from the final
                // output to remain Microsoft Edge compatible.
                extra: node.extra.clone(),
            };

            // Removed write_merge_meta to preserve original JSON structure
            built[h.0] = Some(dto);
        }
    }

    for (root_key, root_handle) in arena.root_container.iter() {
        if let Some(root_node) = built[root_handle.0].take() {
            base.roots.insert(root_key.clone(), root_node);
        }
    }

    // Remove any internal merge metadata added during processing before returning.
    base.extra.remove("x_merge_meta");

    base
}

fn postorder_handles(arena: &Arena, root: Handle) -> Vec<Handle> {
    let mut out: Vec<Handle> = Vec::new();
    let mut stack: Vec<(Handle, bool)> = vec![(root, false)];

    while let Some((h, expanded)) = stack.pop() {
        if arena.nodes[h.0].deleted {
            continue;
        }
        if expanded {
            out.push(h);
            continue;
        }
        stack.push((h, true));
        for ch in arena.nodes[h.0].children.iter().rev() {
            stack.push((*ch, false));
        }
    }

    out
}

fn sort_key(n: &BookmarkNodeDto, canonicalizer: &dyn UrlCanonicalizer) -> (u8, String, String) {
    match n.node_type.as_str() {
        "folder" => (
            0,
            n.name.clone().unwrap_or_default().trim().to_lowercase(),
            n.id.clone().unwrap_or_default(),
        ),
        "url" => (
            1,
            canonicalizer.canonicalize(n.url.as_deref().unwrap_or_default()),
            n.id.clone().unwrap_or_default(),
        ),
        other => (2, other.to_string(), n.name.clone().unwrap_or_default()),
    }
}
