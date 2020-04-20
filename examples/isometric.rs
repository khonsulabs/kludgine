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

#[async_trait]
impl Window for Isometric {
    async fn render(&mut self, scene: &mut Scene) -> KludgineResult<()> {
        if self.map.is_none() {
            let texture = Texture::load("examples/assets/isometric_tile.png")?;
            let sprite = Sprite::single_frame(texture);

            let mut map =
                PersistentTileMap::persistent_with_size(Size::new(120, 80), Size::new(10, 10));
            map.set(Point::new(0, 0), Some(sprite.new_instance()));
            map.set(Point::new(0, 1), Some(sprite.new_instance()));
            map.set(Point::new(1, 1), Some(sprite.new_instance()));
            map.set(Point::new(1, 0), Some(sprite));

            self.map = Some(map);
        }

        let map = self.map.as_ref().unwrap();
        map.draw(scene, Point::zero())?;

        Ok(())
    }
}
