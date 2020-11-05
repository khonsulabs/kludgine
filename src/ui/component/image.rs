use crate::{
    math::{Point, Rect, Scaled, Size},
    sprite::{Sprite, SpriteRotation, SpriteSource},
    style::theme::Selector,
    ui::{
        animation::{FloatChange, PropertyFrameManager, PropertyMutator, Transition},
        AnimatableComponent, Component, Context, ControlEvent, Entity, InteractiveComponent,
        Layout, StyledContext,
    },
    window::event::MouseButton,
    KludgineResult,
};
use async_trait::async_trait;
use std::sync::Arc;

#[derive(Debug)]
pub struct Image {
    sprite: Sprite,
    current_frame: Option<SpriteSource>,
    options: ImageOptions,
}

#[derive(Debug)]
pub enum ImageScaling {
    AspectFit,
    AspectFill,
    Fill,
}

#[derive(Debug)]
pub struct ImageOptions {
    pub scaling: Option<ImageScaling>,
    pub override_frame: Option<OverrideFrame>,
    pub alpha: f32,
}

#[derive(Debug, Clone)]
pub enum OverrideFrame {
    Index(usize),
    Percent(f32),
}

impl Default for ImageOptions {
    fn default() -> Self {
        Self {
            scaling: None,
            alpha: 1.,
            override_frame: None,
        }
    }
}

impl ImageOptions {
    pub fn scaling(mut self, scaling: ImageScaling) -> Self {
        self.scaling = Some(scaling);
        self
    }

    pub fn alpha(mut self, alpha: f32) -> Self {
        self.alpha = alpha;
        self
    }
}

#[derive(Debug, Clone)]
pub enum ImageCommand {
    SetSprite(Sprite),
    SetTag(Option<String>),
    SetAlpha(f32),
    SetOverrideFrame {
        tag: Option<String>,
        frame: OverrideFrame,
    },
}

#[async_trait]
impl InteractiveComponent for Image {
    type Message = ();
    type Command = ImageCommand;
    type Event = ControlEvent;

    async fn receive_command(
        &mut self,
        context: &mut Context,
        command: Self::Command,
    ) -> KludgineResult<()> {
        match command {
            ImageCommand::SetSprite(sprite) => {
                context.set_needs_redraw().await;
                self.sprite = sprite;
            }
            ImageCommand::SetTag(tag) => {
                context.set_needs_redraw().await;
                self.sprite.set_current_tag(tag).await?;
                self.options.override_frame = None;
            }
            ImageCommand::SetAlpha(alpha) => {
                context.set_needs_redraw().await;
                self.options.alpha = alpha;
            }
            ImageCommand::SetOverrideFrame { tag, frame } => {
                context.set_needs_redraw().await;
                self.sprite.set_current_tag(tag).await?;
                self.options.override_frame = Some(frame);
            }
        }
        Ok(())
    }
}

#[async_trait]
impl Component for Image {
    fn classes(&self) -> Option<Vec<Selector>> {
        Some(vec![Selector::from("image"), Selector::from("control")])
    }

    async fn update(&mut self, context: &mut Context) -> KludgineResult<()> {
        self.current_frame = match &self.options.override_frame {
            Some(override_frame) => {
                let current_tag = self.sprite.current_tag().await;
                let frames = self
                    .sprite
                    .animations()
                    .await
                    .frames_for(&current_tag)
                    .await
                    .unwrap();

                let frame_index = match override_frame {
                    OverrideFrame::Index(index) => *index,
                    OverrideFrame::Percent(percent) => ((frames.frames.len() as f32 * percent)
                        .round() as usize)
                        .min(frames.frames.len() - 1),
                };

                Some(frames.frames[frame_index].source.clone())
            }
            None => Some({
                let frame = self
                    .sprite
                    .get_frame(context.scene().elapsed().await)
                    .await?;
                if let Some(remaining_duration) = self.sprite.remaining_frame_duration().await? {
                    context.estimate_next_frame(remaining_duration).await;
                }

                frame
            }),
        };
        Ok(())
    }

    async fn render(
        &mut self,
        context: &mut StyledContext,
        location: &Layout,
    ) -> KludgineResult<()> {
        let render_bounds = location.inner_bounds();
        let target_size = self.calculate_target_size(render_bounds.size).await;
        if let Some(frame) = &self.current_frame {
            let target_bounds = Rect::new(
                render_bounds.origin + (render_bounds.size - target_size) / 2.,
                target_size,
            );

            frame
                .render_with_alpha(
                    context.scene(),
                    target_bounds,
                    SpriteRotation::default(),
                    self.options.alpha,
                )
                .await
        }
        Ok(())
    }

    async fn content_size(
        &self,
        context: &mut StyledContext,
        constraints: &Size<Option<f32>, Scaled>,
    ) -> KludgineResult<Size<f32, Scaled>> {
        let scene_size = context.scene().size().await;
        Ok(self
            .calculate_target_size(Size::new(
                constraints.width.unwrap_or(scene_size.width),
                constraints.height.unwrap_or(scene_size.height),
            ))
            .await)
    }

    async fn clicked(
        &mut self,
        context: &mut Context,
        window_position: Point<f32, Scaled>,
        button: MouseButton,
    ) -> KludgineResult<()> {
        self.callback(
            context,
            ControlEvent::Clicked {
                button,
                window_position,
            },
        )
        .await;
        Ok(())
    }
}

impl Image {
    pub fn new(sprite: Sprite) -> Self {
        Self {
            sprite,
            current_frame: None,
            options: ImageOptions::default(),
        }
    }

    pub fn options(mut self, options: ImageOptions) -> Self {
        self.options = options;
        self
    }

    async fn calculate_target_size(&self, content_size: Size<f32, Scaled>) -> Size<f32, Scaled> {
        if let Some(frame) = &self.current_frame {
            let size_as_points = frame.location.size().to_f32().cast_unit();
            match &self.options.scaling {
                None => size_as_points,
                Some(scaling) => match scaling {
                    ImageScaling::Fill => content_size,
                    _ => {
                        let horizontal_scale = content_size.width / size_as_points.width;
                        let horizontal_fit = size_as_points * horizontal_scale;
                        let vertical_scale = content_size.height / size_as_points.height;
                        let vertical_fit = size_as_points * vertical_scale;

                        match scaling {
                            ImageScaling::AspectFit => {
                                if horizontal_fit.width <= content_size.width
                                    && horizontal_fit.height <= content_size.height
                                {
                                    horizontal_fit
                                } else {
                                    vertical_fit
                                }
                            }

                            ImageScaling::AspectFill => {
                                todo!("This isn't right, it's the same as AspectFit")
                                // if horizontal_fit.contains_rect(&render_bounds) {
                                //     horizontal_fit
                                // } else {
                                //     vertical_fit
                                // }
                            }
                            ImageScaling::Fill => unreachable!(),
                        }
                    }
                },
            }
        } else {
            Size::default()
        }
    }
}

#[derive(Clone, Debug)]
pub struct ImageAlphaMutator {
    image: Entity<Image>,
}

#[async_trait]
impl PropertyMutator<f32> for ImageAlphaMutator {
    async fn update_property(&self, value: f32) {
        let _ = self.image.send(ImageCommand::SetAlpha(value)).await;
    }
}
#[derive(Clone, Debug)]
pub struct ImageFrameMutator {
    image: Entity<Image>,
    tag: Option<String>,
}

#[async_trait]
impl PropertyMutator<f32> for ImageFrameMutator {
    async fn update_property(&self, value: f32) {
        // TODO: Figure out how to get the frames for the image.
        // We can cheat but it seems like it should be something
        // other people could write without being internal to the
        // crate
        let _ = self
            .image
            .send(ImageCommand::SetOverrideFrame {
                tag: self.tag.clone(),
                frame: OverrideFrame::Percent(value),
            })
            .await;
    }
}

pub type ImageAlphaAnimation = crate::ui::animation::PropertyFrameManager<
    f32,
    crate::ui::animation::FloatChange<ImageAlphaMutator>,
>;

pub type ImageFrameAnimation = crate::ui::animation::PropertyFrameManager<
    f32,
    crate::ui::animation::FloatChange<ImageFrameMutator>,
>;

impl AnimatableComponent for Image {
    type AnimationFactory = ImageAnimationFactory;

    fn new_animation_factory(index: Entity<Self>) -> Self::AnimationFactory {
        ImageAnimationFactory(index)
    }
}

// I need to take an array of AnimationFrames, which have durations within them
// And offer a way to switch frames ignoring those durations
// Perhaps the approach is that the Image component can have an explicit "Use this frame"
// setting, and that's what the animation can automate
// 0.0 = frame 0, 1.0 = last frame -- so it'll be a float property, mapped to an integer index.

pub struct ImageAnimationFactory(Entity<Image>);

impl ImageAnimationFactory {
    pub fn alpha<T: Transition + 'static>(self, target: f32, transition: T) -> ImageAlphaAnimation {
        PropertyFrameManager {
            last_value: None,
            target,
            property_change: FloatChange {
                mutator: ImageAlphaMutator { image: self.0 },
                transition: Arc::new(Box::new(transition)),
            },
        }
    }

    pub fn tagged_frame<T: Transition + 'static>(
        self,
        tag: impl ToString,
        target: f32,
        transition: T,
    ) -> ImageFrameAnimation {
        let tag = tag.to_string();

        PropertyFrameManager {
            last_value: None,
            target,
            property_change: FloatChange {
                mutator: ImageFrameMutator {
                    image: self.0,
                    tag: Some(tag),
                },
                transition: Arc::new(Box::new(transition)),
            },
        }
    }

    pub fn frame<T: Transition + 'static>(self, target: f32, transition: T) -> ImageFrameAnimation {
        PropertyFrameManager {
            last_value: None,
            target,
            property_change: FloatChange {
                mutator: ImageFrameMutator {
                    image: self.0,
                    tag: None,
                },
                transition: Arc::new(Box::new(transition)),
            },
        }
    }
}
