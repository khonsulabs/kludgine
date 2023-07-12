use std::cmp::Ordering;
use std::time::Duration;

use appit::winit::event::VirtualKeyCode;
use figures::traits::{FloatConversion, IntoSigned};
use figures::units::{Lp, Px};
use kludgine::app::Window;
use kludgine::figures::{Point, Rect, Size};
use kludgine::render::Renderer;
use kludgine::shapes::Shape;
use kludgine::text::TextOrigin;
use kludgine::{Color, Origin};

const PADDLE_SPEED: Px = Px(300);
const PADDLE_HEIGHT: Px = Px(100);
const PADDLE_WIDTH: Px = Px(20);
const BALL_SIZE: Px = Px(PADDLE_WIDTH.0 / 2);
const BASE_VELOCITY: Px = PADDLE_SPEED;

fn main() {
    let mut state = GameState::default();
    kludgine::app::run(move |renderer, mut window| {
        window.redraw_in(Duration::from_millis(16));
        state.next_frame(renderer, window);

        true
    })
}

#[derive(Default, Debug)]
struct GameState {
    ball_pos: Point<Px>,
    ball_velocity: Point<Px>,
    player_paddle_position: Px,
    bot_paddle_position: Px,
    initialized: bool,
    reset_towards_player: bool,
    player_score: u32,
    bot_score: u32,
}

impl GameState {
    pub fn next_frame(&mut self, mut renderer: Renderer<'_, '_>, window: Window<'_>) {
        let size = renderer.size().into_signed();
        if !self.initialized {
            self.initialize(size);
        }

        let elapsed_seconds = window.elapsed().as_secs_f32();

        // Handle keyboard inputs
        let paddle_movement = PADDLE_SPEED * elapsed_seconds;
        if window.key_pressed(&VirtualKeyCode::Up) || window.key_pressed(&VirtualKeyCode::W) {
            self.player_paddle_position -= paddle_movement;
        }

        if window.key_pressed(&VirtualKeyCode::Down) || window.key_pressed(&VirtualKeyCode::D) {
            self.player_paddle_position += paddle_movement;
        }
        self.player_paddle_position = self.player_paddle_position.clamp(Px(0), size.height);

        // Handle bot actions. Just track the ball position
        let paddle_delta = self.ball_pos.y - self.bot_paddle_position;
        match paddle_delta.cmp(&Px(0)) {
            Ordering::Less => {
                self.bot_paddle_position -= paddle_movement;
            }
            Ordering::Equal => {}
            Ordering::Greater => {
                self.bot_paddle_position += paddle_movement;
            }
        }

        // Update the ball's position
        self.ball_pos += Point::from_float(self.ball_velocity.into_float() * elapsed_seconds);

        // Collision checks.
        if self.ball_pos.y <= BALL_SIZE / 2 || self.ball_pos.y + BALL_SIZE / 2 >= size.height {
            self.ball_velocity.y = -self.ball_velocity.y;
        }
        if (self.ball_pos.x <= PADDLE_WIDTH + BALL_SIZE / 2
            && self.paddle_hit_test(self.player_paddle_position))
            || (self.ball_pos.x + BALL_SIZE / 2 >= size.width - PADDLE_WIDTH
                && self.paddle_hit_test(self.bot_paddle_position))
        {
            self.hit_paddle();
        } else if self.ball_pos.x <= BALL_SIZE / 2 {
            self.bot_score += 1;
            self.reset_after_score(size);
        } else if self.ball_pos.x >= size.width - BALL_SIZE / 2 {
            self.player_score += 1;
            self.reset_after_score(size);
        }

        renderer.draw_shape(
            &Shape::filled_rect(
                Rect::new(
                    Point::new(Px(0), self.player_paddle_position - PADDLE_HEIGHT / 2),
                    Size::new(PADDLE_WIDTH, PADDLE_HEIGHT),
                ),
                Color::BLUE,
            ),
            Point::default(),
            None,
            None,
        );

        renderer.draw_shape(
            &Shape::filled_rect(
                Rect::new(
                    Point::new(
                        renderer.size().width.into_signed() - PADDLE_WIDTH,
                        self.bot_paddle_position - PADDLE_HEIGHT / 2,
                    ),
                    Size::new(PADDLE_WIDTH, PADDLE_HEIGHT),
                ),
                Color::RED,
            ),
            Point::default(),
            None,
            None,
        );

        renderer.draw_shape(
            &Shape::filled_circle(BALL_SIZE, Color::WHITE, Origin::Center),
            self.ball_pos,
            None,
            None,
        );

        renderer.set_font_size(Lp::inches(1));
        renderer.set_line_height(Lp::inches(1));
        renderer.draw_text(
            &self.player_score.to_string(),
            Color::BLUE,
            TextOrigin::Center,
            Point::new(size.width / 4, size.height / 4),
            None,
            None,
        );

        renderer.draw_text(
            &self.bot_score.to_string(),
            Color::RED,
            TextOrigin::Center,
            Point::new(size.width / 4 * 3, size.height / 4),
            None,
            None,
        );
    }

    fn initialize(&mut self, size: Size<Px>) {
        self.initialized = true;
        self.reset_after_score(size);
        self.player_paddle_position = self.ball_pos.y;
        self.bot_paddle_position = self.ball_pos.y;
    }

    fn reset_after_score(&mut self, size: Size<Px>) {
        self.ball_pos = Point::from(size / 2);
        self.ball_velocity = if self.reset_towards_player {
            Point::new(-BASE_VELOCITY, -BASE_VELOCITY)
        } else {
            Point::new(BASE_VELOCITY, BASE_VELOCITY)
        };
        self.reset_towards_player = !self.reset_towards_player;
    }

    fn hit_paddle(&mut self) {
        self.ball_velocity.x = -self.ball_velocity.x;
        self.ball_velocity = Point::from_float(self.ball_velocity.into_float() * 1.1);
    }

    fn paddle_hit_test(&self, paddle_position: Px) -> bool {
        self.ball_pos.y >= paddle_position - PADDLE_HEIGHT / 2
            && self.ball_pos.y <= paddle_position + PADDLE_HEIGHT / 2
    }
}
