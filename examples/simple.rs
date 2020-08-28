extern crate kludgine;
use kludgine::prelude::*;

fn main() {
    SingleWindowApplication::run(Simple::default());
}

#[derive(Default)]
struct Simple {
    source_sprite: Option<SpriteSource>,
}

impl WindowCreator<Simple> for Simple {
    fn window_title() -> String {
        "Simple - Kludgine".to_owned()
    }
}

impl Window for Simple {}

impl StandaloneComponent for Simple {}

#[async_trait]
impl Component for Simple {
    async fn initialize(&mut self, _context: &mut SceneContext) -> KludgineResult<()> {
        let texture = Texture::load("examples/assets/k.png")?;
        self.source_sprite = Some(SpriteSource::entire_texture(texture).await);
        Ok(())
    }

    async fn render(&self, context: &mut StyledContext, _layout: &Layout) -> KludgineResult<()> {
        let sprite = self.source_sprite.as_ref().unwrap();

        sprite.render_at(context.scene(), Point::default()).await;

        Ok(())
    }
}
