use kludgine::{application::Application, runtime::Runtime};
use kludgine::async_trait::async_trait;

fn main() {
    kludgine::runtime::Runtime::new::<OrthoShapes>().run();
}

struct OrthoShapes;

#[async_trait]
impl Application for OrthoShapes {
    fn new() -> Self {
        Self {}
    }
    async fn initialize(&mut self) {
        Runtime::open_window(
            glutin::window::WindowBuilder::new().with_title("Cosmic Verge"),
            MainWindow { meshes: Vec::new() },
        )
        .await
    }
}


struct MainWindow {
    meshes: Vec<Mesh2d>,
}
#[async_trait]
impl Window for MainWindow {
    async fn initialize(&mut self) {}
    async fn render_2d(&mut self, scene: &mut Scene2d) -> KludgineResult<()> {
        if self.meshes.len() == 0 {
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
            self.meshes.push(third_mesh);
            self.meshes.push(second_mesh);
            self.meshes.push(mesh);
        }
        Ok(())
    }
}