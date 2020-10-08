use super::{
    math::{Point, Points, Scaled, Vector},
    scene::{Element, Scene},
    style::EffectiveStyle,
    KludgineResult,
};

#[cfg(feature = "bundled-fonts-enabled")]
pub mod bundled_fonts;
pub mod font;
pub mod prepared;
pub mod wrap;
use font::*;
use prepared::*;
use wrap::*;

#[derive(Debug)]
pub struct Span {
    pub text: String,
    pub style: EffectiveStyle,
}

impl Span {
    pub fn new<S: Into<String>>(text: S, style: EffectiveStyle) -> Self {
        Self {
            text: text.into(),
            style,
        }
    }
}

#[derive(Debug)]
pub struct Text {
    spans: Vec<Span>,
}

impl Text {
    pub fn span<S: Into<String>>(text: S, style: &EffectiveStyle) -> Self {
        Self {
            spans: vec![Span::new(text, style.clone())],
        }
    }

    pub fn new(spans: Vec<Span>) -> Self {
        Self { spans }
    }

    pub async fn wrap(&self, scene: &Scene, options: TextWrap) -> KludgineResult<PreparedText> {
        TextWrapper::wrap(self, scene, options).await // TODO cache result
    }

    pub async fn render_at(
        &self,
        scene: &Scene,
        location: Point<f32, Scaled>,
        wrapping: TextWrap,
    ) -> KludgineResult<()> {
        self.render_core(scene, location, true, wrapping).await
    }

    pub async fn render_baseline_at(
        &self,
        scene: &Scene,
        location: Point<f32, Scaled>,
        wrapping: TextWrap,
    ) -> KludgineResult<()> {
        self.render_core(scene, location, false, wrapping).await
    }

    async fn render_core(
        &self,
        scene: &Scene,
        location: Point<f32, Scaled>,
        offset_baseline: bool,
        wrapping: TextWrap,
    ) -> KludgineResult<()> {
        let prepared_text = self.wrap(scene, wrapping).await?;
        let mut current_line_baseline = Points::new(0.);
        let effective_scale_factor = scene.scale_factor().await;

        if offset_baseline && !prepared_text.lines.is_empty() {
            current_line_baseline += prepared_text.lines[0].metrics.ascent / effective_scale_factor;
        }

        for line in prepared_text.lines.iter() {
            let metrics = line.metrics;
            let cursor_position =
                location + Vector::from_lengths(line.alignment_offset, current_line_baseline);
            for span in line.spans.iter() {
                let location = (cursor_position
                    + span.location.to_vector() / effective_scale_factor)
                    * effective_scale_factor;
                scene
                    .push_element(Element::Text(span.translate(location)))
                    .await;
            }
            current_line_baseline +=
                (metrics.ascent - metrics.descent + metrics.line_gap) / effective_scale_factor;
        }

        Ok(())
    }
}
