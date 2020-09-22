use crate::{
    math::{Length, Point, Raw, Scaled},
    scene::SceneTarget,
    shape::{Fill, Stroke},
    KludgineError, KludgineResult,
};
#[derive(Clone, Debug)]
pub(crate) struct Circle<S> {
    pub center: Point<f32, S>,
    pub radius: Length<f32, S>,
}

impl Circle<Scaled> {
    pub(crate) async fn translate_and_convert_to_device(
        &self,
        location: Point<f32, Scaled>,
        scene: &SceneTarget,
    ) -> Circle<Raw> {
        let effective_scale = scene.effective_scale_factor().await;
        let center =
            scene.user_to_device_point(location + self.center.to_vector()) * effective_scale;
        let radius = self.radius * effective_scale;
        Circle { center, radius }
    }
}

impl Circle<Raw> {
    pub fn build(
        &self,
        builder: &mut rgx_lyon::ShapeBuilder,
        stroke: &Option<Stroke>,
        fill: &Option<Fill>,
    ) -> KludgineResult<()> {
        if let Some(fill) = fill {
            builder.default_color = fill.color.rgba();
            lyon_tessellation::basic_shapes::fill_circle(
                self.center.cast_unit(),
                self.radius.get(),
                &fill.options,
                builder,
            )
            .map_err(KludgineError::TessellationError)?;
        }

        if let Some(stroke) = stroke {
            builder.default_color = stroke.color.rgba();
            lyon_tessellation::basic_shapes::stroke_circle(
                self.center.cast_unit(),
                self.radius.get(),
                &stroke.options,
                builder,
            )
            .map_err(KludgineError::TessellationError)?;
        }

        Ok(())
    }
}
