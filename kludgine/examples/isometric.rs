extern crate kludgine;
use kludgine::prelude::*;

fn main() {
    SingleWindowApplication::<Isometric>::default().run();
}

#[derive(Default)]
struct Isometric {
    map: Option<PersistentTileMap>,
}

impl WindowCreator<Isometric> for Isometric {
    fn window_title() -> String {
        "Isometric - Kludgine".to_owned()
    }
}
static MAP_SIZE: u32 = 100;

#[async_trait]
impl Window for Isometric {
    async fn initialize(&mut self, _scene: &mut Scene) -> KludgineResult<()> {
        self.load_assets()?;
        // self.zoom = 1.0;
        // self.x = MAP_SIZE as f32 * 32.0 / 2.0;
        // self.y = self.x;
        Ok(())
    }

    fn render(&mut self, scene: &mut Scene) -> KludgineResult<()> {
        let map = self.map.as_ref().unwrap();
        map.draw(scene, Point::zero())?;

        Ok(())
    }
}

impl Isometric {
    fn load_assets(&mut self) -> KludgineResult<()> {
        let texture = Texture::load("kludgine/examples/assets/isometric_tile.png")?;
        let sprite = Sprite::single_frame(texture);

        let mut map = PersistentTileMap::persistent_with_size(
            Size::new(126, 62),
            Size::new(MAP_SIZE, MAP_SIZE),
        );
        map.set_stagger(Size::new(63, 31));
        for x in 0..MAP_SIZE {
            for y in 0..MAP_SIZE {
                map.set(Point::new(x, y), Some(sprite.new_instance()));
            }
        }

        self.map = Some(map);
        Ok(())
    }
}
