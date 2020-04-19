extern crate kludgine;
use kludgine::prelude::*;

fn main() {
    SingleWindowApplication::<Isometric>::default().run();
}

#[derive(Default)]
struct Isometric {
    source_sprite: Option<SourceSprite>,
}

impl WindowCreator<Isometric> for Isometric {
    fn window_title() -> String {
        "Isometric - Kludgine".to_owned()
    }
}

#[async_trait]
impl Window for Isometric {
    async fn render(&mut self, scene: &mut Scene) -> KludgineResult<()> {
        if self.source_sprite.is_none() {
            let texture = Texture::load("examples/isometric_title.png")?;
            self.source_sprite = Some(SourceSprite::entire_texture(&texture));
        }

        scene.render_sprite_at(self.source_sprite.as_ref().unwrap(), Point::zero());

        Ok(())
    }
}
