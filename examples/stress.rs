use kludgine::prelude::*;

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
            self.created_shapes = true;
            let mut last_mesh_id = None;
            let material = MaterialKind::Solid {
                color: Color::new(255, 0, 0, 255),
            };
            let mesh = scene.screen().create_mesh(
                Shape::rect(&Rect::new(Point2d::new(0.0, 0.0), Size2d::new(1.0, 1.0))),
                material.clone(),
            );
            for _ in 1..10000 {
                let mesh = scene.create_mesh_clone(&mesh);
                scene.screen().place_mesh(
                    &mesh,
                    last_mesh_id,
                    Point2d::new(1.0, 0.0),
                    Deg(5.0).into(),
                    1.0,
                );
                last_mesh_id = Some(mesh.id);
            }
        }
        Ok(())
    }
}
