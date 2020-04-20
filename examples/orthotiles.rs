extern crate kludgine;
use kludgine::prelude::*;

fn main() {
    SingleWindowApplication::<OrthoTiles>::default().run();
}

#[derive(Default)]
struct OrthoTiles {
    map: Option<PersistentTileMap>,
    stickguy: Option<Sprite>,
    zoom: f32,
    x: f32,
    y: f32,
}

impl WindowCreator<OrthoTiles> for OrthoTiles {
    fn window_title() -> String {
        "Ortho Tiles - Kludgine".to_owned()
    }
}

static MAP_SIZE: u32 = 100;

#[async_trait]
impl Window for OrthoTiles {
    async fn initialize(&mut self) {
        self.zoom = 1.0;
    }

    async fn render(&mut self, scene: &mut Scene) -> KludgineResult<()> {
        scene.set_zoom(self.zoom);
        scene.set_origin(Point::new(
            -self.x + scene.size().width / 2.0,
            -self.y + scene.size().height / 2.0,
        ));
        if self.map.is_none() {
            let texture = Texture::load("examples/assets/grass.png")?;
            let sprite = Sprite::load_aseprite_json(include_str!("assets/grass.json"), texture)?;
            sprite.set_current_tag(Some("Swaying"));

            let mut map = PersistentTileMap::persistent_with_size(
                Size::new(32, 32),
                Size::new(MAP_SIZE, MAP_SIZE),
            );
            for x in 0..MAP_SIZE {
                for y in 0..MAP_SIZE {
                    map.set(Point::new(x, y), Some(sprite.new_instance()));
                }
            }

            self.map = Some(map);

            let texture = Texture::load("examples/assets/stickguy.png")?;
            self.stickguy = Some(Sprite::load_aseprite_json(
                include_str!("assets/stickguy.json"),
                texture,
            )?);
        }

        let map = self.map.as_ref().unwrap();
        map.draw(scene, Point::new(0, 0))?;

        let stickguy = self.stickguy.as_ref().unwrap();
        let mut animation = "Idle";
        if scene.pressed_keys.contains(&VirtualKeyCode::Right) {
            animation ="WalkRight";
            self.x += 32.0 * scene.elapsed().unwrap_or_default().as_secs_f32();
        } else if scene.pressed_keys.contains(&VirtualKeyCode::Left) {
            animation = "WalkLeft";
            self.x -= 32.0 * scene.elapsed().unwrap_or_default().as_secs_f32();
        }
        
        if scene.pressed_keys.contains(&VirtualKeyCode::Up) {
            self.y -= 32.0 * scene.elapsed().unwrap_or_default().as_secs_f32();
        } else if scene.pressed_keys.contains(&VirtualKeyCode::Down) {
            self.y += 32.0 * scene.elapsed().unwrap_or_default().as_secs_f32();
        }
        
        stickguy.set_current_tag(Some(animation));


        let sprite = stickguy.get_frame(scene.elapsed())?;
        sprite.render_at(scene, Point::new(self.x, self.y));

        Ok(())
    }

    async fn process_input(&mut self, event: InputEvent) -> KludgineResult<()> {
        match event.event {
            Event::MouseWheel { delta, .. } => {
                let zoom_amount = match delta {
                    MouseScrollDelta::LineDelta(_x, y) => y,
                    MouseScrollDelta::PixelDelta(point) => point.y as f32,
                };
                self.zoom = (self.zoom + zoom_amount / 100.0).min(10.0).max(0.1);
            }
            _ => {}
        }
        Ok(())
    }
}
