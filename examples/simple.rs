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

impl Window for Simple {
    fn target_fps(&self) -> Option<u16> {
        Some(60)
    }

    fn initialize(&mut self, _scene: &Target) -> KludgineResult<()> {
        let texture = Texture::load("examples/assets/k.png")?;
        self.source_sprite = Some(SpriteSource::entire_texture(texture));
        Ok(())
    }

    fn update(&mut self, scene: &Target, _status: &mut RedrawStatus) -> KludgineResult<()> {
        if let Some(elapsed) = scene.elapsed() {
            self.rotation_angle += Angle::radians(elapsed.as_secs_f32());
        }

        Ok(())
    }

    fn render(&mut self, scene: &Target) -> KludgineResult<()> {
        let sprite = self.source_sprite.as_ref().unwrap();

        let bounds = Rect::new(Point::default(), scene.size());

        sprite.render_at(
            scene,
            bounds.center(),
            SpriteRotation::around_center(self.rotation_angle),
        );

        Ok(())
    }
}
