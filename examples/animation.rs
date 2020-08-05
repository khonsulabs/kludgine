extern crate kludgine;
use kludgine::prelude::*;

fn main() {
    SingleWindowApplication::run(Animation::default());
}

#[derive(Default)]
struct Animation {
    image: Entity<Image>,
}

impl WindowCreator<Animation> for Animation {
    fn window_title() -> String {
        "Animation - Kludgine".to_owned()
    }
}

impl Window for Animation {}

impl StandaloneComponent for Animation {}

#[async_trait]
impl Component for Animation {
    async fn initialize(&mut self, context: &mut SceneContext) -> KludgineResult<()> {
        let sprite = include_aseprite_sprite!("assets/stickguy").await?;
        self.image = self
            .new_entity(context, Image::new(sprite))
            .bounds(AbsoluteBounds {
                left: Dimension::from_points(30.),
                top: Dimension::from_points(30.),
                ..Default::default()
            })
            .insert()
            .await?;
        Ok(())
    }
}
