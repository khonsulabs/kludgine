use crate::{
    color::Color,
    math::{Pixels, Point, PointExt, Points, Raw, Rect, Scaled, Size, SizeExt},
    scene::Target,
    shape::{Fill, Shape},
    style::{theme::Selector, Alignment},
    text::{
        prepared::PreparedText,
        rich::{RichText, RichTextPosition},
        wrap::TextWrap,
    },
    ui::{
        component::ComponentPadding, Component, Context, InteractiveComponent, Layout,
        StyledContext,
    },
    window::event::{EventStatus, MouseButton},
    KludgineError, KludgineResult,
};
use async_trait::async_trait;
use clipboard::{ClipboardContext, ClipboardProvider};
use std::time::{Duration, Instant};
use winit::event::{ElementState, ScanCode, VirtualKeyCode};

static CURSOR_BLINK_MS: u64 = 500;

#[derive(Debug)]
pub struct TextField {
    text: RichText,
    prepared: Option<Vec<PreparedText>>,
    cursor: Cursor,
}

#[derive(Debug, Clone)]
pub enum TextFieldEvent {
    ValueChanged(RichText),
    SelectionChanged {
        start: RichTextPosition,
        end: Option<RichTextPosition>,
    },
}

#[derive(Debug, Default)]
pub struct Cursor {
    pub blink_state: BlinkState,
    pub start: RichTextPosition,
    pub end: Option<RichTextPosition>,
}

impl Cursor {
    pub fn selection_start(&self) -> RichTextPosition {
        if let Some(end) = self.end {
            self.start.min(end)
        } else {
            self.start
        }
    }

    pub fn selection_end(&self) -> RichTextPosition {
        if let Some(end) = self.end {
            self.start.max(end)
        } else {
            self.start
        }
    }
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
    type Event = TextFieldEvent;

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
    fn classes(&self) -> Option<Vec<Selector>> {
        Some(vec![
            Selector::from("text"),
            Selector::from("input"),
            Selector::from("control"),
        ])
    }

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
        let scale = context.scene().scale_factor().await;
        let padding = context
            .effective_style()
            .get_or_default::<ComponentPadding<Raw>>()
            .0
            / scale;

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

        if let Some(end) = self.cursor.end {
            let selection_start = self.cursor.start.min(end);
            let selection_end = self.cursor.start.max(end);
            if let Some(start_position) = self
                .character_rect_for_position(context.scene(), selection_start)
                .await
            {
                if let Some(end_position) = self
                    .character_rect_for_position(context.scene(), selection_end)
                    .await
                {
                    if start_position.max_y() <= end_position.min_y() {
                        // Multi-line
                        // First line is start_position -> end of bounds
                        let mut area = start_position;
                        area.size.width = bounds.size.width - start_position.origin.x;
                        Shape::rect(area)
                            .fill(Fill::new(Color::new(1., 0., 0., 0.3)))
                            .render_at(bounds.origin, context.scene())
                            .await;
                        if start_position.max_y() < end_position.min_y() {
                            // Draw a solid block for all the inner lines
                            Shape::rect(Rect::new(
                                Point::new(0., start_position.max_y()),
                                Size::new(
                                    bounds.size.width,
                                    end_position.min_y() - start_position.max_y(),
                                ),
                            ))
                            .fill(Fill::new(Color::new(1., 0., 0., 0.3)))
                            .render_at(bounds.origin, context.scene())
                            .await;
                        }
                        // Last line is start of line -> start of end position
                        Shape::rect(Rect::new(
                            Point::new(0., end_position.min_y()),
                            Size::new(end_position.origin.x, end_position.size.height),
                        ))
                        .fill(Fill::new(Color::new(1., 0., 0., 0.3)))
                        .render_at(bounds.origin, context.scene())
                        .await;
                    } else {
                        // Single-line
                        let mut area = start_position;
                        area.size.width = end_position.origin.x - start_position.origin.x;
                        Shape::rect(area)
                            .fill(Fill::new(Color::new(1., 0., 0., 0.3)))
                            .render_at(bounds.origin, context.scene())
                            .await;
                    }
                }
            }
        } else if context.is_focused().await && self.cursor.blink_state.visible {
            if let Some(cursor_location) = self
                .character_rect_for_position(context.scene(), self.cursor.start)
                .await
            {
                // No selection, draw a caret
                Shape::rect(Rect::new(
                    Default::default(),
                    Size::from_lengths(Pixels::new(1.) / scale, cursor_location.size.height()),
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
        let scale = context.scene().scale_factor().await;
        let padding = context
            .effective_style()
            .get_or_default::<ComponentPadding<Raw>>()
            .0
            / scale;

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
            let mut size = prepared.size().await;
            if approx::relative_eq!(size.height, 0.) && !prepared.lines.is_empty() {
                size.set_height(prepared.lines[0].metrics.height());
            }
            content_size.width = content_size.width.max(size.width);
            content_size.height += size.height;
        }
        Ok(content_size / scale + padding.minimum_size())
    }

    async fn mouse_down(
        &mut self,
        context: &mut Context,
        window_position: Point<f32, Scaled>,
        button: MouseButton,
    ) -> KludgineResult<EventStatus> {
        if button == MouseButton::Left {
            context.focus().await;
            self.cursor.blink_state.force_on();

            let padding = context
                .style_sheet()
                .await
                .normal
                .get_or_default::<ComponentPadding<Scaled>>()
                .0;
            let bounds = padding.inset_rect(&self.last_layout(context).await.inner_bounds());

            if let Some(location) = self
                .position_for_location(context.scene(), window_position - bounds.origin.to_vector())
                .await
            {
                self.cursor.start = location;
                self.cursor.end = None;
            }

            context.set_needs_redraw().await;

            Ok(EventStatus::Processed)
        } else {
            Ok(EventStatus::Ignored)
        }
    }

    async fn mouse_drag(
        &mut self,
        context: &mut Context,
        window_position: Option<Point<f32, Scaled>>,
        button: MouseButton,
    ) -> KludgineResult<()> {
        if button == MouseButton::Left {
            self.cursor.blink_state.force_on();
            if let Some(window_position) = window_position {
                let padding = context
                    .style_sheet()
                    .await
                    .normal
                    .get_or_default::<ComponentPadding<Scaled>>()
                    .0;
                let bounds = padding.inset_rect(&self.last_layout(context).await.inner_bounds());

                if let Some(location) = self
                    .position_for_location(
                        context.scene(),
                        window_position - bounds.origin.to_vector(),
                    )
                    .await
                {
                    if location == self.cursor.start {
                        if self.cursor.end != None {
                            self.cursor.end = None;
                            context.set_needs_redraw().await;
                        }
                    } else if self.cursor.end != Some(location) {
                        self.cursor.end = Some(location);
                        context.set_needs_redraw().await;
                    }
                }
            } else if self.cursor.end != None {
                self.cursor.end = None;
                context.set_needs_redraw().await;
            }
        }

        Ok(())
    }

    async fn receive_character(
        &mut self,
        context: &mut Context,
        character: char,
    ) -> KludgineResult<()> {
        match character {
            '\x08' => {
                if self.cursor.end.is_none() && self.cursor.start.offset > 0 {
                    // Select the previous character
                    self.cursor.end = Some(self.cursor.start);
                    self.cursor.start = self.text.position_before(self.cursor.start).await;
                }

                self.replace_selection("", context).await
            }
            character => {
                if !character.is_control() {
                    self.replace_selection(&character.to_string(), context)
                        .await
                }
            }
        }
        Ok(())
    }

    async fn keyboard_event(
        &mut self,
        context: &mut Context,
        _scancode: ScanCode,
        key: Option<VirtualKeyCode>,
        state: ElementState,
    ) -> KludgineResult<()> {
        if let Some(key) = key {
            if matches!(state, ElementState::Pressed) {
                // TODO handle modifiers
                match key {
                    VirtualKeyCode::Left => {
                        self.set_selection(
                            context,
                            self.text
                                .position_before(self.cursor.selection_start())
                                .await,
                            None,
                        )
                        .await;
                    }
                    VirtualKeyCode::Right => {
                        self.set_selection(
                            context,
                            self.text
                                .position_after(self.cursor.selection_start())
                                .await,
                            None,
                        )
                        .await;
                    }
                    VirtualKeyCode::Up => {}
                    VirtualKeyCode::Down => {}
                    VirtualKeyCode::A => {
                        if context.scene().modifiers_pressed().await.primary_modifier() {
                            self.set_selection(
                                context,
                                Default::default(),
                                Some(self.text.end().await),
                            )
                            .await;
                        }
                    }
                    VirtualKeyCode::V => {
                        if context.scene().modifiers_pressed().await.primary_modifier() {
                            let mut clipboard = ClipboardContext::new()
                                .map_err(|err| KludgineError::Clipboard(err.to_string()))?;

                            // Convert Result to Option to get rid of the Box<dyn Error> before the await
                            let pasted = clipboard.get_contents().ok();
                            if let Some(pasted) = pasted {
                                self.replace_selection(&pasted, context).await;
                            }
                        }
                    }
                    VirtualKeyCode::X | VirtualKeyCode::C => {
                        if context.scene().modifiers_pressed().await.primary_modifier() {
                            let mut clipboard = ClipboardContext::new()
                                .map_err(|err| KludgineError::Clipboard(err.to_string()))?;

                            let selected = self.selected_string().await;
                            if key == VirtualKeyCode::X {
                                self.replace_selection("", context).await;
                            }

                            let _ = clipboard.set_contents(selected);
                        }
                    }
                    _ => {}
                }

                self.cursor.blink_state.force_on();
                context.set_needs_redraw().await;
            }
        }
        Ok(())
    }
}

impl TextField {
    pub fn new(initial_text: RichText) -> Self {
        Self {
            text: initial_text,
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

    pub async fn replace_selection(&mut self, replacement: &str, context: &mut Context) {
        if self.cursor.end.is_some() {
            let selection_start = self.cursor.selection_start();
            let selection_end = self.cursor.selection_end();
            self.text.remove_range(selection_start..selection_end).await;
            self.cursor.end = None;
            self.cursor.start = selection_start;
        }

        self.text.insert_str(self.cursor.start, replacement).await;
        self.cursor.start.offset += replacement.len();
        self.cursor.blink_state.force_on();

        self.notify_changed(context).await;
        self.notify_selection_changed(context).await;
        context.set_needs_redraw().await;
    }

    pub async fn selected_string(&self) -> String {
        let mut copied_paragraphs = Vec::new();
        self.text
            .for_each_in_range(
                self.cursor.selection_start()..self.cursor.selection_end(),
                |paragraph, relative_range| {
                    let mut span_strings = Vec::new();
                    paragraph.for_each_in_range(relative_range, |span, relative_range| {
                        span_strings.push(span.text[relative_range].to_string());
                    });
                    copied_paragraphs.push(span_strings.join(""));
                },
            )
            .await;
        copied_paragraphs.join("\n")
    }

    async fn prepared_text(
        &self,
        context: &mut StyledContext,
        constraints: &Size<f32, Scaled>,
    ) -> KludgineResult<Vec<PreparedText>> {
        self.text
            .prepare(
                context,
                self.wrapping(
                    constraints,
                    context.effective_style().get_or_default::<Alignment>(),
                ),
            )
            .await
    }

    async fn position_for_location(
        &self,
        scene: &Target,
        location: Point<f32, Scaled>,
    ) -> Option<RichTextPosition> {
        if let Some(prepared) = &self.prepared {
            let mut y = Points::default();
            let scale = scene.scale_factor().await;
            for (paragraph_index, paragraph) in prepared.iter().enumerate() {
                for line in paragraph.lines.iter() {
                    let line_bottom = y + line.size().await.height() / scale;
                    if location.y() < line_bottom {
                        // Click location was within this line
                        for span in line.spans.iter() {
                            let x = span.location.x() / scale;
                            let span_end = x + span.data.width / scale;
                            if !span.data.glyphs.is_empty() && location.x() < span_end {
                                // Click was within this span
                                let relative_pixels = (location.x() - x) * scale;
                                for info in span.data.glyphs.iter() {
                                    if relative_pixels <= info.location().x() + info.width() {
                                        return Some(RichTextPosition {
                                            paragraph: paragraph_index,
                                            offset: info.source_offset,
                                        });
                                    }
                                }

                                return Some(RichTextPosition {
                                    paragraph: paragraph_index,
                                    offset: span.data.glyphs.last().unwrap().source_offset,
                                });
                            }
                        }
                        if let Some(span) = line.spans.last() {
                            if let Some(info) = span.data.glyphs.last() {
                                // Didn't match within the span, put it at the end of the span
                                return Some(RichTextPosition {
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
        scene: &Target,
        position: RichTextPosition,
    ) -> Option<Rect<f32, Scaled>> {
        let mut last_location = None;
        if let Some(prepared) = &self.prepared {
            let prepared = prepared.get(position.paragraph)?;
            let scale = scene.scale_factor().await;
            let mut line_top = Points::default();
            for line in prepared.lines.iter() {
                let line_height = line.height() / scale;
                for span in line.spans.iter() {
                    if !span.data.glyphs.is_empty() {
                        let last_glyph = span.data.glyphs.last().unwrap();
                        if position.offset <= last_glyph.source_offset {
                            // Return a box of the width of the last character with the start of the character at the origin
                            for info in span.data.glyphs.iter() {
                                if info.source_offset >= position.offset {
                                    return Some(Rect::new(
                                        Point::from_lengths(
                                            (span.location.x() + info.location().x()) / scale,
                                            line_top + span.location.y() / scale,
                                        ),
                                        Size::from_lengths(info.width() / scale, line_height),
                                    ));
                                }
                            }
                        }

                        last_location = Some(Rect::new(
                            Point::from_lengths(
                                (span.location.x()
                                    + last_glyph.location().x()
                                    + last_glyph.width())
                                    / scale,
                                line_top + span.location.y() / scale,
                            ),
                            Size::from_lengths(Default::default(), line_height),
                        ));
                    }
                }
                line_top += line_height;
            }
        }
        last_location
    }

    async fn notify_changed(&self, context: &mut Context) {
        self.callback(context, TextFieldEvent::ValueChanged(self.text.clone()))
            .await;
    }

    async fn notify_selection_changed(&self, context: &mut Context) {
        self.callback(
            context,
            TextFieldEvent::SelectionChanged {
                start: self.cursor.start,
                end: self.cursor.end,
            },
        )
        .await;
    }

    pub async fn set_selection(
        &mut self,
        context: &mut Context,
        selection_start: RichTextPosition,
        end: Option<RichTextPosition>,
    ) {
        self.cursor.start = selection_start;
        self.cursor.end = end;
        self.notify_selection_changed(context).await;
    }
}
