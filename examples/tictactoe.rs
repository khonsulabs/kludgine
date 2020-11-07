// https://en.wikipedia.org/wiki/Tic-tac-toe

extern crate kludgine;
use std::fmt::Display;

use kludgine::prelude::*;
use rand::{thread_rng, Rng};

fn main() {
    SingleWindowApplication::run(TicTacToe::default());
}

#[derive(Eq, PartialEq, Clone, Copy)]
enum Player {
    X,
    O,
}

enum Winner {
    Draw,
    Player(Player),
}

impl Display for Player {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Player::X => write!(f, "X"),
            Player::O => write!(f, "O"),
        }
    }
}

impl Player {
    fn next_player(&self) -> Self {
        match self {
            Player::X => Player::O,
            Player::O => Player::X,
        }
    }

    fn random() -> Self {
        let mut rng = thread_rng();
        if rng.gen() {
            Player::X
        } else {
            Player::O
        }
    }
}

#[derive(Clone, Debug)]
enum GameMessage {
    TileClicked(usize, ControlEvent),
}

#[derive(Default)]
struct TicTacToe {
    current_player: Option<Player>,
    tiles: [Option<Player>; 9],
    labels: [Entity<Label>; 9],
    player_turn_label: Entity<Label>,
    message_label: Entity<Label>,
}

impl WindowCreator for TicTacToe {
    fn window_title() -> String {
        "Tic-tac-toe - Kludgine".to_owned()
    }
}

impl Window for TicTacToe {}

#[async_trait]
impl InteractiveComponent for TicTacToe {
    type Command = ();
    type Message = GameMessage;
    type Event = ();

    async fn receive_message(
        &mut self,
        _context: &mut Context,
        message: Self::Message,
    ) -> KludgineResult<()> {
        match message {
            GameMessage::TileClicked(tile, event) => {
                let ControlEvent::Clicked { .. } = event;
                if self.tiles[tile].is_none() {
                    self.play_tile(tile).await?;
                } else {
                    self.set_error_message("Tile already taken. Choose another.")
                        .await?;
                }
            }
        }

        Ok(())
    }
}

#[async_trait]
impl Component for TicTacToe {
    async fn initialize(&mut self, context: &mut Context) -> KludgineResult<()> {
        self.player_turn_label = self
            .new_entity(context, Label::new(""))
            .await
            .style_sheet(
                Style::default()
                    .with(FontSize::new(48.))
                    .with(Alignment::Center),
            )
            .insert()
            .await?;

        self.message_label = self
            .new_entity(context, Label::new(""))
            .await
            .style_sheet(
                Style::default()
                    .with(FontSize::new(48.))
                    .with(Alignment::Center),
            )
            .insert()
            .await?;

        for i in 0..9 {
            let mut style = Style::default()
                .with(FontSize::new(72.))
                .with(Alignment::Center)
                .with(VerticalAlignment::Center);
            if let Some(border) = Self::border_for_tile(i) {
                style = style.with(border);
            }

            self.labels[i] = self
                .new_entity(context, Label::new(""))
                .await
                .style_sheet(style)
                .callback(move |evt| GameMessage::TileClicked(i, evt))
                .insert()
                .await?;
        }

        self.set_player(Player::random()).await?;

        Ok(())
    }

    async fn layout(
        &mut self,
        _context: &mut StyledContext,
    ) -> KludgineResult<Box<dyn LayoutSolver>> {
        RowLayout::default()
            .row(self.player_turn_label.index(), Dimension::Minimal)
            .row(
                ColumnLayout::default()
                    .column(self.labels[0].index(), Dimension::Auto)
                    .column(self.labels[1].index(), Dimension::Auto)
                    .column(self.labels[2].index(), Dimension::Auto),
                Dimension::Auto,
            )
            .row(
                ColumnLayout::default()
                    .column(self.labels[3].index(), Dimension::Auto)
                    .column(self.labels[4].index(), Dimension::Auto)
                    .column(self.labels[5].index(), Dimension::Auto),
                Dimension::Auto,
            )
            .row(
                ColumnLayout::default()
                    .column(self.labels[6].index(), Dimension::Auto)
                    .column(self.labels[7].index(), Dimension::Auto)
                    .column(self.labels[8].index(), Dimension::Auto),
                Dimension::Auto,
            )
            .row(self.message_label.index(), Dimension::Minimal)
            .layout()
    }
}

impl TicTacToe {
    async fn play_tile(&mut self, tile: usize) -> KludgineResult<()> {
        if let Some(player) = self.current_player {
            self.tiles[tile] = Some(player);
            self.labels[tile]
                .send(LabelCommand::SetValue(player.to_string()))
                .await?;

            match self.game_winner() {
                Some(winner) => {
                    match winner {
                        Winner::Draw => self.set_error_message("It's a tie.").await?,
                        Winner::Player(player) => {
                            self.set_success_message(format!("Player {} Wins!", player))
                                .await?
                        }
                    }

                    self.new_game().await?;
                }
                None => {
                    self.set_player(player.next_player()).await?;
                    self.set_message("").await?;
                }
            }
        }

        Ok(())
    }

    async fn set_player(&mut self, player: Player) -> KludgineResult<()> {
        self.current_player = Some(player);

        self.player_turn_label
            .send(LabelCommand::SetValue(format!(
                "{}'s turn",
                player.to_string()
            )))
            .await
    }

    async fn set_error_message<S: ToString>(&mut self, message: S) -> KludgineResult<()> {
        self.set_message(message).await
    }

    async fn set_success_message<S: ToString>(&mut self, message: S) -> KludgineResult<()> {
        self.set_message(message).await
    }

    async fn set_message<S: ToString>(&mut self, message: S) -> KludgineResult<()> {
        self.message_label
            .send(LabelCommand::SetValue(message.to_string()))
            .await
    }

    fn game_winner(&self) -> Option<Winner> {
        self.check_tiles([0, 1, 2])
            .or_else(|| self.check_tiles([3, 4, 5]))
            .or_else(|| self.check_tiles([6, 7, 8]))
            .or_else(|| self.check_tiles([0, 3, 6]))
            .or_else(|| self.check_tiles([1, 4, 7]))
            .or_else(|| self.check_tiles([2, 5, 8]))
            .or_else(|| self.check_tiles([0, 4, 8]))
            .or_else(|| self.check_tiles([2, 4, 6]))
            .or_else(|| {
                if !self.tiles.iter().any(|t| t.is_none()) {
                    Some(Winner::Draw)
                } else {
                    None
                }
            })
    }

    fn check_tiles(&self, tiles: [usize; 3]) -> Option<Winner> {
        let first_tile = self.tiles[tiles[0]];
        if let Some(player) = first_tile {
            if first_tile == self.tiles[tiles[1]] && first_tile == self.tiles[tiles[2]] {
                return Some(Winner::Player(player));
            }
        }
        None
    }

    async fn new_game(&mut self) -> KludgineResult<()> {
        self.set_player(Player::random()).await?;

        for i in 0..9 {
            self.tiles[i] = None;
            self.labels[i]
                .send(LabelCommand::SetValue(Default::default()))
                .await?;
        }

        Ok(())
    }

    fn border_for_tile(tile: usize) -> Option<ComponentBorder> {
        match tile {
            0 | 1 | 3 | 4 => Some(
                ComponentBorder::default()
                    .with_right(Border::new(1., Color::BLACK.into()))
                    .with_bottom(Border::new(1., Color::BLACK.into())),
            ),
            2 | 5 => {
                Some(ComponentBorder::default().with_bottom(Border::new(1., Color::BLACK.into())))
            }
            6 => Some(ComponentBorder::default().with_right(Border::new(1., Color::BLACK.into()))),
            7 => Some(ComponentBorder::default().with_right(Border::new(1., Color::BLACK.into()))),
            8 => None,
            _ => unreachable!(),
        }
    }
}
