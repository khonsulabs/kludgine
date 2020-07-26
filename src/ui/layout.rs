mod absolute;
pub use self::absolute::*;
use crate::{
    math::{Point, Rect, Size, Surround},
    ui::LayoutContext,
    KludgineResult,
};
use async_trait::async_trait;

#[async_trait]
pub trait LayoutSolver: Send + Sync + std::fmt::Debug {
    async fn layout_within(
        &self,
        bounds: &Rect,
        content_size: &Size,
        context: &mut LayoutContext,
    ) -> KludgineResult<()>;
}

#[derive(Debug, Clone, Default)]
pub struct Layout {
    pub bounds: Rect,
    pub padding: Surround,
    pub margin: Surround,
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

    pub fn bounds_without_margin(&self) -> Rect {
        self.bounds.inset(self.margin)
    }

    pub fn inner_bounds(&self) -> Rect {
        self.bounds_without_margin().inset(self.padding)
    }

    pub fn window_to_local(&self, location: Point) -> Point {
        location - self.bounds.origin
    }

    pub fn local_to_window(&self, location: Point) -> Point {
        location + self.bounds.origin
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
    ) -> KludgineResult<()> {
        Ok(())
    }
}
