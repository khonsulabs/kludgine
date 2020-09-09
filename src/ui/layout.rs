mod absolute;
pub use self::absolute::*;
use crate::{
    math::{Point, Rect, Scaled, Size, Surround},
    ui::LayoutContext,
    KludgineResult,
};
use async_trait::async_trait;

#[async_trait]
pub trait LayoutSolver: Send + Sync + std::fmt::Debug {
    async fn layout_within(
        &self,
        bounds: &Rect<f32, Scaled>,
        content_size: &Size<f32, Scaled>,
        context: &mut LayoutContext,
    ) -> KludgineResult<()>;
}

#[derive(Debug, Clone, Default)]
pub struct Layout {
    pub bounds: Rect<f32, Scaled>,
    pub padding: Surround<f32, Scaled>,
    pub margin: Surround<f32, Scaled>,
}

impl Layout {
    pub fn none() -> NoLayout {
        NoLayout::default()
    }
    pub fn absolute() -> AbsoluteLayout {
        AbsoluteLayout::default()
    }

    pub fn bounds(&self) -> &'_ Rect<f32, Scaled> {
        &self.bounds
    }

    pub fn bounds_without_margin(&self) -> Rect<f32, Scaled> {
        self.margin.inset_rect(&self.bounds)
    }

    pub fn inner_bounds(&self) -> Rect<f32, Scaled> {
        self.padding.inset_rect(&self.bounds_without_margin())
    }

    pub fn window_to_local(&self, location: Point<f32, Scaled>) -> Point<f32, Scaled> {
        location - self.bounds.origin.to_vector()
    }

    pub fn local_to_window(&self, location: Point<f32, Scaled>) -> Point<f32, Scaled> {
        location + self.bounds.origin.to_vector()
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
        _bounds: &Rect<f32, Scaled>,
        _content_size: &Size<f32, Scaled>,
        _context: &mut LayoutContext,
    ) -> KludgineResult<()> {
        Ok(())
    }
}
