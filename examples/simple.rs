extern crate kludgine;
use kludgine::prelude::*;

fn main() {
    SingleWindowApplication::<Simple>::default().run();
}

#[derive(Default)]
struct Simple {}

impl WindowCreator<Simple> for Simple {
    fn window_title() -> String {
        "Simple - Kludgine".to_owned()
    }
}

#[async_trait]
impl Window for Simple {
    async fn render_2d(&mut self, scene: &mut Scene) -> KludgineResult<()> {
        if scene.is_initial_frame() {
            let texture = Texture::load("examples/k.png")?;
            let sprite = SourceSprite::entire_texture(texture);
            scene.render_sprite_at(sprite, Point::zero());
        }
        Ok(())
    }
}
