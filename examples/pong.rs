use std::cmp::Ordering;
use std::time::Duration;

use appit::winit::error::EventLoopError;
use appit::winit::keyboard::KeyCode;
use kludgine::app::Window;
use kludgine::figures::units::{Lp, Px};
use kludgine::figures::{FloatConversion, IntoSigned, Point, Rect, Size};
use kludgine::render::Renderer;
use kludgine::shapes::Shape;
use kludgine::text::{Text, TextOrigin};
use kludgine::{Color, DrawableExt, Origin};

const PADDLE_SPEED: Px = Px::new(300);
const PADDLE_HEIGHT: Px = Px::new(100);
const PADDLE_WIDTH: Px = Px::new(20);
const BALL_SIZE: Px = Px::new(PADDLE_WIDTH.get() / 2);
const BASE_VELOCITY: Px = PADDLE_SPEED;

fn main() -> Result<(), EventLoopError> {
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
        if window.key_pressed(KeyCode::ArrowUp) || window.key_pressed(KeyCode::KeyW) {
            self.player_paddle_position -= paddle_movement;
        }

        if window.key_pressed(KeyCode::ArrowDown) || window.key_pressed(KeyCode::KeyS) {
            self.player_paddle_position += paddle_movement;
        }
        self.player_paddle_position = self.player_paddle_position.clamp(Px::ZERO, size.height);

        // Handle bot actions. Just track the ball position
        let paddle_delta = self.ball_pos.y - self.bot_paddle_position;
        match paddle_delta.cmp(&Px::ZERO) {
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

        renderer.draw_shape(&Shape::filled_rect(
            Rect::new(
                Point::new(Px::ZERO, self.player_paddle_position - PADDLE_HEIGHT / 2),
                Size::new(PADDLE_WIDTH, PADDLE_HEIGHT),
            ),
            Color::BLUE,
        ));

        renderer.draw_shape(&Shape::filled_rect(
            Rect::new(
                Point::new(
                    renderer.size().width.into_signed() - PADDLE_WIDTH,
                    self.bot_paddle_position - PADDLE_HEIGHT / 2,
                ),
                Size::new(PADDLE_WIDTH, PADDLE_HEIGHT),
            ),
            Color::RED,
        ));

        renderer.draw_shape(
            Shape::filled_circle(BALL_SIZE, Color::WHITE, Origin::Center)
                .translate_by(self.ball_pos),
        );

        renderer.set_font_size(Lp::inches(1));
        renderer.set_line_height(Lp::inches(1));
        renderer.draw_text(
            Text::new(&self.player_score.to_string(), Color::BLUE)
                .origin(TextOrigin::Center)
                .translate_by(Point::from(size / 4)),
        );

        renderer.draw_text(
            Text::new(&self.bot_score.to_string(), Color::RED)
                .origin(TextOrigin::Center)
                .translate_by(Point::new(size.width / 4 * 3, size.height / 4)),
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
