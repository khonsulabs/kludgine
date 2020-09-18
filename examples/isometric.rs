extern crate kludgine;
use kludgine::prelude::*;

fn main() {
    SingleWindowApplication::run(Isometric::default());
}

#[derive(Default)]
struct Isometric {
    map: Option<PersistentTileMap>,
}

impl WindowCreator for Isometric {
    fn window_title() -> String {
        "Isometric - Kludgine".to_owned()
    }
}
static MAP_SIZE: u32 = 100;

impl Window for Isometric {}

impl StandaloneComponent for Isometric {}

#[async_trait]
impl Component for Isometric {
    async fn initialize(&mut self, _context: &mut SceneContext) -> KludgineResult<()> {
        self.load_assets().await?;
        // self.zoom = 1.0;
        // self.x = MAP_SIZE as f32 * 32.0 / 2.0;
        // self.y = self.x;
        Ok(())
    }

    async fn render(&self, context: &mut StyledContext, _layout: &Layout) -> KludgineResult<()> {
        let map = self.map.as_ref().unwrap();
        map.draw(context.scene(), Point::default()).await?;

        Ok(())
    }
}

impl Isometric {
    async fn load_assets(&mut self) -> KludgineResult<()> {
        let texture = Texture::load("examples/assets/isometric_tile.png")?;
        let sprite = Sprite::single_frame(texture).await;

        let mut map = PersistentTileMap::persistent_with_size(
            Size::new(126, 62),
            Size::new(MAP_SIZE, MAP_SIZE),
        );
        map.set_stagger(Size::new(63, 31));
        for x in 0..MAP_SIZE {
            for y in 0..MAP_SIZE {
                map.set(Point::new(x, y), Some(sprite.new_instance().await));
            }
        }

        self.map = Some(map);
        Ok(())
    }
}
