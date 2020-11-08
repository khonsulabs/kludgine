use crossbeam::sync::ShardedLock;
use once_cell::sync::OnceCell;
use std::{
    collections::HashMap,
    fmt::{Debug, Display},
};

use crate::{math::Scaled, style::UnscaledStyleComponent};

static SELECTOR_MAP: OnceCell<ShardedLock<HashMap<String, Selector>>> = OnceCell::new();

fn selector_map() -> &'static ShardedLock<HashMap<String, Selector>> {
    SELECTOR_MAP.get_or_init(|| ShardedLock::new(HashMap::new()))
}

fn string_to_static_str(s: String) -> &'static str {
    Box::leak(s.into_boxed_str())
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub struct Selector(&'static str);

impl From<&str> for Selector {
    fn from(string: &str) -> Self {
        let selector_map = selector_map();
        {
            let selector_map = selector_map.read().unwrap();
            if let Some(selector) = selector_map.get(string) {
                return *selector;
            }
        }

        let mut selector_map = selector_map.write().unwrap();
        let new_selector = Selector(string_to_static_str(string.to_string()));
        *selector_map
            .entry(string.to_string())
            .or_insert(new_selector)
    }
}

impl Display for Selector {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Display::fmt(self.0, f)
    }
}

#[derive(Debug, Clone)]
pub struct Id(pub Selector);

impl UnscaledStyleComponent<Scaled> for Id {}

impl From<Selector> for Id {
    fn from(selector: Selector) -> Self {
        Self(selector)
    }
}

impl From<&str> for Id {
    fn from(selector: &str) -> Self {
        Self(Selector::from(selector))
    }
}

#[derive(Debug, Clone)]
pub struct Classes(pub Vec<Selector>);

impl UnscaledStyleComponent<Scaled> for Classes {}

impl From<Selector> for Classes {
    fn from(selector: Selector) -> Self {
        Self(vec![selector])
    }
}

impl From<&str> for Classes {
    fn from(selector: &str) -> Self {
        Self::from(Selector::from(selector))
    }
}
