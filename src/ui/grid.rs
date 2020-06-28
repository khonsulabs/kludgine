use super::{Component, ComponentEventStatus, Controller, EventStatus};
use crate::{
    math::{max_f, Point, Rect, Size},
    scene::SceneTarget,
    style::Style,
    KludgineError, KludgineResult,
};
use async_trait::async_trait;
use winit::event::MouseButton;

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
    pub fn component(&self) -> Option<Component> {
        if let GridEntry::Component(component) = self {
            Some(component.clone())
        } else {
            None
        }
    }
}

#[async_trait]
impl Controller for Grid {
    async fn mouse_button_down(
        &mut self,
        _component: &Component,
        button: MouseButton,
        window_position: Point,
    ) -> KludgineResult<ComponentEventStatus> {
        let mut handled = ComponentEventStatus::ignored();
        for child in self.cells.iter().filter_map(|e| e.component()) {
            if child.bounds().await.contains(window_position) {
                handled.update_with(child.mouse_button_down(button, window_position).await?);
            }
        }
        Ok(handled)
    }

    async fn mouse_button_up(
        &mut self,
        _component: &Component,
        button: MouseButton,
        window_position: Point,
    ) -> KludgineResult<ComponentEventStatus> {
        let mut handled = ComponentEventStatus::ignored();
        for child in self.cells.iter().filter_map(|e| e.component()) {
            if child.bounds().await.contains(window_position) {
                handled.update_with(child.mouse_button_up(button, window_position).await?);
            }
        }
        Ok(handled)
    }

    async fn render(
        &self,
        _component: &Component,
        scene: &mut SceneTarget<'_>,
    ) -> KludgineResult<()> {
        for cell in self.cells.iter() {
            if let Some(cell) = cell.component() {
                cell.render(scene).await?;
            }
        }
        Ok(())
    }

    async fn update_style(
        &mut self,
        component: &Component,
        scene: &mut SceneTarget<'_>,
        inherited_style: &Style,
    ) -> KludgineResult<EventStatus> {
        let current_style = component
            .compute_effective_style(inherited_style, scene)
            .await;

        // TODO Scene is limiting this from being something that can be parallelized
        for cell in self.cells.iter_mut().filter_map(|e| e.component()) {
            cell.update_style(scene, &current_style).await?;
        }

        Ok(EventStatus::Processed)
    }

    async fn content_size(
        &self,
        component: &Component,
        maximum_size: &Size,
        scene: &mut SceneTarget<'_>,
    ) -> KludgineResult<Size> {
        let (desired_size, ..) = self
            .calculate_desired_sizes(component, maximum_size, scene)
            .await?;
        Ok(desired_size)
    }
    async fn layout_within(
        &mut self,
        component: &Component,
        scene: &mut SceneTarget<'_>,
        _bounds: Rect,
    ) -> KludgineResult<()> {
        let bounds = component.bounds().await;
        // Use the new bounding box to compute our desired sizes
        let (desired_size, column_widths, row_heights) = self
            .calculate_desired_sizes(component, &bounds.size, scene)
            .await?;

        let empty_columns = column_widths.iter().filter(|w| *w <= &0.0).count();
        let empty_rows = row_heights.iter().filter(|h| *h <= &0.0).count();

        let extra_space_per_cell = (bounds.size - desired_size)
            / Size::new(
                (self.width - empty_columns as u32) as f32,
                (self.height() - empty_rows as u32) as f32,
            );

        let mut y_pos = bounds.origin.y;
        for y in 0..self.height() {
            let mut x_pos = bounds.origin.x;
            let row_height = row_heights[y as usize];
            if row_height > 0.0 {
                let row_height = row_height + extra_space_per_cell.height;
                for x in 0..self.width {
                    let column_width = column_widths[x as usize];
                    if column_width > 0.0 {
                        let column_width = column_width + extra_space_per_cell.width;
                        if let Some(cell) = self.cells[(y * self.width + x) as usize].component() {
                            let cell_bounds = Rect::sized(
                                Point::new(x_pos, y_pos),
                                Size::new(column_width, row_height),
                            );
                            cell.layout_within(scene, cell_bounds).await?;
                        }
                        x_pos += column_width;
                    }
                }
                y_pos += row_height;
            }
        }
        Ok(())
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

    async fn calculate_desired_sizes(
        &self,
        component: &Component,
        maximum_size: &Size,
        scene: &mut SceneTarget<'_>,
    ) -> KludgineResult<(Size<f32>, Vec<f32>, Vec<f32>)> {
        let inner_size = component
            .layout()
            .await
            .size_with_minimal_padding(&maximum_size);
        let mut column_widths = Vec::with_capacity(self.width as usize);
        for _ in 0..self.width {
            column_widths.push(0f32);
        }
        let mut row_heights = Vec::with_capacity(self.height() as usize);
        for _ in 0..self.height() {
            row_heights.push(0f32);
        }

        for y in 0..self.height() {
            for x in 0..self.width {
                if let Some(cell) = self.cells[(y * self.width + x) as usize].component() {
                    let cell_size = cell.content_size(&inner_size, scene).await?;
                    column_widths[x as usize] = max_f(column_widths[x as usize], cell_size.width);
                    row_heights[y as usize] = max_f(row_heights[y as usize], cell_size.height);
                }
            }
        }

        let desired_size = Size::new(column_widths.iter().sum(), row_heights.iter().sum());
        Ok((desired_size, column_widths, row_heights))
    }
}
