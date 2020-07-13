extern crate kludgine;
use kludgine::prelude::*;

fn main() {
    SingleWindowApplication::run(Simple::default());
}

#[derive(Default)]
struct Simple {
    source_sprite: Option<SourceSprite>,
}

impl WindowCreator<Simple> for Simple {
    fn window_title() -> String {
        "Simple - Kludgine".to_owned()
    }
}

#[async_trait]
impl Window for Simple {
    async fn initialize(&mut self, _scene: &mut Scene) -> KludgineResult<()> {
        let texture = Texture::load("examples/assets/k.png")?;
        self.source_sprite = Some(SourceSprite::entire_texture(texture).await);
        Ok(())
    }
    async fn render<'a>(&self, scene: &SceneTarget) -> KludgineResult<()> {
        let sprite = self.source_sprite.as_ref().unwrap();

        sprite.render_at(scene, Point::default()).await;

        Ok(())
    }
}
