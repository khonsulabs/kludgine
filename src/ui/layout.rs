use crate::{
    math::{Rect, Size},
    ui::{Context, HierarchicalArena, Index},
    KludgineError, KludgineHandle, KludgineResult,
};
use std::collections::HashMap;

#[derive(Clone)]
pub struct Layout {
    measurements: KludgineHandle<HashMap<Index, Rect>>,
    arena: KludgineHandle<HierarchicalArena>,
}

impl Layout {
    pub(crate) fn new(arena: KludgineHandle<HierarchicalArena>) -> Self {
        Self {
            measurements: KludgineHandle::default(),
            arena,
        }
    }

    pub async fn measure<I: Into<Index>>(&self, index: I, max_size: Size) -> KludgineResult<Size> {
        let index = index.into();
        let arena = self.arena.read().await;
        let node = arena.get(index).ok_or(KludgineError::InvalidIndex)?;
        let mut context = Context::new(index, self.arena.clone());
        node.component.content_size(&mut context, max_size).await
    }

    pub async fn place<I: Into<Index>>(&self, index: I, rect: Rect) {
        let mut measurements = self.measurements.write().await;
        measurements.insert(index.into(), rect);
    }

    pub async fn placement<I: Into<Index>>(&self, index: I) -> Option<Rect> {
        let measurements = self.measurements.read().await;
        measurements.get(&index.into()).copied()
    }
}
