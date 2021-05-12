extern crate kludgine;
use kludgine::prelude::*;
use std::time::Duration;

fn main() {
    SingleWindowApplication::run(SpriteSheetExample::default());
}

#[derive(Default)]
struct SpriteSheetExample {
    sprite: Option<Sprite>,
    current_frame: Option<SpriteSource>,
}

impl WindowCreator for SpriteSheetExample {
    fn window_title() -> String {
        "Sprite Sheet - Kludgine".to_owned()
    }
}

#[derive(Debug, Hash, Eq, PartialEq)]
enum StickGuy {
    Idle1,
    Idle2,
    Idle3,
    Idle4,
    WalkRight1,
    WalkRight2,
    WalkRight3,
    WalkLeft1,
    WalkLeft2,
    WalkLeft3,
}

#[async_trait]
impl Window for SpriteSheetExample {
    async fn initialize(
        &mut self,
        _scene: &Target,
        _window: &OpenWindow<Self>,
    ) -> KludgineResult<()> {
        let texture = include_texture!("assets/stickguy.png")?;
        let sheet = SpriteSheet::new(
            texture,
            Size::new(32, 32),
            vec![
                StickGuy::Idle1,
                StickGuy::Idle2,
                StickGuy::Idle3,
                StickGuy::Idle4,
                StickGuy::WalkRight1,
                StickGuy::WalkRight2,
                StickGuy::WalkRight3,
                StickGuy::WalkLeft1,
                StickGuy::WalkLeft2,
                StickGuy::WalkLeft3,
            ],
        )
        .await;
        let idle = SpriteAnimation::new(
            sheet
                .sprites(vec![
                    StickGuy::Idle1,
                    StickGuy::Idle2,
                    StickGuy::Idle3,
                    StickGuy::Idle4,
                ])
                .await
                .into_iter()
                .map(|source| SpriteFrame {
                    source,
                    duration: Some(Duration::from_millis(500)),
                })
                .collect(),
            AnimationMode::Forward,
        );
        let walk_left = SpriteAnimation::new(
            sheet
                .sprites(vec![
                    StickGuy::WalkLeft1,
                    StickGuy::WalkLeft2,
                    StickGuy::WalkLeft3,
                ])
                .await
                .into_iter()
                .map(|source| SpriteFrame {
                    source,
                    duration: Some(Duration::from_millis(200)),
                })
                .collect(),
            AnimationMode::PingPong,
        );
        let walk_right = SpriteAnimation::new(
            sheet
                .sprites(vec![
                    StickGuy::WalkRight1,
                    StickGuy::WalkRight2,
                    StickGuy::WalkRight3,
                ])
                .await
                .into_iter()
                .map(|source| SpriteFrame {
                    source,
                    duration: Some(Duration::from_millis(200)),
                })
                .collect(),
            AnimationMode::PingPong,
        );
        let animations = SpriteAnimations::new(kludgine::hash_map!(
            Some("Idle".to_string()) => idle,
            Some("WalkLeft".to_string()) => walk_left,
            Some("WalkRight".to_string()) => walk_right,
        ));
        let sprite = Sprite::from(animations);
        sprite.set_current_tag(Some("Idle".to_string())).await?;
        self.sprite = Some(sprite);

        Ok(())
    }

    async fn update(&mut self, scene: &Target, window: &OpenWindow<Self>) -> KludgineResult<()>
    where
        Self: Sized,
    {
        let sprite = self.sprite.as_ref().unwrap();
        // Update the current frame.
        self.current_frame = Some(sprite.get_frame(scene.elapsed().await).await?);
        // Tell the window when this sprite will need to redraw  new frame.
        if let Some(duration) = sprite.remaining_frame_duration().await? {
            window.estimate_next_frame(duration).await;
        } else {
            window.set_needs_redraw().await;
        }

        Ok(())
    }

    async fn render(&mut self, scene: &Target) -> KludgineResult<()> {
        Shape::rect(Rect::new(Point::default(), scene.size().await))
            .fill(Fill::new(Color::WHITE))
            .render_at(Point::default(), scene)
            .await;

        let sprite = self.current_frame.as_ref().unwrap();
        let bounds = Rect::new(Point::default(), scene.size().await);

        sprite
            .render_at(scene, bounds.center(), SpriteRotation::default())
            .await;

        Ok(())
    }
}
