use kludgine::prelude::*;

fn main() {
    Runtime::new(SingleWindowApplication::<PerspectiveShapes>::default()).run();
}

#[derive(Default)]
struct TextureWindow {
    created_shapes: bool,
}

impl WindowCreator<PerspectiveShapes> for TextureWindow {
    fn window_title() -> String {
        "Texture - Kludgine".to_owned()
    }
}

#[async_trait]
impl Window for TextureWindow {
    async fn render_2d(&mut self, scene: &mut Scene2d) -> KludgineResult<()> {
        if !self.created_shapes {
            self.created_shapes = true;
            let image = image::open("./examples/assets/moon.png")?;
            let material = MaterialKind::Textured {
                texture: image.into(),
            };
            let shape = Shape::rect(&Rect::new(Point2d::new(-0.5, -0.5), Size2d::new(1.0, 1.0)));
            shape.set_texture_coordinates(vec![Point2d::new(0.0, 1.0), Point2d::new(0.0, 0.0), Point2d::new(1.0, 0.0), Point2d::new(1.0, 1.0)]);
            let mesh = scene.create_mesh(
                shape,
                material,
            );
            scene.perspective().place_mesh(
                &mesh,
                None,
                Point3d::new(0.0, 0.0, -2.0),
                Deg(0.0).into(),
                1.0,
            )?;
        }
        Ok(())
    }
}
