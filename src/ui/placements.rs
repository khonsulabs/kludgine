use crate::{
    math::{Rect, Size},
    ui::{HierarchicalArena, Index, StyledContext},
    KludgineError, KludgineHandle, KludgineResult,
};
use std::collections::HashMap;

#[derive(Clone)]
pub struct Placements {
    measurements: KludgineHandle<HashMap<Index, Rect>>,
    arena: KludgineHandle<HierarchicalArena>,
}

impl Placements {
    pub(crate) fn new(arena: KludgineHandle<HierarchicalArena>) -> Self {
        Self {
            measurements: KludgineHandle::default(),
            arena,
        }
    }

    pub async fn measure<I: Into<Index>>(
        &self,
        index: I,
        max_size: &Size,
        context: &mut StyledContext,
    ) -> KludgineResult<Size> {
        let index = index.into();
        let arena = self.arena.read().await;
        let node = arena.get(index).ok_or(KludgineError::InvalidIndex)?;
        let mut context = context.clone_for(index);
        node.layout_within(&mut context, max_size, &self).await
    }

    async fn layout<I: Into<Index>>(
        &self,
        index: I,
        bounds: &Rect,
        context: &mut StyledContext,
    ) -> KludgineResult<Rect> {
        let index = index.into();
        let content_size = self.measure(index, &bounds.size, context).await?;

        let arena = self.arena.read().await;
        let node = arena.get(index).ok_or(KludgineError::InvalidIndex)?;

        Ok(node.layout().await.compute_bounds(&content_size, bounds))
    }

    pub async fn place<I: Into<Index>>(
        &self,
        index: I,
        bounds: &Rect,
        context: &mut StyledContext,
    ) -> KludgineResult<Rect> {
        let index = index.into();
        let relative_bounds = self.layout(index, bounds, context).await?;
        let parent = {
            let arena = self.arena.read().await;
            arena.parent(index)
        };
        let absolute_bounds = match parent {
            Some(parent) => {
                let parent_bounds = self.placement(parent).await.unwrap();
                Rect::sized(
                    parent_bounds.origin + relative_bounds.origin,
                    relative_bounds.size,
                )
            }
            None => relative_bounds,
        };

        let mut measurements = self.measurements.write().await;
        measurements.insert(index, absolute_bounds);

        Ok(relative_bounds)
    }

    pub async fn placement<I: Into<Index>>(&self, index: I) -> Option<Rect> {
        let measurements = self.measurements.read().await;
        measurements.get(&index.into()).copied()
    }
}
