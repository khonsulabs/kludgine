use kludgine::prelude::*;
use kludgine::async_trait::async_trait;

fn main() {
    Runtime::new(SingleWindowApplication::<OrthoShapes>::default()).run();
}

#[derive(Default)]
struct OrthoShapes {
    created_shapes: bool,
}

impl WindowCreator<OrthoShapes> for OrthoShapes {
    
}

#[async_trait]
impl Window for OrthoShapes {
    async fn initialize(&mut self) {}
    async fn render_2d(&mut self, scene: &mut Scene2d) -> KludgineResult<()> {
        if !self.created_shapes {
            self.created_shapes = true;
            let material = Material::Solid {
                color: Color::new_rgba(255, 0, 0, 255),
            };
            let mesh = scene.screen().create_mesh(
                Shape::rect(&Rect::new(
                    Point2d::new(-16.0, -16.0),
                    Size2d::new(32.0, 32.0),
                )),
                material,
            );
            scene
                .screen()
                .place_mesh(&mesh, None, Point2d::new(48.0, 48.0), Rad(0.0), 1.0);
            let second_mesh = scene.screen().create_mesh_clone(&mesh);
            scene.screen().place_mesh(
                &second_mesh,
                Some(mesh.id),
                Point2d::new(32.0, 32.0),
                Deg(45.0).into(),
                1.5,
            );
            let third_mesh = scene.screen().create_mesh_clone(&mesh);
            scene.screen().place_mesh(
                &third_mesh,
                Some(second_mesh.id),
                Point2d::new(32.0, 32.0),
                Deg(0.0).into(),
                1.0,
            );
        }
        Ok(())
    }
}