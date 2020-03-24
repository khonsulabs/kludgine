use kludgine::prelude::*;
use rand::{Rng, thread_rng};

/// This example is a simulation of a worst-case scenario for this engine where an enourmous string of related meshes
/// break the parallelization capabilities. There are ways to optimize this, but it doesn't seem to be a particularly
/// major use case. The design of nesting meshes really is more for UI layout and attaching limbs to main bodies, 
/// so the depth of the trees should be shallow, not 10000 deep.
fn main() {
    Runtime::new(SingleWindowApplication::<Stress>::default()).run();
}

#[derive(Default)]
struct Stress {
    created_shapes: bool,
}

impl WindowCreator<Stress> for Stress {
    fn window_title() -> String {
        "Ortho Shapes - Kludgine".to_owned()
    }
}

#[async_trait]
impl Window for Stress {
    async fn render_2d(&mut self, scene: &mut Scene2d) -> KludgineResult<()> {
        if !self.created_shapes {
            let mut rng = thread_rng();
            self.created_shapes = true;
            let material = MaterialKind::Solid {
                color: Color::new(255, 0, 0, 255),
            };
            let mesh = scene.screen().create_mesh(
                Shape::rect(&Rect::new(Point2d::new(0.0, 0.0), Size2d::new(1.0, 1.0))),
                material.clone(),
            );
            for _ in 1..10000 {
                let mesh = scene.create_mesh_clone(&mesh);
                let x = rng.gen_range(0.0, scene.size().width);
                let y = rng.gen_range(0.0, scene.size().height);
                scene.screen().place_mesh(
                    &mesh,
                    None,
                    Point2d::new(x, y),
                    Deg(0.0).into(),
                    1.0,
                )?;
            }
        }
        Ok(())
    }
}
