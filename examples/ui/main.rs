extern crate kludgine;
use kludgine::prelude::*;

mod main_menu;
mod old;

fn main() {
    SingleWindowApplication::run(main_menu::MainMenu);
}
