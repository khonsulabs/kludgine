extern crate kludgine;
use kludgine::prelude::*;

fn main() {
    SingleWindowApplication::<OrthoTiles>::default().run();
}

#[derive(Default)]
struct OrthoTiles {
    map: Option<PersistentTileMap>,
}

impl WindowCreator<OrthoTiles> for OrthoTiles {
    fn window_title() -> String {
        "Ortho Tiles - Kludgine".to_owned()
    }
}

#[async_trait]
impl Window for OrthoTiles {
    async fn render(&mut self, scene: &mut Scene) -> KludgineResult<()> {
        if self.map.is_none() {
            let texture = Texture::load("examples/assets/grass.png")?;
            let mut atlas = Atlas::from(texture);
            let mut sprite =
                Sprite::load_aseprite_json(include_str!("assets/grass.json"), &mut atlas)?;
            sprite.set_current_tag(Some("Swaying"));

            let mut map =
                PersistentTileMap::persistent_with_size(Size::new(32, 32), Size::new(10, 10));
            map.register_atlas(atlas);
            for x in 0..10 {
                for y in 0..10 {
                    map.set(Point::new(x, y), Some(sprite.clone()));
                }
            }

            self.map = Some(map);
        }

        let map = self.map.as_ref().unwrap();
        map.draw(scene, Point::zero())?;

        Ok(())
    }
}
