extern crate kludgine;
use kludgine::prelude::*;

fn main() {
    SingleWindowApplication::<OrthoTiles>::default().run();
}

#[derive(Default)]
struct OrthoTiles {
    map: Option<PersistentTileMap>,
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
        scene.set_origin(Point::new(self.x, self.y));
        if self.map.is_none() {
            let texture = Texture::load("examples/assets/grass.png")?;
            let mut atlas = Atlas::from(texture);
            let sprite =
                Sprite::load_aseprite_json(include_str!("assets/grass.json"), &mut atlas)?;
            sprite.set_current_tag(Some("Swaying"));

            let mut map =
                PersistentTileMap::persistent_with_size(Size::new(32, 32), Size::new(MAP_SIZE, MAP_SIZE));
            map.register_atlas(atlas);
            for x in 0..MAP_SIZE {
                for y in 0..MAP_SIZE {
                    map.set(Point::new(x, y), Some(sprite.new_instance()));
                }
            }

            self.map = Some(map);
        }

        let map = self.map.as_ref().unwrap();
        map.draw(scene, Point::zero())?;

        Ok(())
    }

    async fn process_input(&mut self, event: InputEvent) -> KludgineResult<()> {
        match event.event {
            Event::MouseWheel{delta, ..} => {
                let zoom_amount = match delta {
                    MouseScrollDelta::LineDelta(_x,y) => y,
                    MouseScrollDelta::PixelDelta(point) => point.y as f32,
                };
                self.zoom = (self.zoom + zoom_amount / 100.0).min(10.0).max(0.1);
            }
            Event::Keyboard{key, state} => {
                if let Some(code) = key {
                    if state == ElementState::Pressed {
                        match code {
                            VirtualKeyCode::Up => {
                                self.y -= 1.0;
                            }
                            VirtualKeyCode::Down => {
                                self.y += 1.0;
                            }
                            VirtualKeyCode::Left => {
                                self.x -= 1.0;
                            }
                            VirtualKeyCode::Right => {
                                self.x += 1.0;
                            }
                            _ => {}
                        }
                    }
                }
            }
            _ => {}
        }
        Ok(())
    }
}
