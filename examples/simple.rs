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
    async fn render<'a>(&mut self, scene: &mut SceneTarget<'a>) -> KludgineResult<()> {
        if self.source_sprite.is_none() {
            let texture = Texture::load("examples/assets/k.png")?;
            self.source_sprite = Some(SourceSprite::entire_texture(texture).await);
        }
        let sprite = self.source_sprite.as_ref().unwrap();

        sprite.render_at(scene, Point::default()).await;

        Ok(())
    }
}
