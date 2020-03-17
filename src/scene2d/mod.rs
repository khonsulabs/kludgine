use generational_arena::Arena;
use std::sync::Mutex;
pub struct Scene2D {
    arena: Mutex<Arena<()>>,
}

impl Scene2D {
    pub fn new() -> Self {
        Self {
            arena: Mutex::new(Arena::new()),
        }
    }
}
