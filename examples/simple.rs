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

impl Window for Simple {}

#[async_trait]
impl Component for Simple {
    type Message = ();

    async fn initialize(&mut self, _context: &mut Context) -> KludgineResult<()> {
        let texture = Texture::load("examples/assets/k.png")?;
        self.source_sprite = Some(SourceSprite::entire_texture(texture).await);
        Ok(())
    }

    async fn render(
        &self,
        _context: &mut Context,
        scene: &SceneTarget,
        _location: Rect,
    ) -> KludgineResult<()> {
        let sprite = self.source_sprite.as_ref().unwrap();

        sprite.render_at(scene, Point::default()).await;

        Ok(())
    }
}
