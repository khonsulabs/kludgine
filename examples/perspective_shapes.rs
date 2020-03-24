use kludgine::prelude::*;

fn main() {
    Runtime::new(SingleWindowApplication::<PerspectiveShapes>::default()).run();
}

#[derive(Default)]
struct PerspectiveShapes {
    created_shapes: bool,
}

impl WindowCreator<PerspectiveShapes> for PerspectiveShapes {
    fn window_title() -> String {
        "Perspective Shapes - Kludgine".to_owned()
    }
}

#[async_trait]
impl Window for PerspectiveShapes {
    async fn render_2d(&mut self, scene: &mut Scene2d) -> KludgineResult<()> {
        if !self.created_shapes {
            self.created_shapes = true;
            for i in 1..10 {
                let material = Material::Solid {
                    color: Color::new_rgba(255, i * 10, i * 10, 255),
                };
                let mesh = scene.create_mesh(
                    Shape::rect(&Rect::new(
                        Point2d::new(-0.5, -0.5),
                        Size2d::new(1.0, 1.0),
                    )),
                    material,
                );
                scene
                    .perspective()
                    .place_mesh(&mesh, None, Point3d::new(0.0, 0.0, -(i as f32)), Deg(5.0).into(), 1.0);
            }
        }
        Ok(())
    }
}