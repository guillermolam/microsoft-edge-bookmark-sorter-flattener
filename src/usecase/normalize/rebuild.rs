use crate::domain::traits::UrlCanonicalizer;
use crate::infrastructure::serde_json_adapter::{BookmarkNodeDto, BookmarksFileDto};
use crate::usecase::normalize::arena::{Arena, Handle};
use serde_json::json;

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

            let mut node = std::mem::take(&mut arena.nodes[h.0]);
            let mut kids: Vec<BookmarkNodeDto> = Vec::new();
            for ch in node.children.iter() {
                if let Some(k) = built[ch.0].take() {
                    kids.push(k);
                }
            }

            // Deterministic child order.
            kids.sort_by_cached_key(|a| sort_key(a, canonicalizer));

            let extra = std::mem::take(&mut node.extra);

            let mut dto = BookmarkNodeDto {
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
                extra,
            };

            write_merge_meta(&mut dto, &mut node);
            built[h.0] = Some(dto);
        }
    }

    for (root_key, root_handle) in arena.root_container.iter() {
        if let Some(root_node) = built[root_handle.0].take() {
            base.roots.insert(root_key.clone(), root_node);
        }
    }

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

fn write_merge_meta(
    dto: &mut BookmarkNodeDto,
    node: &mut crate::usecase::normalize::arena::ArenaNode,
) {
    let mut merged_names = std::mem::take(&mut node.merged_names);
    let mut merged_ids = std::mem::take(&mut node.merged_ids);
    let mut merged_guids = std::mem::take(&mut node.merged_guids);
    let mut merged_paths = std::mem::take(&mut node.merged_paths);
    let merged_from = std::mem::take(&mut node.merged_from);

    merged_names.sort();
    merged_names.dedup();
    merged_ids.sort();
    merged_ids.dedup();
    merged_guids.sort();
    merged_guids.dedup();
    merged_paths.sort();
    merged_paths.dedup();

    if merged_names.is_empty()
        && merged_ids.is_empty()
        && merged_guids.is_empty()
        && merged_paths.is_empty()
        && merged_from.is_empty()
    {
        return;
    }

    dto.extra.insert(
        "x_merge_meta".to_string(),
        json!({
            "merged_names": merged_names,
            "merged_ids": merged_ids,
            "merged_guids": merged_guids,
            "merged_paths": merged_paths,
            "merged_from": merged_from,
        }),
    );
}
