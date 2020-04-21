use kludgine::prelude::*;

// TODO: Can we make the examples in the other folder work correctly in tihs context? 

#[no_mangle]
pub extern fn rust_entry() {
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
    async fn initialize(&mut self) -> KludgineResult<()> {
        self.load_assets()?;
        self.zoom = 1.0;
        self.x = MAP_SIZE as f32 * 32.0 / 2.0;
        self.y = self.x;
        Ok(())
    }

    async fn update(&mut self, scene: &mut Scene) -> KludgineResult<()> {
        let stickguy = self.stickguy.as_ref().unwrap();
        // Our default animation is Idle
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
        stickguy.set_current_tag(Some(animation))?;

        // Set up the camera to zoom and center the stick guy
        scene.set_zoom(self.zoom);
        scene.set_origin(Point::new(
            -self.x + scene.size().width / 2.0,
            -self.y + scene.size().height / 2.0,
        ));

        Ok(())
    }

    async fn render(&mut self, scene: &mut Scene) -> KludgineResult<()> {
        // The map is drawn at a static location of 0,0 (upper-left)
        // It will be offset scene.origin()
        let map = self.map.as_ref().unwrap();
        map.draw(scene, Point::new(0, 0))?;

        // Draw the stickguy with the current frame of animation
        let stickguy = self.stickguy.as_ref().unwrap();
        let sprite = stickguy.get_frame(scene.elapsed())?;
        sprite.render_at(scene, Point::new(self.x - 16.0, self.y - 16.0));

        Ok(())
    }

    async fn process_input(&mut self, event: InputEvent) -> KludgineResult<()> {
        match event.event {
            Event::MouseWheel { delta, .. } => {
                let zoom_amount = match delta {
                    MouseScrollDelta::LineDelta(_x, y) => y,
                    MouseScrollDelta::PixelDelta(point) => point.y as f32,
                };
                self.zoom = (self.zoom + zoom_amount / 100.0).min(10.0).max(0.2);
            }
            _ => {}
        }
        Ok(())
    }
}


impl OrthoTiles {
    fn load_assets(&mut self) -> KludgineResult<()> {
        let sprite = include_aseprite_sprite!("../../assets/grass.json", "../../assets/grass.png")?;
        sprite.set_current_tag(Some("Swaying"))?;

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

        self.stickguy = Some(include_aseprite_sprite!("../../assets/stickguy.json", "../../assets/stickguy.png")?);
        Ok(())
    }
}