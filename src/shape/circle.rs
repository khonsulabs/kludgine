use crate::{
    math::{Pixels, Point, Points},
    scene::SceneTarget,
    shape::{Fill, Stroke},
    KludgineError, KludgineResult,
};
#[derive(Clone)]
pub(crate) struct Circle<S> {
    pub center: Point<S>,
    pub radius: S,
}

impl Circle<Points> {
    pub(crate) async fn translate_and_convert_to_device(
        &self,
        location: Point<Points>,
        scene: &SceneTarget,
    ) -> Circle<Pixels> {
        let effective_scale = scene.effective_scale_factor().await;
        let center = scene
            .user_to_device_point(location + self.center)
            .await
            .to_pixels(effective_scale);
        let radius = self.radius.to_pixels(effective_scale);
        Circle { center, radius }
    }
}

impl Circle<Pixels> {
    pub fn build(
        &self,
        builder: &mut rgx_lyon::ShapeBuilder,
        stroke: &Option<Stroke>,
        fill: &Option<Fill>,
    ) -> KludgineResult<()> {
        if let Some(fill) = fill {
            builder.default_color = fill.color.rgba();
            lyon_tessellation::basic_shapes::fill_circle(
                self.center.into(),
                self.radius.to_f32(),
                &fill.options,
                builder,
            )
            .map_err(KludgineError::TessellationError)?;
        }

        if let Some(stroke) = stroke {
            builder.default_color = stroke.color.rgba();
        }

        Ok(())
    }
}
