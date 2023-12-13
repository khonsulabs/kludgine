use std::time::Duration;

use appit::winit::error::EventLoopError;
use appit::winit::keyboard::{Key, NamedKey};
use figures::units::{Px, UPx};
use kludgine::app::WindowBehavior;
use kludgine::figures::Size;
use kludgine::sprite::{
    AnimationMode, Sprite, SpriteAnimation, SpriteAnimations, SpriteFrame, SpriteSheet,
};
use kludgine::{Color, PreparedGraphic, Texture};

const SPRITE_SIZE: Size<UPx> = Size::new(UPx::new(32), UPx::new(32));

fn main() -> Result<(), EventLoopError> {
    Sprites::run()
}

struct Sprites {
    sprite: Sprite,
    current_frame: Option<PreparedGraphic<Px>>,
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

impl WindowBehavior for Sprites {
    type Context = ();

    fn clear_color(&self) -> Option<Color> {
        Some(Color::WHITE)
    }

    fn initialize(
        _window: kludgine::app::Window<'_, ()>,
        graphics: &mut kludgine::Graphics<'_>,
        _context: Self::Context,
    ) -> Self {
        let texture = Texture::from_image(
            image::open("./examples/assets/stickguy.png").expect("valid image"),
            wgpu::FilterMode::Nearest,
            graphics,
        );
        let sheet = SpriteSheet::new(
            texture,
            SPRITE_SIZE,
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
        );
        let idle = SpriteAnimation::new(
            sheet
                .sprites(vec![
                    StickGuy::Idle1,
                    StickGuy::Idle2,
                    StickGuy::Idle3,
                    StickGuy::Idle4,
                ])
                .into_iter()
                .map(|source| SpriteFrame {
                    source,
                    duration: Some(Duration::from_millis(500)),
                })
                .collect(),
        )
        .with_mode(AnimationMode::Forward);
        let walk_left = SpriteAnimation::new(
            sheet
                .sprites(vec![
                    StickGuy::WalkLeft1,
                    StickGuy::WalkLeft2,
                    StickGuy::WalkLeft3,
                ])
                .into_iter()
                .map(|source| SpriteFrame {
                    source,
                    duration: Some(Duration::from_millis(200)),
                })
                .collect(),
        )
        .with_mode(AnimationMode::PingPong);
        let walk_right = SpriteAnimation::new(
            sheet
                .sprites(vec![
                    StickGuy::WalkRight1,
                    StickGuy::WalkRight2,
                    StickGuy::WalkRight3,
                ])
                .into_iter()
                .map(|source| SpriteFrame {
                    source,
                    duration: Some(Duration::from_millis(200)),
                })
                .collect(),
        )
        .with_mode(AnimationMode::PingPong);
        let animations = SpriteAnimations::new(
            [
                (Some("Idle".to_string()), idle),
                (Some("WalkLeft".to_string()), walk_left),
                (Some("WalkRight".to_string()), walk_right),
            ]
            .into_iter()
            .collect(),
        );
        let mut sprite = Sprite::from(animations);
        sprite
            .set_current_tag(Some("Idle".to_string()))
            .expect("valid tag");
        Self {
            sprite,
            current_frame: None,
        }
    }

    fn prepare(
        &mut self,
        window: kludgine::app::Window<'_, ()>,
        graphics: &mut kludgine::Graphics<'_>,
    ) {
        self.current_frame = self
            .sprite
            .get_frame(Some(window.elapsed()))
            .ok()
            .map(|frame| frame.prepare(Size::squared(Px::new(64)).into(), graphics));
    }

    fn render<'pass>(
        &'pass mut self,
        mut window: kludgine::app::Window<'_, ()>,
        graphics: &mut kludgine::RenderingGraphics<'_, 'pass>,
    ) -> bool {
        if let Some(frame) = &self.current_frame {
            frame.render(graphics);
        }
        window.redraw_in(
            self.sprite
                .remaining_frame_duration()
                .expect("valid tag")
                .unwrap_or_default(),
        );
        true
    }

    fn keyboard_input(
        &mut self,
        mut window: kludgine::app::Window<'_, ()>,
        _kludgine: &mut kludgine::Kludgine,
        _device_id: appit::winit::event::DeviceId,
        input: appit::winit::event::KeyEvent,
        _is_synthetic: bool,
    ) {
        let tag = match (input.logical_key, input.text.as_deref()) {
            (Key::Named(NamedKey::ArrowLeft), _) | (_, Some("a")) => "WalkLeft",
            (Key::Named(NamedKey::ArrowRight), _) | (_, Some("d")) => "WalkRight",
            _ => return,
        };

        let new_tag = Some(if input.state.is_pressed() {
            tag
        } else {
            "Idle"
        });
        if self.sprite.current_tag() != new_tag {
            self.sprite.set_current_tag(new_tag).expect("valid tag");
            window.set_needs_redraw();
        }
    }
}
