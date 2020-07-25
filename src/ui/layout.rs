mod absolute;
pub use self::absolute::*;
use crate::{
    math::{Rect, Size, Surround},
    ui::{Index, LayoutContext},
    KludgineResult,
};
use async_trait::async_trait;
use std::collections::HashMap;

#[async_trait]
pub trait LayoutSolver: Send + Sync + std::fmt::Debug {
    async fn layout_within(
        &self,
        bounds: &Rect,
        content_size: &Size,
        context: &mut LayoutContext,
    ) -> KludgineResult<HashMap<Index, Layout>>;
}

#[derive(Debug, Clone)]
pub struct Layout {
    pub bounds: Rect,
    pub padding: Surround,
}

impl Layout {
    pub fn none() -> NoLayout {
        NoLayout::default()
    }
    pub fn absolute() -> AbsoluteLayout {
        AbsoluteLayout::default()
    }

    pub fn bounds(&self) -> &'_ Rect {
        &self.bounds
    }

    pub fn inner_bounds(&self) -> Rect {
        self.bounds.inset(self.padding)
    }
}

pub trait LayoutSolverExt {
    fn layout(self) -> KludgineResult<Box<dyn LayoutSolver>>;
}

impl<T> LayoutSolverExt for T
where
    T: LayoutSolver + 'static,
{
    fn layout(self) -> KludgineResult<Box<dyn LayoutSolver>> {
        Ok(Box::new(self))
    }
}

#[derive(Debug, Default)]
pub struct NoLayout {}

#[async_trait]
impl LayoutSolver for NoLayout {
    async fn layout_within(
        &self,
        _bounds: &Rect,
        _content_size: &Size,
        _context: &mut LayoutContext,
    ) -> KludgineResult<HashMap<Index, Layout>> {
        Ok(HashMap::default())
    }
}
