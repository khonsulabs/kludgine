use crate::{
    math::{Points, Rect, Size},
    source_sprite::SourceSprite,
    sprite::Sprite,
    ui::{
        animation::Transition,
        animation::{FloatChange, PropertyFrameManager, PropertyMutator},
        AnimatableComponent, Component, Context, Entity, InteractiveComponent, Layout,
        SceneContext, StyledContext,
    },
    KludgineResult,
};
use async_trait::async_trait;
use std::sync::Arc;

#[derive(Debug)]
pub struct Image {
    sprite: Sprite,
    current_frame: Option<SourceSprite>,
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
    type Input = ImageCommand;
    type Output = ();

    async fn receive_input(
        &mut self,
        _context: &mut Context,
        command: Self::Input,
    ) -> KludgineResult<()> {
        match command {
            ImageCommand::SetSprite(sprite) => {
                self.sprite = sprite;
            }
            ImageCommand::SetTag(tag) => {
                self.sprite.set_current_tag(tag).await?;
                self.options.override_frame = None;
            }
            ImageCommand::SetAlpha(alpha) => {
                self.options.alpha = alpha;
            }
            ImageCommand::SetOverrideFrame { tag, frame } => {
                self.sprite.set_current_tag(tag).await?;
                self.options.override_frame = Some(frame);
            }
        }
        Ok(())
    }
}

#[async_trait]
impl Component for Image {
    async fn update(&mut self, context: &mut SceneContext) -> KludgineResult<()> {
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
                    OverrideFrame::Percent(percent) => {
                        (frames.frames.len() as f32 * *percent) as usize
                    }
                };

                Some(frames.frames[frame_index].source.clone())
            }
            None => Some(
                self.sprite
                    .get_frame(context.scene().elapsed().await)
                    .await?,
            ),
        };
        Ok(())
    }

    async fn render(&self, context: &mut StyledContext, location: &Layout) -> KludgineResult<()> {
        if let Some(frame) = &self.current_frame {
            let render_bounds = location.inner_bounds();
            let frame_location = frame.location().await;
            let size_as_points = Size::new(
                Points::from_f32(frame_location.size.width as f32),
                Points::from_f32(frame_location.size.height as f32),
            );
            let target_bounds = match &self.options.scaling {
                None => Rect::sized(render_bounds.origin, size_as_points),
                Some(scaling) => match scaling {
                    ImageScaling::Fill => location.inner_bounds(),
                    _ => {
                        let horizontal_scale = render_bounds.size.width / size_as_points.width;
                        let horizontal_fit =
                            Rect::sized(render_bounds.origin, size_as_points * horizontal_scale);
                        let vertical_scale = render_bounds.size.height / size_as_points.height;
                        let vertical_fit =
                            Rect::sized(render_bounds.origin, size_as_points * vertical_scale);

                        match scaling {
                            ImageScaling::AspectFit => {
                                if render_bounds.approximately_contains_rect(&horizontal_fit) {
                                    horizontal_fit
                                } else {
                                    vertical_fit
                                }
                            }

                            ImageScaling::AspectFill => {
                                if horizontal_fit.approximately_contains_rect(&render_bounds) {
                                    horizontal_fit
                                } else {
                                    vertical_fit
                                }
                            }
                            ImageScaling::Fill => unreachable!(),
                        }
                    }
                },
            };

            frame
                .render_with_alpha(context.scene(), target_bounds, self.options.alpha)
                .await
        }
        Ok(())
    }

    async fn content_size(
        &self,
        _context: &mut StyledContext,
        _constraints: &Size<Option<Points>>,
    ) -> KludgineResult<Size<Points>> {
        // TODO update size desires based on fill settings
        if let Some(frame) = &self.current_frame {
            let frame_location = frame.location().await;
            let size_as_points = Size::new(
                Points::from_f32(frame_location.size.width as f32),
                Points::from_f32(frame_location.size.height as f32),
            );
            Ok(size_as_points)
        } else {
            Ok(Size::default())
        }
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

    pub fn frame<T: Transition + 'static>(
        self,
        tag: Option<impl ToString>,
        target: f32,
        transition: T,
    ) -> ImageFrameAnimation {
        let tag = tag.map(|s| s.to_string());

        PropertyFrameManager {
            last_value: None,
            target,
            property_change: FloatChange {
                mutator: ImageFrameMutator { image: self.0, tag },
                transition: Arc::new(Box::new(transition)),
            },
        }
    }
}
