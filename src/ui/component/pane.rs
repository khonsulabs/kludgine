use crate::{
    style::theme::Selector,
    ui::component::{Component, StandaloneComponent},
};
use async_trait::async_trait;

#[derive(Debug, Default)]
pub struct Pane {}

#[async_trait]
impl Component for Pane {
    fn classes(&self) -> Option<Vec<Selector>> {
        Some(vec![Selector::from("pane")])
    }
}

#[async_trait]
impl StandaloneComponent for Pane {}
