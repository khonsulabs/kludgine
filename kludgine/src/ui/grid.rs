use super::{
    view::{BaseView, View, ViewBuilder, ViewCore, ViewCoreBuilder},
    Component, Controller,
};
use crate::{
    math::{max_f, Point, Rect, Size, Surround},
    scene::SceneTarget,
    style::Style,
    KludgineError, KludgineHandle, KludgineResult,
};
use kludgine_macros::ViewCore;

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
    pub fn view(&self) -> KludgineResult<Option<Box<dyn View>>> {
        let view = match self {
            GridEntry::Component(component) => Some(component.view()?),
            GridEntry::Empty => None,
        };

        Ok(view)
    }
}

impl Controller for Grid {
    fn view(&self) -> KludgineResult<Box<dyn View>> {
        let views: KludgineResult<Vec<_>> = self.cells.iter().map(|c| c.view()).collect();

        GridView::new(self.width, self.height(), views?)
            .with_padding(Surround::uniform(0.0))
            .build()
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
        if self.width <= location.x || self.height() <= location.y {
            return Err(KludgineError::OutOfBounds);
        }

        self.cells[(location.y * self.width + location.x) as usize] =
            GridEntry::Component(component);

        Ok(())
    }

    pub fn with_cell(mut self, location: Point<u32>, component: Component) -> KludgineResult<Self> {
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
    cells: Vec<Option<KludgineHandle<Box<dyn View>>>>,
}

impl View for GridView {
    fn render(&self, scene: &mut SceneTarget) -> KludgineResult<()> {
        for cell in self.cells.iter() {
            if let Some(cell) = cell {
                let view = cell.read().expect("Error locking view for render");
                view.render(scene)?
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

        for cell in self.cells.iter_mut().filter_map(|e| e.as_ref().to_owned()) {
            let mut view = cell.write().expect("Error locking view for update_style");
            view.update_style(scene, &inherited_style)?;
        }

        Ok(())
    }

    fn layout_within(&mut self, scene: &mut SceneTarget, bounds: Rect) -> KludgineResult<()> {
        // Let the base view handle padding
        self.view
            .layout_within(&self.content_size(&bounds.size, scene)?, bounds)?;

        // Use the new bounding box to compute our desired sizes
        let (desired_size, column_widths, row_heights) =
            self.calculate_desired_sizes(&self.view.bounds.size, scene)?;

        let empty_columns = column_widths.iter().filter(|w| *w <= &0.0).count();
        let empty_rows = row_heights.iter().filter(|h| *h <= &0.0).count();

        let extra_space_per_cell = (self.view.bounds.size - desired_size)
            / Size::new(
                (self.width - empty_columns as u32) as f32,
                (self.height - empty_rows as u32) as f32,
            );

        let mut y_pos = self.view.bounds.origin.y;
        for y in 0..self.height {
            let mut x_pos = self.view.bounds.origin.x;
            let row_height = row_heights[y as usize];
            if row_height > 0.0 {
                let row_height = row_height + extra_space_per_cell.height;
                for x in 0..self.width {
                    let column_width = column_widths[x as usize];
                    if column_width > 0.0 {
                        let column_width = column_width + extra_space_per_cell.width;
                        if let Some(cell) =
                            self.cells.get_mut((y * self.width + x) as usize).unwrap()
                        {
                            let cell_bounds = Rect::sized(
                                Point::new(x_pos, y_pos),
                                Size::new(column_width, row_height),
                            );
                            let mut view =
                                cell.write().expect("Error locking view for layout_within");
                            view.layout_within(scene, cell_bounds)?;
                        }
                        x_pos += column_width;
                    }
                }
                y_pos += row_height;
            }
        }
        Ok(())
    }

    fn content_size(&self, maximum_size: &Size, scene: &mut SceneTarget) -> KludgineResult<Size> {
        let (desired_size, ..) = self.calculate_desired_sizes(maximum_size, scene)?;
        Ok(desired_size)
    }
}

impl GridView {
    pub fn new(width: u32, height: u32, cells: Vec<Option<Box<dyn View>>>) -> Self {
        Self {
            view: BaseView::default(),
            width,
            height,
            cells: cells
                .into_iter()
                .map(|v| v.map(|view| KludgineHandle::new(view)))
                .collect(),
        }
    }

    fn calculate_desired_sizes(
        &self,
        maximum_size: &Size,
        scene: &mut SceneTarget,
    ) -> KludgineResult<(Size<f32>, Vec<f32>, Vec<f32>)> {
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
                        let view = cell.read().expect("Error locking view for render");
                        let cell_size = view.content_size(&inner_size, scene)?;
                        column_widths[x as usize] =
                            max_f(column_widths[x as usize], cell_size.width);
                        row_heights[y as usize] = max_f(row_heights[y as usize], cell_size.height);
                    }
                    None => {}
                }
            }
        }

        let desired_size = Size::new(column_widths.iter().sum(), row_heights.iter().sum());
        Ok((desired_size, column_widths, row_heights))
    }
}
