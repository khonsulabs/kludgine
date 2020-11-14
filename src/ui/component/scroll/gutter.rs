use super::ScrollGutterColor;
use crate::{
    ui::{Component, ComponentBorder, Layout, StandaloneComponent, StyledContext},
    KludgineResult,
};
use async_trait::async_trait;

#[derive(Debug)]
pub(crate) struct Gutter;

#[async_trait]
impl Component for Gutter {
    async fn render_background(
        &self,
        context: &mut StyledContext,
        layout: &Layout,
    ) -> KludgineResult<()> {
        self.render_standard_background::<ScrollGutterColor, ComponentBorder>(context, layout)
            .await
    }
}

#[async_trait]
impl StandaloneComponent for Gutter {}
