use super::{
    view::{BaseView, View, ViewBuilder, ViewCore},
    Component, Controller,
};
use crate::{
    math::{max_f, Point, Rect, Size},
    scene::SceneTarget,
    style::Style,
    KludgineError, KludgineResult,
};
use kludgine_macros::ViewCore;
use std::sync::Arc;

#[derive(Debug)]
pub struct Grid {
    width: u32,
    cells: Vec<GridEntry>,
}

#[derive(Debug)]
pub enum GridEntry {
    Empty,
    Component(Component),
}

impl GridEntry {
    pub fn view(&self) -> KludgineResult<Option<Arc<Box<dyn View>>>> {
        let view = match self {
            GridEntry::Component(component) => Some(Arc::new(component.view()?)),
            GridEntry::Empty => None,
        };

        Ok(view)
    }
}

impl Controller for Grid {
    fn view(&self) -> KludgineResult<Box<dyn View>> {
        let views: KludgineResult<Vec<_>> = self.cells.iter().map(|c| c.view()).collect();

        GridView::new(self.width, self.height(), views?).build()
    }
}

impl Grid {
    pub fn new(initial_width: u32, initial_height: u32) -> Self {
        let total_cells = initial_width * initial_height;
        let mut cells = Vec::with_capacity(total_cells as usize);
        for _ in 0..total_cells {
            cells.push(GridEntry::Empty);
        }
        Grid {
            width: initial_width,
            cells,
        }
    }

    pub fn set_cell(&mut self, location: Point<u32>, component: Component) -> KludgineResult<()> {
        if self.width >= location.x || self.height() >= location.y {
            return Err(KludgineError::OutOfBounds);
        }

        self.cells[(location.y * self.width + location.x) as usize] =
            GridEntry::Component(component);

        Ok(())
    }

    pub fn with_cell(
        &mut self,
        location: Point<u32>,
        component: Component,
    ) -> KludgineResult<&mut Self> {
        self.set_cell(location, component)?;
        Ok(self)
    }

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.cells.len() as u32 / self.width
    }
}

#[derive(ViewCore, Debug, Default, Clone)]
pub struct GridView {
    view: BaseView,
    width: u32,
    height: u32,
    cells: Vec<Option<Arc<Box<dyn View>>>>,
}

impl View for GridView {
    fn render(&self, scene: &mut SceneTarget) -> KludgineResult<()> {
        for cell in self.cells.iter() {
            if let Some(cell) = cell {
                cell.render(scene)?
            }
        }
        Ok(())
    }

    fn update_style(
        &mut self,
        scene: &mut SceneTarget,
        inherited_style: &Style,
    ) -> KludgineResult<()> {
        let inherited_style = self.view.style.inherit_from(&inherited_style);
        self.view.effective_style = inherited_style.effective_style(scene);
        Ok(())
    }

    fn layout_within(&mut self, scene: &mut SceneTarget, bounds: Rect) -> KludgineResult<()> {
        self.view
            .layout_within(&self.content_size(&bounds.size, scene)?, bounds)
    }

    fn content_size(&self, maximum_size: &Size, scene: &mut SceneTarget) -> KludgineResult<Size> {
        let inner_size = &self.view.layout.size_with_minimal_padding(&maximum_size);
        let mut column_widths = Vec::with_capacity(self.width as usize);
        for _ in 0..self.width {
            column_widths.push(0f32);
        }
        let mut row_heights = Vec::with_capacity(self.height as usize);
        for _ in 0..self.height {
            row_heights.push(0f32);
        }

        for y in 0..self.height {
            for x in 0..self.width {
                match self.cells.get((y * self.width + x) as usize).unwrap() {
                    Some(cell) => {
                        let cell_size = cell.content_size(&inner_size, scene)?;
                        column_widths[x as usize] =
                            max_f(column_widths[x as usize], cell_size.width);
                        row_heights[y as usize] = max_f(row_heights[y as usize], cell_size.height);
                    }
                    None => {}
                }
            }
        }

        Ok(Size::new(
            column_widths.iter().sum(),
            row_heights.iter().sum(),
        ))
    }
}

impl GridView {
    pub fn new(width: u32, height: u32, cells: Vec<Option<Arc<Box<dyn View>>>>) -> Self {
        Self {
            view: BaseView::default(),
            width,
            height,
            cells,
        }
    }
}
