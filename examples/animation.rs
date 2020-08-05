extern crate kludgine;
use kludgine::prelude::*;
use std::time::{Duration, Instant};

fn main() {
    SingleWindowApplication::run(Animation::default());
}

#[derive(Default)]
struct Animation {
    image: Entity<Image>,
    manager: RequiresInitialization<AnimationManager<ImageAlphaAnimation>>,
    fade_in: bool,
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
        context
            .set_style_sheet(
                Style {
                    background_color: Some(Color::GREEN),
                    ..Default::default()
                }
                .into(),
            )
            .await;
        let sprite = include_aseprite_sprite!("assets/stickguy").await?;
        sprite.set_current_tag(Some("Idle")).await?;
        self.image = self
            .new_entity(context, Image::new(sprite))
            .bounds(AbsoluteBounds {
                left: Dimension::from_points(30.),
                top: Dimension::from_points(30.),
                ..Default::default()
            })
            .insert()
            .await?;

        self.manager.initialize_with(AnimationManager::new(
            self.image.animate().alpha(0.3, LinearTransition),
        ));

        self.fade();

        Ok(())
    }

    async fn update(&mut self, _context: &mut SceneContext) -> KludgineResult<()> {
        self.manager.update().await;
        Ok(())
    }

    async fn clicked(
        &mut self,
        _context: &mut Context,
        _window_position: &Point<Points>,
        _button: MouseButton,
    ) -> KludgineResult<()> {
        self.fade();
        Ok(())
    }
}

impl Animation {
    fn fade(&mut self) {
        self.fade_in = !self.fade_in;
        let target_opacity = if self.fade_in { 1.0 } else { 0.1 };
        let completion_time = Instant::now().checked_add(Duration::from_secs(1)).unwrap();
        self.manager.push_frame(
            self.image.animate().alpha(target_opacity, LinearTransition),
            completion_time,
        );
    }
}
