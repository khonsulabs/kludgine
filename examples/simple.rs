extern crate kludgine;
use kludgine::prelude::*;

fn main() {
    SingleWindowApplication::<Simple>::default().run();
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
    async fn render_2d(&mut self, scene: &mut Scene) -> KludgineResult<()> {
        if self.source_sprite.is_none() {
            let texture = Texture::load("examples/k.png")?;
            self.source_sprite = Some(SourceSprite::entire_texture(texture));
        }

        scene.render_sprite_at(self.source_sprite.as_ref().unwrap(), Point::zero());

        Ok(())
    }
}
