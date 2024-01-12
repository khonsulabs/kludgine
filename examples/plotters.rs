use std::time::Duration;

use appit::winit::error::EventLoopError;
use plotters::coord::Shift;
use plotters::prelude::*;

// This is copied from the sierpinski.rs example in the plotters
// repository.
pub fn sierpinski_carpet<A>(
    depth: u32,
    drawing_area: &DrawingArea<A, Shift>,
) -> Result<(), Box<dyn std::error::Error>>
where
    A: DrawingBackend,
    A::ErrorType: 'static,
{
    if depth > 0 {
        let sub_areas = drawing_area.split_evenly((3, 3));
        for (idx, sub_area) in (0..).zip(sub_areas.iter()) {
            if idx != 4 {
                sub_area.fill(&BLUE)?;
                sierpinski_carpet(depth - 1, sub_area)?;
            } else {
                sub_area.fill(&WHITE)?;
            }
        }
    }
    Ok(())
}

fn main() -> Result<(), EventLoopError> {
    let mut depth = 1;
    kludgine::app::run(move |mut renderer, mut window| {
        sierpinski_carpet(depth, &renderer.as_plot_area()).unwrap();

        depth += 1;
        if depth == 6 {
            depth = 1;
        }

        window.redraw_in(Duration::from_secs(1));
        true
    })
}
