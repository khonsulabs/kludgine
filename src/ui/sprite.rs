use crate::{
    sprite::Sprite,
    ui::{Component, Context, LayoutConstraints},
    KludgineResult,
};
use async_trait::async_trait;

#[derive(Debug)]
pub struct Image {
    sprite: Sprite,
}

#[async_trait]
impl Component for Image {
    type Message = ();

    async fn render(&self, context: &mut Context) -> KludgineResult<()> {
        todo!()
    }

    async fn layout(&mut self, context: &mut Context) -> KludgineResult<LayoutConstraints> {
        todo!()
    }
}
