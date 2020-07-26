use crate::{
    math::Size,
    shape::{Fill, Shape},
    ui::{Context, Layout, LayoutSolver, LayoutSolverExt, SceneContext, StyledContext},
    window::InputEvent,
    KludgineResult,
};
use async_trait::async_trait;

pub struct LayoutConstraints {}

#[async_trait]
pub(crate) trait BaseComponent: Send + Sync {
    /// Called once the Window is opened
    async fn initialize(&mut self, context: &mut Context) -> KludgineResult<()>;

    async fn update(&mut self, context: &mut SceneContext) -> KludgineResult<()>;

    async fn process_input(
        &mut self,
        context: &mut Context,
        event: InputEvent,
    ) -> KludgineResult<()>;

    async fn content_size(
        &self,
        context: &mut StyledContext,
        constraints: &Size<Option<f32>>,
    ) -> KludgineResult<Size>;

    async fn layout(
        &mut self,
        context: &mut StyledContext,
    ) -> KludgineResult<Box<dyn LayoutSolver>>;

    async fn render(&self, context: &mut StyledContext, layout: &Layout) -> KludgineResult<()>;
    async fn render_background(
        &self,
        context: &mut StyledContext,
        layout: &Layout,
    ) -> KludgineResult<()>;
}

#[async_trait]
#[allow(unused_variables)]
pub trait Component: Send + Sync {
    type Message: Send + Sync + std::fmt::Debug;

    /// Called once the Window is opened
    async fn initialize(&mut self, context: &mut Context) -> KludgineResult<()> {
        Ok(())
    }

    async fn receive_message(
        &mut self,
        context: &mut Context,
        message: Self::Message,
    ) -> KludgineResult<()> {
        unimplemented!(
            "Component::receive_message() must be implemented if you're receiving messages"
        )
    }

    async fn content_size(
        &self,
        context: &mut StyledContext,
        constraints: &Size<Option<f32>>,
    ) -> KludgineResult<Size> {
        Ok(Size {
            width: constraints.width.unwrap_or_default(),
            height: constraints.height.unwrap_or_default(),
        })
    }

    async fn render(&self, context: &mut StyledContext, layout: &Layout) -> KludgineResult<()>;

    async fn update(&mut self, context: &mut SceneContext) -> KludgineResult<()> {
        Ok(())
    }

    async fn layout(
        &mut self,
        context: &mut StyledContext,
    ) -> KludgineResult<Box<dyn LayoutSolver>> {
        Layout::none().layout()
    }

    async fn process_input(
        &mut self,
        context: &mut Context,
        event: InputEvent,
    ) -> KludgineResult<()> {
        Ok(())
    }

    async fn render_background(
        &self,
        context: &mut StyledContext,
        layout: &Layout,
    ) -> KludgineResult<()> {
        if let Some(background) = context.effective_style().background_color {
            context
                .scene()
                .draw_shape(
                    Shape::rect(
                        layout.bounds_without_margin().coord1(),
                        layout.bounds_without_margin().coord2(),
                    )
                    .fill(Fill::Solid(background)),
                )
                .await;
        }
        Ok(())
    }
}
