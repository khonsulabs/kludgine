use std::{
    cmp::Ordering,
    time::{Duration, Instant},
};

use crate::{
    color::Color,
    event::MouseButton,
    math::{Point, PointExt, Points, Raw, Rect, Scaled, Size, SizeExt, Surround, Vector},
    prelude::Scene,
    shape::{Fill, Shape},
    style::{
        Alignment, FallbackStyle, GenericStyle, Style, StyleComponent, UnscaledFallbackStyle,
        UnscaledStyleComponent,
    },
    text::{prepared::PreparedText, wrap::TextWrap, Span, Text},
    ui::{
        component::render_background,
        control::{ControlBackgroundColor, ControlTextColor},
        Component, Context, ControlEvent, ControlPadding, InteractiveComponent, Layout,
        StyledContext,
    },
    KludgineResult,
};
use async_trait::async_trait;
use euclid::{Length, Scale};

static CURSOR_BLINK_MS: u64 = 500;

#[derive(Debug)]
pub struct TextField {
    paragraphs: Vec<String>,
    prepared: Option<Vec<PreparedText>>,
    cursor: Cursor,
}

#[derive(Debug, Default, Clone, Copy)]
pub struct CursorPosition {
    pub paragraph: usize,
    pub offset: usize,
}

#[derive(Debug, Default)]
pub struct Cursor {
    pub blink_state: BlinkState,
    pub start: CursorPosition,
    pub end: Option<CursorPosition>,
}

#[derive(Debug, Clone)]
pub struct BlinkState {
    pub visible: bool,
    pub change_at: Instant,
}

impl Default for BlinkState {
    fn default() -> Self {
        Self {
            visible: true,
            change_at: Self::next_blink(),
        }
    }
}

impl BlinkState {
    pub fn next_blink() -> Instant {
        let now = Instant::now();
        now.checked_add(Duration::from_millis(CURSOR_BLINK_MS))
            .unwrap_or(now)
    }

    pub fn force_on(&mut self) {
        self.visible = true;
        self.change_at = Self::next_blink();
    }

    pub fn update(&mut self) -> Option<Duration> {
        let now = Instant::now();
        if self.change_at < now {
            self.visible = !self.visible;
            self.change_at = Self::next_blink();
        }

        self.change_at.checked_duration_since(now)
    }
}

// #[derive(Clone, Debug)]
// pub enum LabelCommand {
//     SetValue(String),
// }

#[async_trait]
impl InteractiveComponent for TextField {
    type Command = ();
    type Message = ();
    type Event = ();

    // async fn receive_command(
    //     &mut self,
    //     context: &mut Context,
    //     command: Self::Command,
    // ) -> KludgineResult<()> {
    //     match command {
    //         LabelCommand::SetValue(new_value) => {
    //             if self.value != new_value {
    //                 self.value = new_value;
    //                 context.set_needs_redraw().await;
    //             }
    //         }
    //     }
    //     Ok(())
    // }
}

#[async_trait]
impl Component for TextField {
    async fn update(&mut self, context: &mut Context) -> KludgineResult<()> {
        if context.is_focused().await {
            if let Some(duration) = self.cursor.blink_state.update() {
                context.estimate_next_frame(duration).await;
            } else {
                context.set_needs_redraw().await;
            }
        }
        Ok(())
    }

    async fn render(&mut self, context: &mut StyledContext, layout: &Layout) -> KludgineResult<()> {
        let padding = TextFieldPadding::<Raw>::lookup(context.effective_style())
            .unwrap_or_default()
            .0
            / context.scene().scale_factor().await;

        let bounds = padding.inset_rect(&layout.inner_bounds());
        let mut y = Points::default();
        let prepared = self.prepared_text(context, &bounds.size).await?;
        for paragraph in prepared.iter() {
            y += paragraph
                .render(
                    context.scene(),
                    Point::from_lengths(bounds.origin.x(), bounds.origin.y() + y),
                    true,
                )
                .await?;
        }
        self.prepared = Some(prepared);

        if context.is_focused().await && self.cursor.blink_state.visible {
            if let Some(cursor_location) = self
                .character_rect_for_position(context.scene(), self.cursor.start)
                .await
            {
                Shape::rect(Rect::new(
                    Default::default(),
                    Size::new(1., cursor_location.size.height),
                ))
                .fill(Fill::new(Color::RED))
                .render_at(
                    bounds.origin + cursor_location.origin.to_vector(),
                    context.scene(),
                )
                .await;
            }
        }

        Ok(())
    }

    async fn content_size(
        &self,
        context: &mut StyledContext,
        constraints: &Size<Option<f32>, Scaled>,
    ) -> KludgineResult<Size<f32, Scaled>> {
        let padding = TextFieldPadding::<Raw>::lookup(context.effective_style())
            .unwrap_or_default()
            .0
            / context.scene().scale_factor().await;

        let contraints_minus_padding = padding.inset_constraints(constraints);

        let mut content_size = Size::<f32, Raw>::default();
        for prepared in self
            .prepared_text(
                context,
                &Size::new(
                    contraints_minus_padding.width.unwrap_or_else(|| f32::MAX),
                    contraints_minus_padding.height.unwrap_or_else(|| f32::MAX),
                ),
            )
            .await?
        {
            let size = prepared.size().await;
            content_size.width = content_size.width.max(size.width);
            content_size.height += size.height;
        }
        Ok(content_size / context.scene().scale_factor().await + padding.minimum_size())
    }

    async fn clicked(
        &mut self,
        context: &mut Context,
        window_position: Point<f32, Scaled>,
        button: MouseButton,
    ) -> KludgineResult<()> {
        context.focus().await;
        self.cursor.blink_state.force_on();

        let padding = TextFieldPadding::<Scaled>::lookup(&context.style_sheet().await.normal)
            .unwrap_or_default()
            .0;
        let bounds = padding.inset_rect(&context.last_layout().await.inner_bounds());

        if let Some(location) = dbg!(
            self.position_for_location(
                context.scene(),
                window_position - bounds.origin.to_vector(),
            )
            .await
        ) {
            self.cursor.start = location;
        }

        Ok(())
    }

    async fn render_background(
        &self,
        context: &mut StyledContext,
        layout: &Layout,
    ) -> KludgineResult<()> {
        render_background::<TextFieldBackgroundColor>(context, layout).await
    }
}

impl TextField {
    pub fn new(value: impl ToString) -> Self {
        let value = value.to_string();
        // Normalize the line endings so that \r, \r\n, and \n are all split equally
        let value = value.replace("\r\n", "\n").replace('\r', "\n");
        let paragraphs = value.split('\n').map(|s| s.to_string()).collect();

        Self {
            paragraphs,
            cursor: Default::default(),
            prepared: None,
        }
    }

    fn wrapping(&self, size: &Size<f32, Scaled>, alignment: Alignment) -> TextWrap {
        TextWrap::SingleLine {
            max_width: Points::new(size.width),
            truncate: true,
            alignment,
        }
    }

    async fn prepared_text(
        &self,
        context: &mut StyledContext,
        constraints: &Size<f32, Scaled>,
    ) -> KludgineResult<Vec<PreparedText>> {
        let mut prepared = Vec::new();
        for paragraph in self.paragraphs.iter() {
            let text = Text::span(paragraph, context.effective_style());
            let wrapping = self.wrapping(
                constraints,
                context.effective_style().get_or_default::<Alignment>(),
            );
            prepared.push(text.wrap(context.scene(), wrapping).await?);
        }
        Ok(prepared)
    }

    async fn position_for_location(
        &self,
        scene: &Scene,
        location: Point<f32, Scaled>,
    ) -> Option<CursorPosition> {
        dbg!(location);
        if let Some(prepared) = &self.prepared {
            let mut y = Points::default();
            let scale = scene.scale_factor().await;
            for (paragraph_index, paragraph) in prepared.iter().enumerate() {
                for line in paragraph.lines.iter() {
                    let line_bottom = y + line.size().await.height() / scale;
                    if location.y() < line_bottom {
                        // Click location was within this line
                        for span in line.spans.iter() {
                            let x = dbg!(&span.location).x() / scale;
                            let span_end = x + dbg!(span.data.width) / scale;
                            if !span.data.glyphs.is_empty() && location.x() < span_end {
                                // Click was within this span
                                let relative_pixels = (location.x() - x) * scale;
                                for info in span.data.glyphs.iter() {
                                    if let Some(bounding_box) = info.glyph.pixel_bounding_box() {
                                        if (relative_pixels.get() as i32) < bounding_box.max.x {
                                            return Some(CursorPosition {
                                                paragraph: paragraph_index,
                                                offset: info.source_offset,
                                            });
                                        }
                                    }
                                }
                            }
                        }
                        if let Some(span) = line.spans.last() {
                            if let Some(info) = span.data.glyphs.last() {
                                // Didn't match within the span, put it at the end of the span
                                return Some(CursorPosition {
                                    paragraph: paragraph_index,
                                    offset: info.source_offset + 1,
                                });
                            }
                        }
                    }

                    y = line_bottom;
                }
            }
        }

        None
    }

    async fn character_rect_for_position(
        &self,
        scene: &Scene,
        position: CursorPosition,
    ) -> Option<Rect<f32, Scaled>> {
        let mut last_location = None;
        if let Some(prepared) = &self.prepared {
            let prepared = prepared.get(position.paragraph)?;
            let scale = scene.scale_factor().await;
            let mut location: Point<f32, Scaled> = Point::default();
            for line in prepared.lines.iter() {
                location.y = (location.y() + line.metrics.ascent / scale).get();
                location.x = line.alignment_offset.get();
                for span in line.spans.iter() {
                    dbg!(&span.data.glyphs);
                    let next_x = location.x() + (span.data.width / scale);
                    if !span.data.glyphs.is_empty() {
                        let last_glyph = span.data.glyphs.last().unwrap();
                        if dbg!(position.offset) <= dbg!(last_glyph.source_offset) {
                            // Return a box of the width of the last character with the start of the character at the origin
                            for info in span.data.glyphs.iter() {
                                if info.source_offset >= position.offset {
                                    if let Some(bounding_box) = info.glyph.pixel_bounding_box() {
                                        return Some(Rect::new(
                                            Point::from_lengths(
                                                (span.location.x()
                                                    + Length::<i32, Raw>::new(bounding_box.min.x)
                                                        .cast::<f32>())
                                                    / scale,
                                                span.location.y() / scale,
                                            ),
                                            Size::from_lengths(
                                                Length::<i32, Raw>::new(
                                                    bounding_box.max.x - bounding_box.min.x,
                                                )
                                                .cast::<f32>(),
                                                Length::<f32, Raw>::new(span.data.metrics.ascent),
                                            ) / scale,
                                        ));
                                    } else {
                                        panic!("Unsure if this is reachable or not. A glphy didn't have a bounding box")
                                    }
                                }
                            }
                        }

                        if let Some(bounding_box) = last_glyph.glyph.pixel_bounding_box() {
                            last_location = Some(Rect::new(
                                Point::from_lengths(
                                    (span.location.x()
                                        + Length::<i32, Raw>::new(bounding_box.max.x)
                                            .cast::<f32>())
                                        / scale,
                                    span.location.y() / scale,
                                ),
                                Size::from_lengths(
                                    Default::default(),
                                    Length::<f32, Raw>::new(span.data.metrics.ascent) / scale,
                                ),
                            ));
                        }
                    }
                    location.x = next_x.get();
                }
            }
        }
        last_location
    }
}

#[derive(Debug, Clone, Default)]
pub struct TextFieldBackgroundColor(pub Color);
impl UnscaledStyleComponent<Scaled> for TextFieldBackgroundColor {}

impl UnscaledFallbackStyle for TextFieldBackgroundColor {
    fn lookup_unscaled(style: GenericStyle) -> Option<Self> {
        style.get::<Self>().cloned().or_else(|| {
            ControlBackgroundColor::lookup_unscaled(style).map(|fg| TextFieldBackgroundColor(fg.0))
        })
    }
}

impl Into<Color> for TextFieldBackgroundColor {
    fn into(self) -> Color {
        self.0
    }
}

#[derive(Debug, Clone, Default)]
pub struct TextFieldTextColor(pub Color);
impl UnscaledStyleComponent<Scaled> for TextFieldTextColor {}

impl UnscaledFallbackStyle for TextFieldTextColor {
    fn lookup_unscaled(style: GenericStyle) -> Option<Self> {
        style
            .get::<Self>()
            .cloned()
            .or_else(|| ControlTextColor::lookup_unscaled(style).map(|fg| TextFieldTextColor(fg.0)))
    }
}

impl Into<Color> for TextFieldTextColor {
    fn into(self) -> Color {
        self.0
    }
}

#[derive(Debug, Clone, Default)]
pub struct TextFieldPadding<Unit>(pub Surround<f32, Unit>);

impl StyleComponent<Scaled> for TextFieldPadding<Scaled> {
    fn scale(&self, scale: Scale<f32, Scaled, Raw>, destination: &mut Style<Raw>) {
        destination.push(TextFieldPadding(self.0 * scale))
    }
}

impl StyleComponent<Raw> for TextFieldPadding<Raw> {
    fn scale(&self, _scale: Scale<f32, Raw, Raw>, map: &mut Style<Raw>) {
        map.push(TextFieldPadding(self.0));
    }
}

impl FallbackStyle<Scaled> for TextFieldPadding<Scaled> {
    fn lookup(style: &Style<Scaled>) -> Option<Self> {
        style
            .get::<Self>()
            .cloned()
            .or_else(|| ControlPadding::<Scaled>::lookup(style).map(|cp| TextFieldPadding(cp.0)))
    }
}

impl FallbackStyle<Raw> for TextFieldPadding<Raw> {
    fn lookup(style: &Style<Raw>) -> Option<Self> {
        style
            .get::<Self>()
            .cloned()
            .or_else(|| ControlPadding::<Raw>::lookup(style).map(|cp| TextFieldPadding(cp.0)))
    }
}
