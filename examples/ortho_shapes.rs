use kludgine::prelude::*;

fn main() {
    Runtime::new(SingleWindowApplication::<OrthoShapes>::default()).run();
}

#[derive(Default)]
struct OrthoShapes {
    created_shapes: bool,
}

impl WindowCreator<OrthoShapes> for OrthoShapes {
    fn window_title() -> String {
        "Ortho Shapes - Kludgine".to_owned()
    }
}

#[async_trait]
impl Window for OrthoShapes {
    async fn render_2d(&mut self, scene: &mut Scene2d) -> KludgineResult<()> {
        if !self.created_shapes {
            self.created_shapes = true;
            let mut last_mesh_id = None;
            for _ in 1..10 {
                let material = Material::Solid {
                    color: Color::new_rgba(255, 0, 0, 255),
                };
                let mesh = scene.screen().create_mesh(
                    Shape::rect(&Rect::new(
                        Point2d::new(0.0, 0.0),
                        Size2d::new(32.0, 32.0),
                    )),
                    material,
                );
                scene
                    .screen()
                    .place_mesh(&mesh, last_mesh_id, Point2d::new(32.0, 0.0), Deg(5.0).into(), 1.2);
                    last_mesh_id = Some(mesh.id);
            }
        }
        Ok(())
    }
}