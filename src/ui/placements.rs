use crate::{
    math::{Rect, Size},
    ui::{HierarchicalArena, Index, StyledContext},
    KludgineError, KludgineHandle, KludgineResult,
};
use std::collections::HashMap;

#[derive(Clone)]
pub struct Placements {
    measurements: KludgineHandle<HashMap<Index, Rect>>,
    arena: HierarchicalArena,
}

impl Placements {
    pub(crate) fn new(arena: HierarchicalArena) -> Self {
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
        let node = self
            .arena
            .get(index)
            .await
            .ok_or(KludgineError::InvalidIndex)?;
        let layout = node.layout().await;

        let mut context = context.clone_for(index);
        let desired_size = node
            .layout_within(
                &mut context,
                &layout.interior_size_with_padding(max_size),
                &self,
            )
            .await?;
        Ok(desired_size + layout.padding.minimum_size())
    }

    pub async fn place<I: Into<Index>>(
        &self,
        index: I,
        bounds: &Rect,
        context: &mut StyledContext,
    ) -> KludgineResult<Rect> {
        let index = index.into();
        // let padding = self.layout(index, bounds, context).await?;
        let relative_bounds = bounds; // TODO take measurements from layout for x/y
        let parent = self.arena.parent(index).await;
        let absolute_bounds = match parent {
            Some(parent) => {
                let parent_bounds = self.placement(parent).await.unwrap();
                Rect::sized(
                    parent_bounds.origin + relative_bounds.origin,
                    relative_bounds.size,
                )
            }
            None => *relative_bounds,
        };

        let mut measurements = self.measurements.write().await;
        measurements.insert(index, absolute_bounds);

        Ok(*relative_bounds)
    }

    pub async fn placement<I: Into<Index>>(&self, index: I) -> Option<Rect> {
        let measurements = self.measurements.read().await;
        measurements.get(&index.into()).copied()
    }
}
