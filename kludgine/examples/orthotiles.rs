use kludgine::prelude::*;
use kludgine_app::RedrawRequester;
use kludgine_core::figures::Vectorlike;

fn main() {
    SingleWindowApplication::run(OrthoTiles::default());
}

#[derive(Default)]
struct OrthoTiles {
    map: Option<PersistentTileMap>,
    stickguy: Option<Sprite>,
    zoom: f32,
    position: Point<f32, Scaled>,
}

impl WindowCreator for OrthoTiles {
    fn window_title() -> String {
        "Ortho Tiles - Kludgine".to_owned()
    }
}

static MAP_SIZE: u32 = 100;

impl Window for OrthoTiles {
    fn target_fps(&self) -> Option<u16> {
        Some(60)
    }

    fn initialize(&mut self, _scene: &Target, _requester: RedrawRequester) -> kludgine::Result<()> {
        self.load_assets()?;
        self.zoom = 1.0;
        // self.position.x = MAP_SIZE as f32 * 32.0 / 2.0;
        // self.position.y = self.position.x;
        Ok(())
    }

    fn update(&mut self, scene: &Target, _status: &mut RedrawStatus) -> kludgine::Result<()> {
        let stickguy = self.stickguy.as_mut().unwrap();
        // Our default animation is Idle
        let mut animation = "Idle";
        if scene.keys_pressed.contains(&VirtualKeyCode::Right) {
            animation = "WalkRight";
            self.position.x += 32.0 * scene.elapsed().unwrap_or_default().as_secs_f32();
        } else if scene.keys_pressed.contains(&VirtualKeyCode::Left) {
            animation = "WalkLeft";
            self.position.x -= 32.0 * scene.elapsed().unwrap_or_default().as_secs_f32();
        }

        if scene.keys_pressed.contains(&VirtualKeyCode::Up) {
            self.position.y -= 32.0 * scene.elapsed().unwrap_or_default().as_secs_f32();
        } else if scene.keys_pressed.contains(&VirtualKeyCode::Down) {
            self.position.y += 32.0 * scene.elapsed().unwrap_or_default().as_secs_f32();
        }
        stickguy.set_current_tag(Some(animation))?;

        Ok(())
    }

    fn render(&mut self, scene: &Target, _status: &mut RedrawStatus) -> kludgine::Result<()> {
        let center = scene.size().to_vector().to_point() / 2.0;
        let map = self.map.as_mut().unwrap();
        map.render_scaled(
            &scene,
            center - self.position.to_vector() * self.zoom,
            Scale::new(self.zoom),
        )?;

        // Draw the stickguy with the current frame of animation
        let stickguy = self.stickguy.as_mut().unwrap();
        let sprite = stickguy.get_frame(scene.elapsed())?;

        // Calculate the zoomed size
        let rendered_size = (sprite.location.size().cast::<f32>().to_vector() * self.zoom)
            .to_size()
            .cast_unit();
        let rendered_bounds = Rect::new(center - rendered_size / 2., rendered_size);

        sprite.render_within(&scene, rendered_bounds, SpriteRotation::none());

        Ok(())
    }

    fn process_input(
        &mut self,
        input: InputEvent,
        status: &mut RedrawStatus,
        _scene: &Target,
    ) -> kludgine::Result<()> {
        if let Event::MouseWheel { delta, .. } = input.event {
            let zoom_amount = match delta {
                MouseScrollDelta::LineDelta(_x, y) => y * 16.,
                MouseScrollDelta::PixelDelta(point) => point.y as f32,
            };
            self.zoom = (self.zoom + zoom_amount / 100.0).min(10.0).max(0.2);
            status.set_needs_redraw();
        }
        Ok(())
    }
}

impl OrthoTiles {
    fn load_assets(&mut self) -> kludgine::Result<()> {
        let mut sprite = include_aseprite_sprite!("assets/grass")?;
        sprite.set_current_tag(Some("Swaying"))?;

        let mut map = PersistentTileMap::persistent_with_size(
            Size::new(32, 32),
            Size::new(MAP_SIZE, MAP_SIZE),
        );
        for x in 0..MAP_SIZE {
            for y in 0..MAP_SIZE {
                map.set(Point::new(x, y), Some(sprite.clone()));
            }
        }

        self.map = Some(map);

        self.stickguy = Some(include_aseprite_sprite!("assets/stickguy")?);
        Ok(())
    }
}
