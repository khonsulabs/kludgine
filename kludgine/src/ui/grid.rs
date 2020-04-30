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
use async_trait::async_trait;
use futures::future::join_all;
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
    pub async fn view(&self) -> KludgineResult<Option<KludgineHandle<Box<dyn View>>>> {
        let view = match self {
            GridEntry::Component(component) => Some(component.view().await?),
            GridEntry::Empty => None,
        };

        Ok(view)
    }
}

#[async_trait]
impl Controller for Grid {
    async fn view(&self) -> KludgineResult<KludgineHandle<Box<dyn View>>> {
        let views: KludgineResult<Vec<_>> = join_all(self.cells.iter().map(|c| c.view()))
            .await
            .into_iter()
            .collect();

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

#[async_trait]
impl View for GridView {
    async fn render<'a>(&self, scene: &mut SceneTarget<'a>) -> KludgineResult<()> {
        for cell in self.cells.iter() {
            if let Some(cell) = cell {
                let view = cell.read().await;
                view.render(scene).await?
            }
        }
        Ok(())
    }

    async fn update_style<'a>(
        &mut self,
        scene: &mut SceneTarget<'a>,
        inherited_style: &Style,
    ) -> KludgineResult<()> {
        let current_style = self.compute_effective_style(inherited_style, scene);

        // TODO Scene is limiting this from being something that can be parallelized
        for cell in self.cells.iter_mut().filter_map(|e| e.as_ref().to_owned()) {
            let mut view = cell.write().await;
            view.update_style(scene, &current_style).await?;
        }

        Ok(())
    }

    async fn layout_within<'a>(
        &mut self,
        scene: &mut SceneTarget<'a>,
        bounds: Rect,
    ) -> KludgineResult<()> {
        // Let the base view handle padding
        self.view
            .layout_within(&self.content_size(&bounds.size, scene).await?, bounds)?;

        // Use the new bounding box to compute our desired sizes
        let (desired_size, column_widths, row_heights) = self
            .calculate_desired_sizes(&self.view.bounds.size, scene)
            .await?;

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
                            let mut view = cell.write().await;
                            view.layout_within(scene, cell_bounds).await?;
                        }
                        x_pos += column_width;
                    }
                }
                y_pos += row_height;
            }
        }
        Ok(())
    }

    async fn content_size<'a>(
        &self,
        maximum_size: &Size,
        scene: &mut SceneTarget<'a>,
    ) -> KludgineResult<Size> {
        let (desired_size, ..) = self.calculate_desired_sizes(maximum_size, scene).await?;
        Ok(desired_size)
    }

    async fn hovered_at(&mut self, window_position: Point) -> KludgineResult<()> {
        self.view.hovered_at(window_position)?;
        for cell in self.cells.iter_mut().filter_map(|f| f.as_ref()) {
            let mut view = cell.write().await;
            if view.bounds().contains(window_position) {
                view.hovered_at(window_position).await?;
            } else if view.base_view().mouse_status.is_some() {
                view.unhovered().await?;
            }
        }
        Ok(())
    }
    async fn unhovered(&mut self) -> KludgineResult<()> {
        self.view.unhovered()?;
        for cell in self.cells.iter_mut().filter_map(|f| f.as_ref()) {
            let mut view = cell.write().await;
            if view.base_view().mouse_status.is_some() {
                view.unhovered().await?;
                break;
            }
        }
        Ok(())
    }
}

impl GridView {
    pub fn new(width: u32, height: u32, cells: Vec<Option<KludgineHandle<Box<dyn View>>>>) -> Self {
        Self {
            view: BaseView::default(),
            width,
            height,
            cells,
        }
    }

    async fn calculate_desired_sizes<'a>(
        &self,
        maximum_size: &Size,
        scene: &mut SceneTarget<'a>,
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
                        let view = cell.read().await;
                        let cell_size = view.content_size(&inner_size, scene).await?;
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
