use crate::{ui::Node, KludgineHandle};
use generational_arena::Arena;
use std::collections::VecDeque;
use std::collections::{HashMap, HashSet};

pub use generational_arena::Index;

#[derive(Default, Clone)]
pub(crate) struct HierarchicalArena {
    handle: KludgineHandle<HierarchicalArenaData>,
}

impl HierarchicalArena {
    pub async fn insert(&self, parent: Option<Index>, node: Node) -> Index {
        let mut arena = self.handle.write().await;
        arena.insert(parent, node)
    }

    pub async fn set_parent(&self, child: Index, parent: Option<Index>) {
        let mut arena = self.handle.write().await;
        arena.set_parent(child, parent)
    }

    pub async fn parent(&self, child: Index) -> Option<Index> {
        let arena = self.handle.read().await;
        arena.parent(child)
    }

    pub async fn children(&self, parent: &Option<Index>) -> HashSet<Index> {
        let arena = self.handle.read().await;
        arena.children(parent)
    }

    pub async fn get<I: Into<Index>>(&self, index: I) -> Option<Node> {
        let arena = self.handle.read().await;
        arena.get(index)
    }

    pub async fn traverse(&self) -> ArenaTraverser {
        let queue = self.children(&None).await.into_iter().collect();
        ArenaTraverser {
            handle: self.clone(),
            queue,
            processed: HashSet::new(),
            last: None,
        }
    }
}

#[derive(Clone)]
struct HierarchicalArenaData {
    arena: Arena<Node>,
    children_by_parent: HashMap<Option<Index>, HashSet<Index>>,
    parents: HashMap<Index, Option<Index>>,
}

impl Default for HierarchicalArenaData {
    fn default() -> Self {
        Self {
            arena: Arena::new(),
            children_by_parent: HashMap::new(),
            parents: HashMap::new(),
        }
    }
}

impl HierarchicalArenaData {
    pub fn insert(&mut self, parent: Option<Index>, node: Node) -> Index {
        let index = self.arena.insert(node);

        self.set_parent(index, parent);

        index
    }

    pub fn set_parent(&mut self, child: Index, parent: Option<Index>) {
        if let Some(old_parent) = self.parents.get(&child) {
            self.children_by_parent
                .get_mut(old_parent)
                .unwrap()
                .remove(&child);
        }

        self.children_by_parent
            .entry(parent)
            .and_modify(|children| {
                children.insert(child);
            })
            .or_insert_with(|| hash_set!(child));
    }

    pub fn parent(&self, child: Index) -> Option<Index> {
        self.parents.get(&child).copied().flatten()
    }

    pub fn children(&self, parent: &Option<Index>) -> HashSet<Index> {
        if let Some(children) = self.children_by_parent.get(parent) {
            children.clone()
        } else {
            HashSet::default()
        }
    }

    pub fn get<I: Into<Index>>(&self, index: I) -> Option<Node> {
        self.arena.get(index.into()).cloned()
    }
}
pub struct ArenaTraverser {
    handle: HierarchicalArena,
    queue: VecDeque<Index>,
    processed: HashSet<Index>,
    last: Option<Index>,
}

impl ArenaTraverser {
    pub async fn next(&mut self) -> Option<Index> {
        if let Some(last) = std::mem::take(&mut self.last) {
            let arena_handle = self.handle.clone();
            let arena = arena_handle.handle.read().await;
            self.queue.extend(arena.children(&Some(last)));
        }

        if let Some(index) = self.queue.pop_front() {
            if self.processed.contains(&index) {
                panic!("Cycle detected in hierarchy");
            }

            self.processed.insert(index);
            self.last = Some(index);
            Some(index)
        } else {
            None
        }
    }
}
