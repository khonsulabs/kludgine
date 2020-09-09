extern crate kludgine;
use kludgine::prelude::*;

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

impl WindowCreator<OrthoTiles> for OrthoTiles {
    fn window_title() -> String {
        "Ortho Tiles - Kludgine".to_owned()
    }
}

static MAP_SIZE: u32 = 100;

impl Window for OrthoTiles {
    fn target_fps(&self) -> Option<u16> {
        Some(60)
    }
}

impl StandaloneComponent for OrthoTiles {}

#[async_trait]
impl Component for OrthoTiles {
    async fn initialize(&mut self, _context: &mut SceneContext) -> KludgineResult<()> {
        self.load_assets().await?;
        self.zoom = 1.0;
        self.position.x = MAP_SIZE as f32 * 32.0 / 2.0;
        self.position.y = self.position.x;
        Ok(())
    }

    async fn update(&mut self, context: &mut SceneContext) -> KludgineResult<()> {
        let stickguy = self.stickguy.as_ref().unwrap();
        // Our default animation is Idle
        let mut animation = "Idle";
        if context
            .scene()
            .pressed_keys()
            .await
            .contains(&VirtualKeyCode::Right)
        {
            animation = "WalkRight";
            self.position.x += 32.0
                * context
                    .scene()
                    .elapsed()
                    .await
                    .unwrap_or_default()
                    .as_secs_f32();
        } else if context
            .scene()
            .pressed_keys()
            .await
            .contains(&VirtualKeyCode::Left)
        {
            animation = "WalkLeft";
            self.position.x -= 32.0
                * context
                    .scene()
                    .elapsed()
                    .await
                    .unwrap_or_default()
                    .as_secs_f32();
        }

        if context
            .scene()
            .pressed_keys()
            .await
            .contains(&VirtualKeyCode::Up)
        {
            self.position.y -= 32.0
                * context
                    .scene()
                    .elapsed()
                    .await
                    .unwrap_or_default()
                    .as_secs_f32();
        } else if context
            .scene()
            .pressed_keys()
            .await
            .contains(&VirtualKeyCode::Down)
        {
            self.position.y += 32.0
                * context
                    .scene()
                    .elapsed()
                    .await
                    .unwrap_or_default()
                    .as_secs_f32();
        }
        stickguy.set_current_tag(Some(animation)).await?;

        Ok(())
    }

    async fn render(&self, context: &mut StyledContext, _layout: &Layout) -> KludgineResult<()> {
        let camera_scene = context.scene().set_camera(
            self.zoom,
            self.position - context.scene().size().await / 2.0 / self.zoom,
        );
        // The map is drawn at a static location of 0,0 (upper-left)
        // It will be offset scene.origin()
        let map = self.map.as_ref().unwrap();
        map.draw(&camera_scene, Point::default()).await?;

        // Draw the stickguy with the current frame of animation
        let stickguy = self.stickguy.as_ref().unwrap();
        let sprite = stickguy.get_frame(camera_scene.elapsed().await).await?;
        sprite
            .render_at(
                &camera_scene,
                Point::new(self.position.x - 16.0, self.position.y - 16.0),
            )
            .await;

        Ok(())
    }

    async fn process_input(
        &mut self,
        _context: &mut Context,
        event: InputEvent,
    ) -> KludgineResult<()> {
        if let Event::MouseWheel { delta, .. } = event.event {
            let zoom_amount = match delta {
                MouseScrollDelta::LineDelta(_x, y) => y,
                MouseScrollDelta::PixelDelta(point) => point.y as f32,
            };
            self.zoom = (self.zoom + zoom_amount / 100.0).min(10.0).max(0.2);
        }
        Ok(())
    }
}

impl OrthoTiles {
    async fn load_assets(&mut self) -> KludgineResult<()> {
        let sprite = include_aseprite_sprite!("assets/grass").await?;
        sprite.set_current_tag(Some("Swaying")).await?;

        let mut map = PersistentTileMap::persistent_with_size(
            Size::new(32, 32),
            Size::new(MAP_SIZE, MAP_SIZE),
        );
        for x in 0..MAP_SIZE {
            for y in 0..MAP_SIZE {
                map.set(Point::new(x, y), Some(sprite.new_instance().await));
            }
        }

        self.map = Some(map);

        self.stickguy = Some(include_aseprite_sprite!("assets/stickguy").await?);
        Ok(())
    }
}
