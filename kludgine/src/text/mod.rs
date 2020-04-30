use super::{
    math::Point,
    scene::{Element, SceneTarget},
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

    pub async fn wrap<'a>(
        &self,
        scene: &mut SceneTarget<'a>,
        options: TextWrap,
    ) -> KludgineResult<PreparedText> {
        TextWrapper::wrap(self, scene, options).await // TODO cache result
    }

    pub async fn render_at<'a>(
        &self,
        scene: &mut SceneTarget<'a>,
        location: Point,
        wrapping: TextWrap,
    ) -> KludgineResult<()> {
        let prepared_text = self.wrap(scene, wrapping).await?;
        let mut current_line_baseline = 0.0;
        let effective_scale_factor = scene.effective_scale_factor();

        for line in prepared_text.lines.iter() {
            let metrics = line.metrics.as_ref().unwrap();
            let cursor_position = Point::new(location.x, location.y + current_line_baseline);
            for span in line.spans.iter() {
                let mut location = scene
                    .user_to_device_point(Point::new(cursor_position.x, cursor_position.y))
                    * effective_scale_factor;
                location.x += span.x().await;
                scene.push_element(Element::Text(span.translate(location)));
            }
            current_line_baseline = current_line_baseline
                + (metrics.ascent - metrics.descent + metrics.line_gap) / effective_scale_factor;
        }

        Ok(())
    }
}
