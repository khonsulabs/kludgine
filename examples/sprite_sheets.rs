extern crate kludgine;
use std::time::Duration;

use kludgine::prelude::*;

fn main() {
    SingleWindowApplication::run(SpriteSheetExample::default());
}

#[derive(Default)]
struct SpriteSheetExample {}

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

impl Window for SpriteSheetExample {}

impl StandaloneComponent for SpriteSheetExample {}

#[async_trait]
impl Component for SpriteSheetExample {
    async fn initialize(&mut self, context: &mut Context) -> KludgineResult<()> {
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
        self.new_entity(context, Image::new(sprite))
            .style_sheet(Style::new().with(BackgroundColor(Color::GREEN)))
            .insert()
            .await?;
        Ok(())
    }
}
