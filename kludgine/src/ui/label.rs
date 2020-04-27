use crate::{
    math::{Point, Rect, Size},
    scene::SceneTarget,
    style::Style,
    text::{Text, TextWrap},
    ui::view::{BaseView, View, ViewCore},
    KludgineResult,
};
use kludgine_macros::ViewCore;

#[derive(ViewCore, Debug, Default, Clone)]
pub struct Label {
    view: BaseView,
    value: Option<String>,
}

impl View for Label {
    fn render(&self, scene: &mut SceneTarget) -> KludgineResult<()> {
        let font = scene.lookup_font(
            &self.view.effective_style.font_family,
            self.view.effective_style.font_weight,
        )?;
        let metrics = font.metrics(self.view.effective_style.font_size);
        match self.create_text()? {
            Some(text) => text.render_at(
                scene,
                Point::new(
                    self.view.bounds.origin.x,
                    self.view.bounds.origin.y + metrics.ascent / scene.effective_scale_factor(),
                ),
                self.wrapping(&self.view.bounds.size),
            ),
            None => Ok(()),
        }
    }

    fn update_style(
        &mut self,
        scene: &mut SceneTarget,
        inherited_style: &Style,
    ) -> KludgineResult<()> {
        let inherited_style = self.view.style.inherit_from(&inherited_style);
        self.view.effective_style = inherited_style.effective_style(scene);
        Ok(())
    }

    fn layout_within(&mut self, scene: &mut SceneTarget, bounds: Rect) -> KludgineResult<()> {
        self.view
            .layout_within(&self.content_size(&bounds.size, scene)?, bounds)
    }

    fn content_size(&self, maximum_size: &Size, scene: &mut SceneTarget) -> KludgineResult<Size> {
        let size = match self.create_text()? {
            Some(text) => {
                text.wrap(
                    scene,
                    self.wrapping(&self.view.layout.size_with_minimal_padding(&maximum_size)),
                )?
                .size()
                    / scene.effective_scale_factor()
            }
            None => Size::default(),
        };
        Ok(size)
    }
}

impl Label {
    pub fn with_value<S: Into<String>>(&mut self, value: S) -> &mut Self {
        self.value = Some(value.into());
        self
    }

    fn create_text(&self) -> KludgineResult<Option<Text>> {
        if let Some(value) = &self.value {
            Ok(Some(Text::span(value, &self.view.effective_style)))
        } else {
            Ok(None)
        }
    }

    fn wrapping(&self, size: &Size) -> TextWrap {
        TextWrap::SingleLine {
            max_width: size.width,
            truncate: true,
        }
    }
}
