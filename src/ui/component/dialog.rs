use crate::{
    color::Color,
    math::{Dimension, Point, Points, Raw, Scaled, Size, SizeExt},
    shape::{Fill, Shape},
    style::{theme::Selector, Alignment, ColorPair, Style, StyleComponent, UnscaledStyleComponent},
    ui::{
        component::{
            pending::PendingComponent, Button, Component, ControlEvent, InteractiveComponent,
            InteractiveComponentExt, Label,
        },
        AbsoluteBounds, AbsoluteLayout, Context, Entity, Indexable, Layout, LayoutSolver,
        LayoutSolverExt, StyledContext,
    },
    KludgineResult,
};
use async_trait::async_trait;
use euclid::{Length, Scale};
use generational_arena::Index;
use std::fmt::Debug;
use winit::event::{ElementState, ScanCode, VirtualKeyCode};

#[derive(Default, Debug)]
struct ButtonBarLayout {
    left: Vec<ButtonLayout>,
    middle: Vec<ButtonLayout>,
    right: Vec<ButtonLayout>,
}

impl ButtonBarLayout {
    pub fn height(&self) -> Points {
        self.left
            .iter()
            .chain(self.middle.iter())
            .chain(self.right.iter())
            .next()
            .unwrap()
            .size
            .height()
    }

    pub fn section_width(&self, which_section: Alignment, button_spacing: Points) -> Points {
        let section = match which_section {
            Alignment::Left => &self.left,
            Alignment::Center => &self.middle,
            Alignment::Right => &self.right,
        };
        let total_widths = section.iter().map(|bl| bl.size.width().get()).sum();

        let spacing = if section.len() > 1 {
            button_spacing * (section.len() - 1) as f32
        } else {
            Points::default()
        };
        spacing + Points::new(total_widths)
    }

    pub fn width(&self, button_spacing: Points) -> Points {
        let mut sections = Vec::new();

        if !self.left.is_empty() {
            sections.push(self.section_width(Alignment::Left, button_spacing).get());
        }
        if !self.middle.is_empty() {
            sections.push(self.section_width(Alignment::Center, button_spacing).get());
        }
        if !self.right.is_empty() {
            sections.push(self.section_width(Alignment::Right, button_spacing).get());
        }

        let spacing = if sections.len() > 1 {
            button_spacing * (sections.len() - 1) as f32
        } else {
            Points::default()
        };

        spacing + Points::new(sections.iter().sum())
    }

    pub fn size(&self, button_spacing: Points) -> Size<f32, Scaled> {
        Size::from_lengths(self.width(button_spacing), self.height())
    }
}

#[derive(Debug)]
struct ButtonLayout {
    index: Index,
    size: Size<f32, Scaled>,
}

#[derive(Debug)]
struct DialogButtonInstance<T> {
    entity: Entity<Button>,
    value: Option<T>,
    primary: bool,
    cancel: bool,
}

pub struct Dialog<C, T = ()>
where
    C: InteractiveComponent,
{
    contents: PendingComponent<C>,
    left_buttons: Vec<DialogButtonInstance<T>>,
    middle_buttons: Vec<DialogButtonInstance<T>>,
    right_buttons: Vec<DialogButtonInstance<T>>,
    autodismiss_value: Option<Option<T>>,
    _phantom: std::marker::PhantomData<T>,
}

impl<C, T> Dialog<C, T>
where
    C: InteractiveComponent + 'static,
    T: Clone + Debug + Send + Sync + 'static,
{
    pub fn new(contents: C) -> Self {
        Self {
            contents: PendingComponent::Pending(contents),
            left_buttons: Vec::new(),
            middle_buttons: Vec::new(),
            right_buttons: Vec::new(),
            autodismiss_value: Default::default(),
            _phantom: Default::default(),
        }
    }

    pub async fn open(self, context: &mut Context) -> KludgineResult<Entity<Self>> {
        context.new_layer(self).insert().await
    }

    async fn measure_buttons(
        &self,
        context: &mut StyledContext,
    ) -> KludgineResult<ButtonBarLayout> {
        let mut layout = ButtonBarLayout::default();
        for (button, alignment) in self.all_buttons() {
            let (content_size, padding) = context
                .content_size_with_padding(&button.entity, &Size::new(None, None))
                .await?;

            let button = ButtonLayout {
                index: button.entity.index(),
                size: content_size + padding.minimum_size(),
            };

            match alignment {
                Alignment::Left => layout.left.push(button),
                Alignment::Center => layout.middle.push(button),
                Alignment::Right => layout.right.push(button),
            }
        }
        Ok(layout)
    }

    fn all_buttons(&self) -> impl Iterator<Item = (&DialogButtonInstance<T>, Alignment)> {
        self.left_buttons
            .iter()
            .map(|b| (b, Alignment::Left))
            .chain(self.middle_buttons.iter().map(|b| (b, Alignment::Center)))
            .chain(self.right_buttons.iter().map(|b| (b, Alignment::Right)))
    }

    async fn button_clicked(&self, value: Option<T>, context: &mut Context) {
        self.callback(context, value).await;

        // Close the dialog once any button has been pressed
        context.remove(&context.index()).await;
    }

    async fn button_spacing(&self, context: &mut StyledContext) -> Length<f32, Raw> {
        if let Some(spacing) = context.effective_style().get::<DialogButtonSpacing<Raw>>() {
            spacing.0
        } else {
            Points::new(10.) * context.scene().scale_factor().await
        }
    }
}

impl<T> Dialog<Label, T>
where
    T: Clone + Debug + Send + Sync + 'static,
{
    pub fn text<S: ToString>(contents: S) -> Self {
        Self::new(Label::new(contents))
    }
}

#[async_trait]
impl<C, T> Component for Dialog<C, T>
where
    C: InteractiveComponent + 'static,
    T: Clone + Debug + Send + Sync + 'static,
{
    fn classes(&self) -> Option<Vec<Selector>> {
        Some(vec![Selector::from("dialog")])
    }

    async fn initialize(&mut self, context: &mut Context) -> KludgineResult<()> {
        if let PendingComponent::Pending(contents) = std::mem::replace(
            &mut self.contents,
            PendingComponent::Entity(Default::default()),
        ) {
            self.contents =
                PendingComponent::Entity(self.new_entity(context, contents).await.insert().await?);
        } else {
            unreachable!("A component should never be re-initialized");
        }

        let buttons = self.component::<DialogButtons<T>>(context).await;
        let buttons = if let Some(buttons) = buttons {
            let buttons = buttons.read().await;
            buttons.clone()
        } else {
            DialogButtons(vec![DialogButton::default()
                .caption("Ok")
                .primary()
                .cancel()])
        };

        let autodismiss_value = self.component::<DialogAutodismiss<T>>(context).await;
        if let Some(autodismiss_value) = autodismiss_value {
            let autodismiss_value = autodismiss_value.read().await;
            self.autodismiss_value = Some(autodismiss_value.0.clone());
        };

        for DialogButton {
            caption,
            class,
            value,
            primary,
            cancel,
            alignment,
        } in buttons.0
        {
            let value_for_callback = value.clone();
            let mut button = self
                .new_entity(context, Button::new(caption))
                .await
                .callback(&self.entity(context), move |evt| {
                    let ControlEvent::Clicked { .. } = evt;
                    DialogMessage::ButtonClicked(value_for_callback.clone())
                });

            if let Some(class) = class {
                button = button.with_class(class).await;
            } else if primary {
                button = button.with_class("is-primary").await;
            } else if cancel {
                button = button.with_class("is-cancel").await;
            }

            let button = DialogButtonInstance {
                entity: button.insert().await?,
                value,
                primary,
                cancel,
            };
            match alignment {
                Alignment::Left => self.left_buttons.push(button),
                Alignment::Center => self.middle_buttons.push(button),
                Alignment::Right => self.right_buttons.push(button),
            }
        }

        Ok(())
    }

    async fn content_size(
        &self,
        context: &mut StyledContext,
        constraints: &Size<Option<f32>, Scaled>,
    ) -> KludgineResult<Size<f32, Scaled>> {
        let (content_size, padding) = context
            .content_size_with_padding(&self.contents.entity(), &constraints)
            .await?;
        let button_bar_layout = self.measure_buttons(context).await?;
        let scale_factor = context.scene().scale_factor().await;
        let button_spacing = self.button_spacing(context).await / scale_factor;
        let spacing_between_contents_and_buttons =
            Size::from_lengths(Default::default(), button_spacing);
        Ok(content_size
            + padding.minimum_size()
            + button_bar_layout.size(button_spacing)
            + spacing_between_contents_and_buttons)
    }

    async fn layout(
        &mut self,
        context: &mut StyledContext,
    ) -> KludgineResult<Box<dyn LayoutSolver>> {
        let button_bar_layout = self.measure_buttons(context).await?;
        let scale_factor = context.scene().scale_factor().await;
        let button_spacing = self.button_spacing(context).await / scale_factor;

        let mut layout = AbsoluteLayout::default().child(
            &self.contents.entity(),
            AbsoluteBounds {
                left: Dimension::from_f32(0.),
                top: Dimension::from_f32(0.),
                right: Dimension::from_f32(0.),
                bottom: Dimension::Length(button_bar_layout.height()),
                ..Default::default()
            },
        )?;

        let mut x = Points::default();
        for button_layout in &button_bar_layout.left {
            layout = layout.child(
                &button_layout.index,
                AbsoluteBounds {
                    bottom: Dimension::from_f32(0.),
                    left: Dimension::Length(x),
                    ..Default::default()
                },
            )?;
            x += button_layout.size.width() + button_spacing;
        }

        let mut x = (context.scene().size().await.width()
            - button_bar_layout.section_width(Alignment::Center, button_spacing))
            / 2.;
        for button_layout in &button_bar_layout.middle {
            layout = layout.child(
                &button_layout.index,
                AbsoluteBounds {
                    bottom: Dimension::from_f32(0.),
                    left: Dimension::Length(x),
                    ..Default::default()
                },
            )?;
            x += button_layout.size.width() + button_spacing;
        }

        let mut x = Points::default();
        for button_layout in &button_bar_layout.right {
            layout = layout.child(
                &button_layout.index,
                AbsoluteBounds {
                    bottom: Dimension::from_f32(0.),
                    right: Dimension::Length(x),
                    ..Default::default()
                },
            )?;
            x += button_layout.size.width() + button_spacing;
        }

        layout.layout()
    }

    async fn render_background(
        &self,
        context: &mut StyledContext,
        layout: &Layout,
    ) -> KludgineResult<()> {
        let background = context
            .effective_style()
            .get_or_default::<DialogOverlayColor>();
        let color_pair = background.0;
        let color = color_pair.themed_color(&context.scene().system_theme().await);

        if color.visible() {
            Shape::rect(layout.bounds)
                .fill(Fill::new(color))
                .render_at(Point::default(), context.scene())
                .await;
        }

        self.render_standard_background(context, layout).await
    }

    // Dialogs intercept all clicks while they're open
    async fn hit_test(
        &self,
        context: &mut Context,
        _window_position: Point<f32, Scaled>,
    ) -> KludgineResult<bool> {
        if let Some(value) = &self.autodismiss_value {
            self.button_clicked(value.clone(), context).await;
        }

        Ok(true)
    }

    async fn keyboard_event(
        &mut self,
        context: &mut Context,
        _scancode: ScanCode,
        key: Option<VirtualKeyCode>,
        state: ElementState,
    ) -> KludgineResult<()> {
        if let Some(key) = key {
            let modifiers = context.scene().modifiers_pressed().await;
            let button = if key == VirtualKeyCode::Escape
                || key == VirtualKeyCode::Period && modifiers.command_key()
            {
                self.all_buttons().find(|(b, _)| b.cancel)
            } else if key == VirtualKeyCode::Return || key == VirtualKeyCode::NumpadEnter {
                self.all_buttons().find(|(b, _)| b.primary)
            } else {
                None
            };

            if let Some((button, _)) = button {
                if state == ElementState::Pressed {
                    context.activate(button.entity.clone()).await;
                } else {
                    context.deactivate(button.entity.clone()).await;
                    self.button_clicked(button.value.clone(), context).await;
                }
            }
        }

        Ok(())
    }
}

#[derive(Debug, Clone)]
pub enum DialogMessage<T> {
    ButtonClicked(Option<T>),
}

#[async_trait]
impl<C, T> InteractiveComponent for Dialog<C, T>
where
    C: InteractiveComponent + 'static,
    T: Clone + Debug + Send + Sync + 'static,
{
    type Message = DialogMessage<T>;
    type Command = ();
    type Event = Option<T>;

    async fn receive_message(
        &mut self,
        context: &mut Context,
        message: Self::Message,
    ) -> KludgineResult<()> {
        let DialogMessage::ButtonClicked(value) = message;
        self.button_clicked(value, context).await;

        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct DialogButton<T> {
    pub caption: String,
    pub class: Option<Selector>,
    pub value: Option<T>,
    pub primary: bool,
    pub cancel: bool,
    pub alignment: Alignment,
}

#[derive(Debug, Clone)]
pub struct DialogButtons<T>(pub Vec<DialogButton<T>>);

impl<T> Default for DialogButton<T> {
    fn default() -> Self {
        DialogButton {
            caption: String::default(),
            class: None,
            value: None,
            alignment: Alignment::Right,
            primary: false,
            cancel: false,
        }
    }
}

impl<T> DialogButton<T> {
    pub fn caption<S: ToString>(mut self, caption: S) -> Self {
        self.caption = caption.to_string();
        self
    }

    pub fn class<C: Into<Selector>>(mut self, class: C) -> Self {
        self.class = Some(class.into());
        self
    }

    pub fn value(mut self, value: T) -> Self {
        self.value = Some(value);
        self
    }

    pub fn alignment(mut self, alignment: Alignment) -> Self {
        self.alignment = alignment;
        self
    }

    pub fn primary(mut self) -> Self {
        self.primary = true;
        self
    }

    pub fn cancel(mut self) -> Self {
        self.cancel = true;
        self
    }
}

#[derive(Debug, Clone, Copy)]
pub struct DialogAutodismiss<T>(pub Option<T>);

#[derive(Debug, Clone, Copy)]
pub struct DialogButtonSpacing<Unit>(pub Length<f32, Unit>);

impl StyleComponent<Scaled> for DialogButtonSpacing<Scaled> {
    fn scale(&self, scale: Scale<f32, Scaled, Raw>, destination: &mut Style<Raw>) {
        destination.push(DialogButtonSpacing(self.0 * scale))
    }

    fn should_be_inherited(&self) -> bool {
        false
    }
}

impl StyleComponent<Raw> for DialogButtonSpacing<Raw> {
    fn scale(&self, _scale: Scale<f32, Raw, Raw>, map: &mut Style<Raw>) {
        map.push(DialogButtonSpacing(self.0));
    }

    fn should_be_inherited(&self) -> bool {
        false
    }
}

#[derive(Debug, Clone)]
pub struct DialogOverlayColor(pub ColorPair);
impl UnscaledStyleComponent<Scaled> for DialogOverlayColor {
    fn unscaled_should_be_inherited(&self) -> bool {
        false
    }
}

impl Default for DialogOverlayColor {
    fn default() -> Self {
        DialogOverlayColor(ColorPair {
            light_color: Color::new(1., 1., 1., 0.7),
            dark_color: Color::new(0., 0., 0., 0.7),
        })
    }
}

impl Into<ColorPair> for DialogOverlayColor {
    fn into(self) -> ColorPair {
        self.0
    }
}
