use super::{
    math::{Point, Raw, Scaled},
    scene::Scene,
    style::Style,
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

#[derive(Debug, Clone)]
pub struct Span {
    pub text: String,
    pub style: Style<Raw>,
}

impl Span {
    pub fn new<S: Into<String>>(text: S, style: Style<Raw>) -> Self {
        Self {
            text: text.into(),
            style,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Text {
    spans: Vec<Span>,
}

impl Text {
    pub fn span<S: Into<String>>(text: S, style: &Style<Raw>) -> Self {
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
        prepared_text.render(scene, location, offset_baseline).await
    }
}
