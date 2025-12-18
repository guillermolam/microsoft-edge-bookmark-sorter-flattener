use crate::infrastructure::serde_json_adapter::{BookmarkNodeDto, BookmarksFileDto};
use crate::usecase::normalize::arena::{Arena, ArenaNode, Handle};
use crate::usecase::stats::NormalizeStats;

pub fn build_arena_from_dto(input: &BookmarksFileDto, stats: &mut NormalizeStats) -> Arena {
    let mut arena = Arena::default();

    // Stable iteration over roots (BTreeMap).
    for (root_key, root_node) in input.roots.iter() {
        // Root container nodes are special: we allocate the container itself here,
        // and then allocate its immediate children separately so they start in merge space.
        // Prevent `alloc_node` from also expanding `root_node.children`, which would
        // duplicate the entire subtree and create unreachable nodes.
        let mut root_shallow = root_node.clone();
        root_shallow.children.clear();
        // Ensure root containers are treated as folders for processing
        root_shallow.node_type = "folder".to_string();

        let container = alloc_node(
            &mut arena,
            None,
            root_key.clone(),
            root_key.clone(),
            0,
            &root_shallow,
        );
        arena.root_container.insert(root_key.clone(), container);

        // Child folders under root containers are in merge space.
        let mut child_handles = Vec::with_capacity(root_node.children.len());
        for (i, child) in root_node.children.iter().enumerate() {
            let child_path = format!("{root_key}/{i}");
            let h = alloc_node(
                &mut arena,
                Some(container),
                root_key.clone(),
                child_path,
                1,
                child,
            );
            child_handles.push(h);
        }
        arena.nodes[container.0].children = child_handles;
    }

    for node in arena.nodes.iter() {
        if node.node_type == "folder" {
            stats.folders_seen += 1;
        } else if node.node_type == "url" {
            stats.urls_seen += 1;
        }
    }

    arena
}

fn alloc_node(
    arena: &mut Arena,
    parent: Option<Handle>,
    root_key: String,
    path: String,
    depth: usize,
    dto: &BookmarkNodeDto,
) -> Handle {
    let handle = Handle(arena.nodes.len());

    arena.nodes.push(ArenaNode {
        node_type: dto.node_type.clone(),
        name: dto.name.clone(),
        url: dto.url.clone(),
        children: Vec::new(),
        date_added: dto.date_added.clone(),
        date_modified: dto.date_modified.clone(),
        date_last_used: dto.date_last_used.clone(),
        visit_count: dto.visit_count,
        guid: dto.guid.clone(),
        id: dto.id.clone(),
        source: dto.source.clone(),
        show_icon: dto.show_icon,
        extra: dto.extra.clone(),
        root_key: Some(root_key),
        path,
        depth,
        deleted: false,
    });
    arena.parent.push(parent);

    if dto.children.is_empty() {
        return handle;
    }

    // Iterative expansion (no recursion).
    type BuildStackItem = (Handle, Vec<(usize, BookmarkNodeDto)>, String, usize);
    let mut stack: Vec<BuildStackItem> = Vec::new();
    stack.push((
        handle,
        dto.children.iter().cloned().enumerate().collect(),
        arena.nodes[handle.0].path.clone(),
        depth,
    ));

    while let Some((parent_h, mut kids, parent_path, parent_depth)) = stack.pop() {
        if kids.is_empty() {
            continue;
        }
        kids.reverse();

        let mut built: Vec<Handle> = Vec::with_capacity(kids.len());
        for (idx, kid) in kids {
            let kid_path = format!("{parent_path}/{idx}");
            let kid_h = Handle(arena.nodes.len());

            arena.nodes.push(ArenaNode {
                node_type: kid.node_type.clone(),
                name: kid.name.clone(),
                url: kid.url.clone(),
                children: Vec::new(),
                date_added: kid.date_added.clone(),
                date_modified: kid.date_modified.clone(),
                date_last_used: kid.date_last_used.clone(),
                visit_count: kid.visit_count,
                guid: kid.guid.clone(),
                id: kid.id.clone(),
                source: kid.source.clone(),
                show_icon: kid.show_icon,
                extra: kid.extra.clone(),
                root_key: arena.nodes[parent_h.0].root_key.clone(),
                path: kid_path.clone(),
                depth: parent_depth + 1,
                deleted: false,
            });
            arena.parent.push(Some(parent_h));

            if !kid.children.is_empty() {
                stack.push((
                    kid_h,
                    kid.children.iter().cloned().enumerate().collect(),
                    kid_path,
                    parent_depth + 1,
                ));
            }

            built.push(kid_h);
        }

        built.reverse();
        arena.nodes[parent_h.0].children = built;
    }

    handle
}
