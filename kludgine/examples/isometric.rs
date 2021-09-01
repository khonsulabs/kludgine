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
impl Window for Isometric {
    fn initialize(
        &mut self,
        _scene: &Target,
        _requester: RedrawRequester,
        _window: WindowHandle,
    ) -> kludgine::Result<()> {
        self.load_assets()?;
        // self.zoom = 1.0;
        // self.x = MAP_SIZE as f32 * 32.0 / 2.0;
        // self.y = self.x;
        Ok(())
    }

    fn render(
        &mut self,
        scene: &Target,
        _status: &mut RedrawStatus,
        _window: WindowHandle,
    ) -> kludgine::Result<()> {
        let map = self.map.as_mut().unwrap();
        map.render(scene, Point::default())?;

        Ok(())
    }
}

impl Isometric {
    fn load_assets(&mut self) -> kludgine::Result<()> {
        let texture = Texture::load("kludgine/examples/assets/isometric_tile.png")?;
        let sprite = Sprite::single_frame(texture);

        let mut map = PersistentTileMap::persistent_with_size(
            Size::new(126, 79),
            Size::new(MAP_SIZE, MAP_SIZE),
        );
        // TODO this isn't actually working
        map.set_stagger(Size::new(63, 31));
        for x in 0..MAP_SIZE {
            for y in 0..MAP_SIZE {
                map.set(Point::new(x, y), Some(sprite.clone()));
            }
        }

        self.map = Some(map);
        Ok(())
    }
}
