pub trait VecExt {
    fn reversed(self) -> Self;
}

impl<T> VecExt for Vec<T> {
    fn reversed(mut self) -> Self {
        self.reverse();
        self
    }
}
