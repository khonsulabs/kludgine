extern crate kludgine;
use kludgine::prelude::*;

fn main() {
    SingleWindowApplication::run(Simple::default());
}

#[derive(Default)]
struct Simple {
    source_sprite: Option<SpriteSource>,
    rotation_angle: Angle,
}

impl WindowCreator for Simple {
    fn window_title() -> String {
        "Simple - Kludgine".to_owned()
    }
}

#[async_trait]
impl Window for Simple {
    fn target_fps(&self) -> Option<u16> {
        Some(60)
    }

    async fn update(&mut self, scene: &Target) -> KludgineResult<()> {
        if self.source_sprite.is_none() {
            let texture = Texture::load("examples/assets/k.png")?;
            self.source_sprite = Some(SpriteSource::entire_texture(texture));
        }

        if let Some(elapsed) = scene.elapsed().await {
            self.rotation_angle += Angle::radians(elapsed.as_secs_f32());
        }

        Ok(())
    }

    async fn render(&mut self, scene: &Target) -> KludgineResult<()> {
        let sprite = self.source_sprite.as_ref().unwrap();

        let bounds = Rect::new(Point::default(), scene.size().await);

        sprite
            .render_at(
                scene,
                bounds.center(),
                SpriteRotation::around_center(self.rotation_angle),
            )
            .await;

        Ok(())
    }
}
