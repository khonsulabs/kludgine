#![doc = include_str!("../README.md")]
// This crate uses unsafe, but attempts to minimize its usage. All functions
// that utilize unsafe must explicitly enable it.
#![deny(unsafe_code)]
#![warn(missing_docs, clippy::pedantic)]
#![allow(clippy::module_name_repetitions)]

use std::borrow::Cow;
use std::collections::HashMap;
use std::fmt::{self, Debug, Formatter};
use std::hash::{self, BuildHasher, Hash};
use std::mem::size_of;
use std::ops::{Add, AddAssign, Deref, DerefMut, Div, Neg};
use std::sync::atomic::{self, AtomicU64};
use std::sync::{Arc, Mutex, Weak};

use ahash::{AHashMap, AHasher};
use bytemuck::{Pod, Zeroable};
#[cfg(feature = "cosmic-text")]
pub use cosmic_text;
use figures::units::UPx;
use figures::{Angle, Fraction, FromComponents, Point, Rect, Size, UPx2D};
#[cfg(feature = "image")]
pub use image;
use intentional::{Assert, Cast};
use pipeline::PushConstants;
use sealed::ShapeSource as _;
use wgpu::util::DeviceExt;
pub use {figures, wgpu};

use crate::pipeline::{Uniforms, Vertex};
use crate::sealed::{ClipRect, TextureSource as _};
use crate::text::Text;

/// Application and Windowing Support.
#[cfg(feature = "app")]
pub mod app;
mod atlas;
mod buffer;
/// An easy-to-use batching renderer.
pub mod drawing;
mod pipeline;
mod pod;
mod sealed;
/// Types for drawing paths and shapes.
pub mod shapes;
/// Types for animating textures.
pub mod sprite;
/// Types for text rendering.
#[cfg(feature = "cosmic-text")]
pub mod text;
pub mod tilemap;

pub use atlas::{CollectedTexture, TextureCollection};
use buffer::Buffer;
pub use pipeline::{PreparedGraphic, ShaderScalable};

/// A 2d graphics instance.
///
/// This type contains the GPU state for a single instance of Kludgine. To
/// render graphics correctly, it must know the size and scale of the surface
/// being rendered to. These values are provided in the constructor, but can be
/// updated using [`resize()`](Self::resize).
///
/// To draw using Kludgine, create a [`Frame`] using
/// [`next_frame()`](Self::next_frame). [`wgpu`] has lifetime requirements on
/// the [`wgpu::RenderPass`] which causes each item being rendered to be
/// attached to the lifetime of the render pass. This means that no temporary
/// variables can be used to render.
///
/// Instead, graphics must be prepared before rendering, and stored somewhere
/// during the remainder of the [`RenderingGraphics`]. To prepare graphics to be
/// rendered, call [`Frame::prepare()`] to receive a [`Graphics`] instance that
/// can be used in various Kludgine APIs such as
/// [`Shape::prepare`](shapes::Shape::prepare).
#[derive(Debug)]
pub struct Kludgine {
    id: KludgineId,
    default_bindings: wgpu::BindGroup,
    pipeline: wgpu::RenderPipeline,
    _shader: wgpu::ShaderModule,
    binding_layout: wgpu::BindGroupLayout,
    linear_sampler: wgpu::Sampler,
    nearest_sampler: wgpu::Sampler,
    uniforms: Buffer<Uniforms>,
    size: Size<UPx>,
    scale: Fraction,
    #[cfg(feature = "cosmic-text")]
    text: text::TextSystem,
}

impl Kludgine {
    /// The features that wgpu requires in compatible devices.
    pub const REQURED_FEATURES: wgpu::Features = wgpu::Features::PUSH_CONSTANTS;

    /// Returns a new instance of Kludgine with the provided parameters.
    #[must_use]
    #[cfg_attr(not(feature = "cosmic-text"), allow(unused_variables))]
    pub fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        format: wgpu::TextureFormat,
        multisample: wgpu::MultisampleState,
        initial_size: Size<UPx>,
        scale: f32,
    ) -> Self {
        let id = KludgineId::unique();
        let scale = Fraction::from(scale);
        let uniforms = Buffer::new(
            &[Uniforms::new(initial_size, scale)],
            wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            device,
        );

        let binding_layout = pipeline::bind_group_layout(device, false);

        let pipeline_layout = pipeline::layout(device, &binding_layout);

        let empty_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: None,
            size: wgpu::Extent3d {
                width: 1,
                height: 1,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });

        let nearest_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            min_filter: wgpu::FilterMode::Nearest,
            mag_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..wgpu::SamplerDescriptor::default()
        });
        let linear_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            min_filter: wgpu::FilterMode::Linear,
            mag_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Linear,
            ..wgpu::SamplerDescriptor::default()
        });
        let default_bindings = pipeline::bind_group(
            device,
            &binding_layout,
            &uniforms.wgpu,
            &empty_texture.create_view(&wgpu::TextureViewDescriptor::default()),
            &nearest_sampler,
        );

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: None,
            source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(include_str!("shader.wgsl"))),
        });

        let pipeline = pipeline::new(device, &pipeline_layout, &shader, format, multisample);

        Self {
            id,
            #[cfg(feature = "cosmic-text")]
            text: text::TextSystem::new(&ProtoGraphics {
                id,
                device,
                queue,
                binding_layout: &binding_layout,
                linear_sampler: &linear_sampler,
                nearest_sampler: &nearest_sampler,
                uniforms: &uniforms.wgpu,
            }),
            default_bindings,
            pipeline,
            _shader: shader,
            linear_sampler,
            nearest_sampler,
            size: initial_size,
            scale,

            uniforms,
            binding_layout,
        }
    }

    /// Adjusts and returns the wgpu limits to support features used by
    /// Kludgine.
    #[must_use]
    pub fn adjust_limits(mut limits: wgpu::Limits) -> wgpu::Limits {
        limits.max_push_constant_size = limits
            .max_push_constant_size
            .max(size_of::<PushConstants>().try_into().assert_expected());
        limits
    }

    /// Returns the unique id of this instance.
    #[must_use]
    pub const fn id(&self) -> KludgineId {
        self.id
    }

    /// Updates the size and scale of this Kludgine instance.
    ///
    /// This function updates data stored in the GPU that affects how graphics
    /// are rendered. It should be called before calling `next_frame()` if the
    /// size or scale of the underlying surface has changed.
    pub fn resize(&mut self, new_size: Size<UPx>, new_scale: f32, queue: &wgpu::Queue) {
        let new_scale = Fraction::from(new_scale);
        if self.size != new_size || self.scale != new_scale {
            self.size = new_size;
            self.scale = new_scale;
            self.uniforms
                .update(0, &[Uniforms::new(self.size, self.scale)], queue);
        }

        #[cfg(feature = "cosmic-text")]
        self.text.scale_changed(self.scale);
    }

    /// Begins rendering a new frame.
    pub fn next_frame(&mut self) -> Frame<'_> {
        #[cfg(feature = "cosmic-text")]
        self.text.new_frame();
        Frame {
            kludgine: self,
            commands: None,
        }
    }

    /// Returns the currently configured size to render.
    pub const fn size(&self) -> Size<UPx> {
        self.size
    }

    /// Returns the current scaling factor for the display this instance is
    /// rendering to.
    pub const fn scale(&self) -> Fraction {
        self.scale
    }
}

/// The unique ID of a [`Kludgine`] instance.
#[derive(Clone, Copy, Eq, PartialEq, Debug, Hash)]
pub struct KludgineId(u64);

impl KludgineId {
    fn unique() -> Self {
        static ID_COUNTER: AtomicU64 = AtomicU64::new(0);
        Self(ID_COUNTER.fetch_add(1, atomic::Ordering::Release))
    }
}

/// A frame that can be rendered.
///
/// # Panics
///
/// After [`Frame::render()`] has been invoked, this type will panic if dropped
/// before either [`Frame::submit()`] or [`Frame::abort()`] are invoked. This
/// panic is designed to prevent accidentally forgetting to submit a frame to the GPU.q
pub struct Frame<'gfx> {
    kludgine: &'gfx mut Kludgine,
    commands: Option<wgpu::CommandEncoder>,
}

impl Frame<'_> {
    /// Creates a [`Graphics`] context for this frame that can be used to
    /// prepare graphics for rendering:
    ///
    /// - [`Shape::prepare`](shapes::Shape::prepare)
    /// - [`Texture::prepare`]
    /// - [`Texture::prepare_partial`]
    /// - [`CollectedTexture::prepare`]
    /// - [`Drawing::new_frame`](render::Drawing::new_frame)
    ///
    /// The returned graphics provides access to the various types to update
    /// their representation on the GPU so that they can be rendered later.
    pub fn prepare<'gfx>(
        &'gfx mut self,
        device: &'gfx wgpu::Device,
        queue: &'gfx wgpu::Queue,
    ) -> Graphics<'gfx> {
        Graphics::new(self.kludgine, device, queue)
    }

    /// Creates a [`RenderingGraphics`] context for this frame which is used to
    /// render previously prepared graphics:
    ///
    /// - [`PreparedGraphic`]
    /// - [`PreparedText`](text::PreparedText)
    /// - [`Drawing`](render::Drawing)
    #[must_use]
    pub fn render<'gfx, 'pass>(
        &'pass mut self,
        pass: &wgpu::RenderPassDescriptor<'pass, '_>,
        device: &'gfx wgpu::Device,
        queue: &'gfx wgpu::Queue,
    ) -> RenderingGraphics<'gfx, 'pass> {
        if self.commands.is_none() {
            self.commands =
                Some(device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default()));
        }
        RenderingGraphics::new(
            self.commands
                .as_mut()
                .assert("initialized above")
                .begin_render_pass(pass),
            self.kludgine,
            device,
            queue,
        )
    }

    /// Creates a [`RenderingGraphics`] that renders into `texture` for this
    /// frame. The returned context can be used to render previously prepared
    /// graphics:
    ///
    /// - [`PreparedGraphic`]
    /// - [`PreparedText`](text::PreparedText)
    /// - [`Drawing`](render::Drawing)
    pub fn render_into<'gfx, 'pass>(
        &'pass mut self,
        texture: &'pass Texture,
        load_op: wgpu::LoadOp<Color>,
        device: &'gfx wgpu::Device,
        queue: &'gfx wgpu::Queue,
    ) -> RenderingGraphics<'gfx, 'pass> {
        self.render(
            &wgpu::RenderPassDescriptor {
                label: None,
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &texture.data.view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: match load_op {
                            wgpu::LoadOp::Clear(color) => wgpu::LoadOp::Clear(color.into()),
                            wgpu::LoadOp::Load => wgpu::LoadOp::Load,
                        },
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            },
            device,
            queue,
        )
    }

    /// Submits all of the commands for this frame to the GPU.
    ///
    /// This function does not block for the operations to finish. The returned
    /// [`wgpu::SubmissionIndex`] can be used to block until completion if
    /// desired.
    #[allow(clippy::must_use_candidate)]
    pub fn submit(mut self, queue: &wgpu::Queue) -> Option<wgpu::SubmissionIndex> {
        let commands = self.commands.take()?;
        Some(queue.submit([commands.finish()]))
    }

    /// Aborts rendering this frame.
    ///
    /// If [`Frame::render()`] has been invoked, this function must be used
    /// instead of dropping the frame. This type implements a panic-on-drop to
    /// prevent forgetting to submit the frame to the GPU, and this function
    /// prevents the panic from happening.
    pub fn abort(mut self) {
        // Clear out the commands, preventing drop from panicking.
        self.commands.take();
    }
}

impl Drop for Frame<'_> {
    fn drop(&mut self) {
        assert!(
            self.commands.is_none(),
            "Frame dropped without calling finish() or abort()"
        );
    }
}

/// A generic graphics context.
///
/// This generic trait is used on some APIs because they are utilized both
/// publicly and internally. The only user-facing type that implements this
/// trait is [`Graphics`].
pub trait KludgineGraphics: sealed::KludgineGraphics {}

struct ProtoGraphics<'gfx> {
    id: KludgineId,
    device: &'gfx wgpu::Device,
    queue: &'gfx wgpu::Queue,
    binding_layout: &'gfx wgpu::BindGroupLayout,
    linear_sampler: &'gfx wgpu::Sampler,
    nearest_sampler: &'gfx wgpu::Sampler,
    uniforms: &'gfx wgpu::Buffer,
}

impl<'a> ProtoGraphics<'a> {
    fn new(device: &'a wgpu::Device, queue: &'a wgpu::Queue, kludgine: &'a Kludgine) -> Self {
        Self {
            id: kludgine.id,
            device,
            queue,
            binding_layout: &kludgine.binding_layout,
            linear_sampler: &kludgine.linear_sampler,
            nearest_sampler: &kludgine.nearest_sampler,
            uniforms: &kludgine.uniforms.wgpu,
        }
    }
}

impl KludgineGraphics for ProtoGraphics<'_> {}

impl sealed::KludgineGraphics for ProtoGraphics<'_> {
    fn id(&self) -> KludgineId {
        self.id
    }

    fn device(&self) -> &wgpu::Device {
        self.device
    }

    fn queue(&self) -> &wgpu::Queue {
        self.queue
    }

    fn binding_layout(&self) -> &wgpu::BindGroupLayout {
        self.binding_layout
    }

    fn uniforms(&self) -> &wgpu::Buffer {
        self.uniforms
    }

    fn nearest_sampler(&self) -> &wgpu::Sampler {
        self.nearest_sampler
    }

    fn linear_sampler(&self) -> &wgpu::Sampler {
        self.linear_sampler
    }
}

impl KludgineGraphics for Graphics<'_> {}

impl sealed::KludgineGraphics for Graphics<'_> {
    fn id(&self) -> KludgineId {
        self.kludgine.id
    }

    fn device(&self) -> &wgpu::Device {
        self.device
    }

    fn queue(&self) -> &wgpu::Queue {
        self.queue
    }

    fn binding_layout(&self) -> &wgpu::BindGroupLayout {
        &self.kludgine.binding_layout
    }

    fn uniforms(&self) -> &wgpu::Buffer {
        &self.kludgine.uniforms.wgpu
    }

    fn nearest_sampler(&self) -> &wgpu::Sampler {
        &self.kludgine.nearest_sampler
    }

    fn linear_sampler(&self) -> &wgpu::Sampler {
        &self.kludgine.linear_sampler
    }
}

#[derive(Debug)]
struct ClipStack {
    current: ClipRect,
    previous_clips: Vec<ClipRect>,
}

impl ClipStack {
    pub fn new(size: Size<UPx>) -> Self {
        Self {
            current: size.into(),
            previous_clips: Vec::new(),
        }
    }

    pub fn push_clip(&mut self, clip: Rect<UPx>) {
        let previous_clip = self.current;
        self.current = previous_clip.clip_to(clip.expand_rounded());
        self.previous_clips.push(previous_clip);
    }

    pub fn pop_clip(&mut self) {
        self.current = self.previous_clips.pop().expect("unpaired pop_clip");
    }
}

/// A context used to prepare graphics to render.
///
/// This type is used in these APIs:
///
/// - [`Shape::prepare`](shapes::Shape::prepare)
/// - [`Texture::prepare`]
/// - [`Texture::prepare_partial`]
/// - [`CollectedTexture::prepare`]
/// - [`Drawing::new_frame`](render::Drawing::new_frame)
#[derive(Debug)]
pub struct Graphics<'gfx> {
    kludgine: &'gfx mut Kludgine,
    device: &'gfx wgpu::Device,
    queue: &'gfx wgpu::Queue, // Need this eventually to be able to have dynamic shape collections
    clip: ClipStack,
}

impl<'gfx> Graphics<'gfx> {
    /// Returns a new instance.
    pub fn new(
        kludgine: &'gfx mut Kludgine,
        device: &'gfx wgpu::Device,
        queue: &'gfx wgpu::Queue,
    ) -> Self {
        Self {
            clip: ClipStack::new(kludgine.size),
            kludgine,
            device,
            queue,
        }
    }

    /// Returns a reference to the underlying [`wgpu::Device`].
    #[must_use]
    pub const fn device(&self) -> &'gfx wgpu::Device {
        self.device
    }

    /// Returns a reference to the underlying [`wgpu::Queue`].
    #[must_use]
    pub const fn queue(&self) -> &'gfx wgpu::Queue {
        self.queue
    }

    /// Returns a mutable reference to the [`cosmic_text::FontSystem`] used when
    /// rendering text.
    #[cfg(feature = "cosmic-text")]
    pub fn font_system(&mut self) -> &mut cosmic_text::FontSystem {
        self.kludgine.font_system()
    }

    /// Returns the current clipped size of the context.
    ///
    /// If this context has not been clipped, the value returned will be
    /// equivalent to [`Kludgine::size`].
    #[must_use]
    pub const fn size(&self) -> Size<UPx> {
        self.clip.current.0.size
    }

    /// Returns the current rectangular area of the context.
    ///
    /// If this context has not been clipped, the value returned will be
    /// equivalent to [`Kludgine::size`] with an origin of `0,0`.
    #[must_use]
    pub const fn clip_rect(&self) -> Rect<UPx> {
        self.clip.current.0
    }
}

impl AsRef<wgpu::Device> for Graphics<'_> {
    fn as_ref(&self) -> &wgpu::Device {
        self.device()
    }
}

impl AsRef<wgpu::Queue> for Graphics<'_> {
    fn as_ref(&self) -> &wgpu::Queue {
        self.queue()
    }
}

impl Deref for Graphics<'_> {
    type Target = Kludgine;

    fn deref(&self) -> &Self::Target {
        self.kludgine
    }
}

impl DerefMut for Graphics<'_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.kludgine
    }
}

impl Clipped for Graphics<'_> {
    fn push_clip(&mut self, clip: Rect<UPx>) {
        self.clip.push_clip(clip);
    }

    fn pop_clip(&mut self) {
        self.clip.pop_clip();
    }
}

impl sealed::Clipped for Graphics<'_> {}

/// A graphics context used to render previously prepared graphics.
///
/// This type is used to render these types:
///
/// - [`PreparedGraphic`]
/// - [`PreparedText`](text::PreparedText)
/// - [`Drawing`](render::Drawing)
pub struct RenderingGraphics<'gfx, 'pass> {
    pass: wgpu::RenderPass<'pass>,
    kludgine: &'pass Kludgine,
    device: &'gfx wgpu::Device,
    queue: &'gfx wgpu::Queue,
    clip: ClipStack,
    pipeline_is_active: bool,
}

impl<'gfx, 'pass> RenderingGraphics<'gfx, 'pass> {
    fn new(
        pass: wgpu::RenderPass<'pass>,
        kludgine: &'pass Kludgine,
        device: &'gfx wgpu::Device,
        queue: &'gfx wgpu::Queue,
    ) -> Self {
        Self {
            pass,
            clip: ClipStack::new(kludgine.size),
            kludgine,
            device,
            queue,
            pipeline_is_active: false,
        }
    }

    /// Returns a reference to the underlying [`wgpu::Device`].
    #[must_use]
    pub const fn device(&self) -> &'gfx wgpu::Device {
        self.device
    }

    /// Returns a reference to the underlying [`wgpu::Queue`].
    #[must_use]
    pub const fn queue(&self) -> &'gfx wgpu::Queue {
        self.queue
    }

    fn active_pipeline_if_needed(&mut self) -> bool {
        if self.pipeline_is_active {
            false
        } else {
            self.pipeline_is_active = true;
            self.pass.set_pipeline(&self.kludgine.pipeline);
            true
        }
    }

    /// Returns a [`ClipGuard`] that causes all drawing operations to be offset
    /// and clipped to `clip` until it is dropped.
    ///
    /// This function causes the [`RenderingGraphics`] to act as if the origin
    /// of the context is `clip.origin`, and the size of the context is
    /// `clip.size`. This means that rendering at 0,0 will actually render at
    /// the effective clip rect's origin.
    ///
    /// `clip` is relative to the current clip rect and cannot extend the
    /// current clipping rectangle.
    pub fn clipped_to(&mut self, clip: Rect<UPx>) -> ClipGuard<'_, Self> {
        self.push_clip(clip);
        ClipGuard { clipped: self }
    }

    /// Returns the current size of the graphics area being rendered to.
    ///
    /// If the graphics has been clipped, this returns the current width of the
    /// clipped area.
    #[must_use]
    pub const fn size(&self) -> Size<UPx> {
        self.clip.current.0.size
    }

    /// Returns the current scaling factor of the display being rendered to.
    #[must_use]
    pub const fn scale(&self) -> Fraction {
        self.kludgine.scale()
    }
}

/// A graphics context that has been clipped.
pub trait Clipped: Sized + sealed::Clipped {
    /// Pushes a new clipping state to the clipping stack.
    ///
    /// This function causes this type to act as if the origin of the context is
    /// `clip.origin`, and the size of the context is `clip.size`. This means
    /// that rendering at 0,0 will actually render at the effective clip rect's
    /// origin.
    ///
    /// `clip` is relative to the current clip rect and cannot extend the
    /// current clipping rectangle.
    ///
    /// To restore the clipping rect to the state it was before this function
    /// was called, use [`Clipped::pop_clip()`].
    fn push_clip(&mut self, clip: Rect<UPx>);
    /// Restores the clipping rect to the previous state before the last call to
    /// [`Clipped::push_clip()`].
    ///
    /// # Panics
    ///
    /// This function will panic if it is called more times than
    /// [`Clipped::push_clip()`].
    fn pop_clip(&mut self);

    /// Returns a [`ClipGuard`] that causes all drawing operations to be offset
    /// and clipped to `clip` until it is dropped.
    ///
    /// This function causes this type to act as if the origin of the context is
    /// `clip.origin`, and the size of the context is `clip.size`. This means
    /// that rendering at 0,0 will actually render at the effective clip rect's
    /// origin.
    ///
    /// `clip` is relative to the current clip rect and cannot extend the
    /// current clipping rectangle.
    fn clipped_to(&mut self, clip: Rect<UPx>) -> ClipGuard<'_, Self> {
        self.push_clip(clip);
        ClipGuard { clipped: self }
    }
}

impl Clipped for RenderingGraphics<'_, '_> {
    fn pop_clip(&mut self) {
        self.clip.pop_clip();
        if self.clip.current.size.width > 0 && self.clip.current.size.height > 0 {
            self.pass.set_scissor_rect(
                self.clip.current.origin.x.into(),
                self.clip.current.origin.y.into(),
                self.clip.current.size.width.into(),
                self.clip.current.size.height.into(),
            );
        }
    }

    fn push_clip(&mut self, clip: Rect<UPx>) {
        self.clip.push_clip(clip);
        if self.clip.current.size.width > 0 && self.clip.current.size.height > 0 {
            self.pass.set_scissor_rect(
                self.clip.current.origin.x.into(),
                self.clip.current.origin.y.into(),
                self.clip.current.size.width.into(),
                self.clip.current.size.height.into(),
            );
        }
    }
}

impl sealed::Clipped for RenderingGraphics<'_, '_> {}

impl Drop for RenderingGraphics<'_, '_> {
    fn drop(&mut self) {
        // This shouldn't be necessary, but under the GL backend, Cushy only
        // renders the final widget/clipped region. By setting this, it makes
        // Cushy work on the GL backend.
        self.pass.set_scissor_rect(
            0,
            0,
            self.kludgine.size.width.get(),
            self.kludgine.size.height.get(),
        );
    }
}

/// A clipped surface.
///
/// When dropped, the clipped type will have its clip rect restored to the
/// previously clipped rect. [`ClipGuard`]s can be nested.
///
/// This type implements [`Deref`]/[`DerefMut`] to provide access to the
/// underyling clipped type.
#[derive(Debug)]
pub struct ClipGuard<'clip, T>
where
    T: Clipped,
{
    clipped: &'clip mut T,
}

impl<T> Drop for ClipGuard<'_, T>
where
    T: Clipped,
{
    fn drop(&mut self) {
        self.clipped.pop_clip();
    }
}

impl<T> Deref for ClipGuard<'_, T>
where
    T: Clipped,
{
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.clipped
    }
}

impl<T> DerefMut for ClipGuard<'_, T>
where
    T: Clipped,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.clipped
    }
}

/// A red, green, blue, and alpha color value stored in 32-bits.
#[derive(Clone, Copy, Eq, PartialEq, Hash, Pod, Zeroable)]
#[repr(C)]
pub struct Color(u32);

pub(crate) fn f32_component_to_u8(component: f32) -> u8 {
    (component.clamp(0., 1.0) * 255.).round().cast()
}

impl Color {
    /// Returns a new color with the provided components.
    #[must_use]
    pub const fn new(red: u8, green: u8, blue: u8, alpha: u8) -> Self {
        Self((red as u32) << 24 | (green as u32) << 16 | (blue as u32) << 8 | alpha as u32)
    }

    /// Returns a new color by converting each component from its `0.0..=1.0`
    /// range into a `0..=255` range.
    #[must_use]
    pub fn new_f32(red: f32, green: f32, blue: f32, alpha: f32) -> Self {
        Self::new(
            f32_component_to_u8(red),
            f32_component_to_u8(green),
            f32_component_to_u8(blue),
            f32_component_to_u8(alpha),
        )
    }

    /// Returns the red component of this color, range 0-255.
    #[must_use]
    #[allow(clippy::cast_possible_truncation)] // truncation desired
    pub const fn red(self) -> u8 {
        (self.0 >> 24) as u8
    }

    /// Returns the red component of this color, range 0.0-1.0.
    #[must_use]
    pub fn red_f32(self) -> f32 {
        f32::from(self.red()) / 255.
    }

    /// Returns the green component of this color, range 0-255.
    #[must_use]
    #[allow(clippy::cast_possible_truncation)] // truncation desired
    pub const fn green(self) -> u8 {
        (self.0 >> 16) as u8
    }

    /// Returns the green component of this color, range 0.0-1.0.
    #[must_use]
    pub fn green_f32(self) -> f32 {
        f32::from(self.green()) / 255.
    }

    /// Returns the blue component of this color, range 0-255.
    #[must_use]
    #[allow(clippy::cast_possible_truncation)] // truncation desired
    pub const fn blue(self) -> u8 {
        (self.0 >> 8) as u8
    }

    /// Returns the blue component of this color, range 0.0-1.0.
    #[must_use]
    pub fn blue_f32(self) -> f32 {
        f32::from(self.blue()) / 255.
    }

    /// Returns the alpha component of this color, range 0-255. A value of 255
    /// is completely opaque.
    #[must_use]
    #[allow(clippy::cast_possible_truncation)] // truncation desired
    pub const fn alpha(self) -> u8 {
        self.0 as u8
    }

    /// Returns the alpha component of this color, range 0.0-1.0. A value of 1.0
    /// is completely opaque.
    #[must_use]
    pub fn alpha_f32(self) -> f32 {
        f32::from(self.alpha()) / 255.
    }

    /// Returns a new color replacing this colors red channel with `red`.
    #[must_use]
    pub const fn with_red(self, red: u8) -> Self {
        Self(self.0 & 0x00FF_FFFF | ((red as u32) << 24))
    }

    /// Returns a new color replacing this colors green channel with `green`.
    #[must_use]
    pub const fn with_green(self, red: u8) -> Self {
        Self(self.0 & 0xFF00_FFFF | ((red as u32) << 16))
    }

    /// Returns a new color replacing this colors blue channel with `blue`.
    #[must_use]
    pub const fn with_blue(self, blue: u8) -> Self {
        Self(self.0 & 0xFFFF_00FF | ((blue as u32) << 8))
    }

    /// Returns a new color replacing this colors alpha channel with `alpha`.
    #[must_use]
    pub const fn with_alpha(self, alpha: u8) -> Self {
        Self(self.0 & 0xFFFF_FF00 | alpha as u32)
    }

    /// Returns a new color replacing this colors red channel with `red`.
    #[must_use]
    pub fn with_red_f32(self, red: f32) -> Self {
        self.with_red(f32_component_to_u8(red))
    }

    /// Returns a new color replacing this colors green channel with `green`.
    #[must_use]
    pub fn with_green_f32(self, green: f32) -> Self {
        self.with_green(f32_component_to_u8(green))
    }

    /// Returns a new color replacing this colors blue channel with `blue`.
    #[must_use]
    pub fn with_blue_f32(self, blue: f32) -> Self {
        self.with_blue(f32_component_to_u8(blue))
    }

    /// Returns a new color replacing this colors alpha channel with `alpha`.
    #[must_use]
    pub fn with_alpha_f32(self, alpha: f32) -> Self {
        self.with_alpha(f32_component_to_u8(alpha))
    }
}

fn srgb_to_linear(red: f32, green: f32, blue: f32, alpha: f32) -> Color {
    let linear = palette::rgb::Srgba::new(red, green, blue, alpha).into_linear();
    Color::new_f32(linear.red, linear.green, linear.blue, linear.alpha)
}

impl Debug for Color {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "#{:08x}", self.0)
    }
}

impl From<Color> for wgpu::Color {
    fn from(color: Color) -> Self {
        Self {
            r: f64::from(color.red_f32()),
            g: f64::from(color.green_f32()),
            b: f64::from(color.blue_f32()),
            a: f64::from(color.alpha_f32()),
        }
    }
}

#[cfg(feature = "cosmic-text")]
impl From<cosmic_text::Color> for Color {
    fn from(value: cosmic_text::Color) -> Self {
        Self::new(value.r(), value.g(), value.b(), value.a())
    }
}

#[cfg(feature = "cosmic-text")]
impl From<Color> for cosmic_text::Color {
    fn from(value: Color) -> Self {
        Self::rgba(value.red(), value.green(), value.blue(), value.alpha())
    }
}

#[test]
fn color_debug() {
    assert_eq!(format!("{:?}", Color::new(1, 2, 3, 4)), "#01020304");
}

impl Color {
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/named-color) of the same name.
    pub const ALICEBLUE: Self = Self::new(240, 248, 255, 255);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/named-color) of the same name.
    pub const ANTIQUEWHITE: Self = Self::new(250, 235, 215, 255);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/named-color) of the same name.
    pub const AQUA: Self = Self::new(0, 255, 255, 255);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/named-color) of the same name.
    pub const AQUAMARINE: Self = Self::new(127, 255, 212, 255);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/named-color) of the same name.
    pub const AZURE: Self = Self::new(240, 255, 255, 255);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/named-color) of the same name.
    pub const BEIGE: Self = Self::new(245, 245, 220, 255);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/named-color) of the same name.
    pub const BISQUE: Self = Self::new(255, 228, 196, 255);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/named-color) of the same name.
    pub const BLACK: Self = Self::new(0, 0, 0, 255);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/named-color) of the same name.
    pub const BLANCHEDALMOND: Self = Self::new(255, 235, 205, 255);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/named-color) of the same name.
    pub const BLUE: Self = Self::new(0, 0, 255, 255);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/named-color) of the same name.
    pub const BLUEVIOLET: Self = Self::new(138, 43, 226, 255);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/named-color) of the same name.
    pub const BROWN: Self = Self::new(165, 42, 42, 255);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/named-color) of the same name.
    pub const BURLYWOOD: Self = Self::new(222, 184, 135, 255);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/named-color) of the same name.
    pub const CADETBLUE: Self = Self::new(95, 158, 160, 255);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/named-color) of the same name.
    pub const CHARTREUSE: Self = Self::new(127, 255, 0, 255);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/named-color) of the same name.
    pub const CHOCOLATE: Self = Self::new(210, 105, 30, 255);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/named-color) of the same name.
    pub const CLEAR_BLACK: Self = Self::new(0, 0, 0, 0);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/named-color) of the same name.
    pub const CLEAR_WHITE: Self = Self::new(255, 255, 255, 0);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/named-color) of the same name.
    pub const CORAL: Self = Self::new(255, 127, 80, 255);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/named-color) of the same name.
    pub const CORNFLOWERBLUE: Self = Self::new(100, 149, 237, 255);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/named-color) of the same name.
    pub const CORNSILK: Self = Self::new(255, 248, 220, 255);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/named-color) of the same name.
    pub const CRIMSON: Self = Self::new(220, 20, 60, 255);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/named-color) of the same name.
    pub const CYAN: Self = Self::new(0, 255, 255, 255);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/named-color) of the same name.
    pub const DARKBLUE: Self = Self::new(0, 0, 139, 255);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/named-color) of the same name.
    pub const DARKCYAN: Self = Self::new(0, 139, 139, 255);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/named-color) of the same name.
    pub const DARKGOLDENROD: Self = Self::new(184, 134, 11, 255);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/named-color) of the same name.
    pub const DARKGRAY: Self = Self::new(169, 169, 169, 255);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/named-color) of the same name.
    pub const DARKGREEN: Self = Self::new(0, 100, 0, 255);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/named-color) of the same name.
    pub const DARKGREY: Self = Self::new(169, 169, 169, 255);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/named-color) of the same name.
    pub const DARKKHAKI: Self = Self::new(189, 183, 107, 255);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/named-color) of the same name.
    pub const DARKMAGENTA: Self = Self::new(139, 0, 139, 255);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/named-color) of the same name.
    pub const DARKOLIVEGREEN: Self = Self::new(85, 107, 47, 255);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/named-color) of the same name.
    pub const DARKORANGE: Self = Self::new(255, 140, 0, 255);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/named-color) of the same name.
    pub const DARKORCHID: Self = Self::new(153, 50, 204, 255);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/named-color) of the same name.
    pub const DARKRED: Self = Self::new(139, 0, 0, 255);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/named-color) of the same name.
    pub const DARKSALMON: Self = Self::new(233, 150, 122, 255);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/named-color) of the same name.
    pub const DARKSEAGREEN: Self = Self::new(143, 188, 143, 255);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/named-color) of the same name.
    pub const DARKSLATEBLUE: Self = Self::new(72, 61, 139, 255);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/named-color) of the same name.
    pub const DARKSLATEGRAY: Self = Self::new(47, 79, 79, 255);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/named-color) of the same name.
    pub const DARKSLATEGREY: Self = Self::new(47, 79, 79, 255);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/named-color) of the same name.
    pub const DARKTURQUOISE: Self = Self::new(0, 206, 209, 255);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/named-color) of the same name.
    pub const DARKVIOLET: Self = Self::new(148, 0, 211, 255);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/named-color) of the same name.
    pub const DEEPPINK: Self = Self::new(255, 20, 147, 255);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/named-color) of the same name.
    pub const DEEPSKYBLUE: Self = Self::new(0, 191, 255, 255);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/named-color) of the same name.
    pub const DIMGRAY: Self = Self::new(105, 105, 105, 255);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/named-color) of the same name.
    pub const DIMGREY: Self = Self::new(105, 105, 105, 255);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/named-color) of the same name.
    pub const DODGERBLUE: Self = Self::new(30, 144, 255, 255);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/named-color) of the same name.
    pub const FIREBRICK: Self = Self::new(178, 34, 34, 255);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/named-color) of the same name.
    pub const FLORALWHITE: Self = Self::new(255, 250, 240, 255);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/named-color) of the same name.
    pub const FORESTGREEN: Self = Self::new(34, 139, 34, 255);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/named-color) of the same name.
    pub const FUCHSIA: Self = Self::new(255, 0, 255, 255);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/named-color) of the same name.
    pub const GAINSBORO: Self = Self::new(220, 220, 220, 255);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/named-color) of the same name.
    pub const GHOSTWHITE: Self = Self::new(248, 248, 255, 255);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/named-color) of the same name.
    pub const GOLD: Self = Self::new(255, 215, 0, 255);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/named-color) of the same name.
    pub const GOLDENROD: Self = Self::new(218, 165, 32, 255);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/named-color) of the same name.
    pub const GRAY: Self = Self::new(128, 128, 128, 255);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/named-color) of the same name.
    pub const GREEN: Self = Self::new(0, 128, 0, 255);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/named-color) of the same name.
    pub const GREENYELLOW: Self = Self::new(173, 255, 47, 255);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/named-color) of the same name.
    pub const GREY: Self = Self::new(128, 128, 128, 255);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/named-color) of the same name.
    pub const HONEYDEW: Self = Self::new(240, 255, 240, 255);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/named-color) of the same name.
    pub const HOTPINK: Self = Self::new(255, 105, 180, 255);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/named-color) of the same name.
    pub const INDIANRED: Self = Self::new(205, 92, 92, 255);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/named-color) of the same name.
    pub const INDIGO: Self = Self::new(75, 0, 130, 255);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/named-color) of the same name.
    pub const IVORY: Self = Self::new(255, 255, 240, 255);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/named-color) of the same name.
    pub const KHAKI: Self = Self::new(240, 230, 140, 255);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/named-color) of the same name.
    pub const LAVENDER: Self = Self::new(230, 230, 250, 255);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/named-color) of the same name.
    pub const LAVENDERBLUSH: Self = Self::new(255, 240, 245, 255);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/named-color) of the same name.
    pub const LAWNGREEN: Self = Self::new(124, 252, 0, 255);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/named-color) of the same name.
    pub const LEMONCHIFFON: Self = Self::new(255, 250, 205, 255);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/named-color) of the same name.
    pub const LIGHTBLUE: Self = Self::new(173, 216, 230, 255);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/named-color) of the same name.
    pub const LIGHTCORAL: Self = Self::new(240, 128, 128, 255);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/named-color) of the same name.
    pub const LIGHTCYAN: Self = Self::new(224, 255, 255, 255);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/named-color) of the same name.
    pub const LIGHTGOLDENRODYELLOW: Self = Self::new(250, 250, 210, 255);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/named-color) of the same name.
    pub const LIGHTGRAY: Self = Self::new(211, 211, 211, 255);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/named-color) of the same name.
    pub const LIGHTGREEN: Self = Self::new(144, 238, 144, 255);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/named-color) of the same name.
    pub const LIGHTGREY: Self = Self::new(211, 211, 211, 255);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/named-color) of the same name.
    pub const LIGHTPINK: Self = Self::new(255, 182, 193, 255);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/named-color) of the same name.
    pub const LIGHTSALMON: Self = Self::new(255, 160, 122, 255);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/named-color) of the same name.
    pub const LIGHTSEAGREEN: Self = Self::new(32, 178, 170, 255);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/named-color) of the same name.
    pub const LIGHTSKYBLUE: Self = Self::new(135, 206, 250, 255);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/named-color) of the same name.
    pub const LIGHTSLATEGRAY: Self = Self::new(119, 136, 153, 255);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/named-color) of the same name.
    pub const LIGHTSLATEGREY: Self = Self::new(119, 136, 153, 255);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/named-color) of the same name.
    pub const LIGHTSTEELBLUE: Self = Self::new(176, 196, 222, 255);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/named-color) of the same name.
    pub const LIGHTYELLOW: Self = Self::new(255, 255, 224, 255);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/named-color) of the same name.
    pub const LIME: Self = Self::new(0, 255, 0, 255);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/named-color) of the same name.
    pub const LIMEGREEN: Self = Self::new(50, 205, 50, 255);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/named-color) of the same name.
    pub const LINEN: Self = Self::new(250, 240, 230, 255);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/named-color) of the same name.
    pub const MAGENTA: Self = Self::new(255, 0, 255, 255);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/named-color) of the same name.
    pub const MAROON: Self = Self::new(128, 0, 0, 255);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/named-color) of the same name.
    pub const MEDIUMAQUAMARINE: Self = Self::new(102, 205, 170, 255);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/named-color) of the same name.
    pub const MEDIUMBLUE: Self = Self::new(0, 0, 205, 255);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/named-color) of the same name.
    pub const MEDIUMORCHID: Self = Self::new(186, 85, 211, 255);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/named-color) of the same name.
    pub const MEDIUMPURPLE: Self = Self::new(147, 112, 219, 255);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/named-color) of the same name.
    pub const MEDIUMSEAGREEN: Self = Self::new(60, 179, 113, 255);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/named-color) of the same name.
    pub const MEDIUMSLATEBLUE: Self = Self::new(123, 104, 238, 255);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/named-color) of the same name.
    pub const MEDIUMSPRINGGREEN: Self = Self::new(0, 250, 154, 255);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/named-color) of the same name.
    pub const MEDIUMTURQUOISE: Self = Self::new(72, 209, 204, 255);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/named-color) of the same name.
    pub const MEDIUMVIOLETRED: Self = Self::new(199, 21, 133, 255);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/named-color) of the same name.
    pub const MIDNIGHTBLUE: Self = Self::new(25, 25, 112, 255);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/named-color) of the same name.
    pub const MINTCREAM: Self = Self::new(245, 255, 250, 255);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/named-color) of the same name.
    pub const MISTYROSE: Self = Self::new(255, 228, 225, 255);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/named-color) of the same name.
    pub const MOCCASIN: Self = Self::new(255, 228, 181, 255);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/named-color) of the same name.
    pub const NAVAJOWHITE: Self = Self::new(255, 222, 173, 255);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/named-color) of the same name.
    pub const NAVY: Self = Self::new(0, 0, 128, 255);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/named-color) of the same name.
    pub const OLDLACE: Self = Self::new(253, 245, 230, 255);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/named-color) of the same name.
    pub const OLIVE: Self = Self::new(128, 128, 0, 255);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/named-color) of the same name.
    pub const OLIVEDRAB: Self = Self::new(107, 142, 35, 255);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/named-color) of the same name.
    pub const ORANGE: Self = Self::new(255, 165, 0, 255);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/named-color) of the same name.
    pub const ORANGERED: Self = Self::new(255, 69, 0, 255);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/named-color) of the same name.
    pub const ORCHID: Self = Self::new(218, 112, 214, 255);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/named-color) of the same name.
    pub const PALEGOLDENROD: Self = Self::new(238, 232, 170, 255);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/named-color) of the same name.
    pub const PALEGREEN: Self = Self::new(152, 251, 152, 255);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/named-color) of the same name.
    pub const PALETURQUOISE: Self = Self::new(175, 238, 238, 255);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/named-color) of the same name.
    pub const PALEVIOLETRED: Self = Self::new(219, 112, 147, 255);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/named-color) of the same name.
    pub const PAPAYAWHIP: Self = Self::new(255, 239, 213, 255);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/named-color) of the same name.
    pub const PEACHPUFF: Self = Self::new(255, 218, 185, 255);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/named-color) of the same name.
    pub const PERU: Self = Self::new(205, 133, 63, 255);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/named-color) of the same name.
    pub const PINK: Self = Self::new(255, 192, 203, 255);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/named-color) of the same name.
    pub const PLUM: Self = Self::new(221, 160, 221, 255);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/named-color) of the same name.
    pub const POWDERBLUE: Self = Self::new(176, 224, 230, 255);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/named-color) of the same name.
    pub const PURPLE: Self = Self::new(128, 0, 128, 255);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/named-color) of the same name.
    pub const REBECCAPURPLE: Self = Self::new(102, 51, 153, 255);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/named-color) of the same name.
    pub const RED: Self = Self::new(255, 0, 0, 255);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/named-color) of the same name.
    pub const ROSYBROWN: Self = Self::new(188, 143, 143, 255);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/named-color) of the same name.
    pub const ROYALBLUE: Self = Self::new(65, 105, 225, 255);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/named-color) of the same name.
    pub const SADDLEBROWN: Self = Self::new(139, 69, 19, 255);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/named-color) of the same name.
    pub const SALMON: Self = Self::new(250, 128, 114, 255);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/named-color) of the same name.
    pub const SANDYBROWN: Self = Self::new(244, 164, 96, 255);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/named-color) of the same name.
    pub const SEAGREEN: Self = Self::new(46, 139, 87, 255);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/named-color) of the same name.
    pub const SEASHELL: Self = Self::new(255, 245, 238, 255);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/named-color) of the same name.
    pub const SIENNA: Self = Self::new(160, 82, 45, 255);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/named-color) of the same name.
    pub const SILVER: Self = Self::new(192, 192, 192, 255);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/named-color) of the same name.
    pub const SKYBLUE: Self = Self::new(135, 206, 235, 255);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/named-color) of the same name.
    pub const SLATEBLUE: Self = Self::new(106, 90, 205, 255);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/named-color) of the same name.
    pub const SLATEGRAY: Self = Self::new(112, 128, 144, 255);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/named-color) of the same name.
    pub const SLATEGREY: Self = Self::new(112, 128, 144, 255);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/named-color) of the same name.
    pub const SNOW: Self = Self::new(255, 250, 250, 255);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/named-color) of the same name.
    pub const SPRINGGREEN: Self = Self::new(0, 255, 127, 255);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/named-color) of the same name.
    pub const STEELBLUE: Self = Self::new(70, 130, 180, 255);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/named-color) of the same name.
    pub const TAN: Self = Self::new(210, 180, 140, 255);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/named-color) of the same name.
    pub const TEAL: Self = Self::new(0, 128, 128, 255);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/named-color) of the same name.
    pub const THISTLE: Self = Self::new(216, 191, 216, 255);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/named-color) of the same name.
    pub const TOMATO: Self = Self::new(255, 99, 71, 255);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/named-color) of the same name.
    pub const TURQUOISE: Self = Self::new(64, 224, 208, 255);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/named-color) of the same name.
    pub const VIOLET: Self = Self::new(238, 130, 238, 255);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/named-color) of the same name.
    pub const WHEAT: Self = Self::new(245, 222, 179, 255);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/named-color) of the same name.
    pub const WHITE: Self = Self::new(255, 255, 255, 255);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/named-color) of the same name.
    pub const WHITESMOKE: Self = Self::new(245, 245, 245, 255);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/named-color) of the same name.
    pub const YELLOW: Self = Self::new(255, 255, 0, 255);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/named-color) of the same name.
    pub const YELLOWGREEN: Self = Self::new(154, 205, 50, 255);
}

/// A [`TextureSource`] that loads its data lazily.
///
/// This texture type can be shared between multiple [`wgpu::Device`]s. When a
/// clone of this texture is used, a unique copy will be loaded once per
/// [`wgpu::Device`].
#[derive(Debug)]
pub struct LazyTexture {
    data: Arc<LazyTextureData>,
    last_loaded: Mutex<Option<(KludgineId, SharedTexture)>>,
}

impl LazyTexture {
    /// Returns a new texture that loads its data to the gpu once used.
    #[must_use]
    pub fn from_data(
        size: Size<UPx>,
        format: wgpu::TextureFormat,
        usage: wgpu::TextureUsages,
        filter_mode: wgpu::FilterMode,
        data: Vec<u8>,
    ) -> Self {
        Self {
            data: Arc::new(LazyTextureData {
                id: sealed::TextureId::new_unique_id(),
                size,
                format,
                usage,
                filter_mode,
                loaded_by_device: Mutex::default(),
                data,
            }),
            last_loaded: Mutex::default(),
        }
    }

    /// Returns a texture that loads `image` into the gpu when it is used.
    #[must_use]
    #[cfg(feature = "image")]
    pub fn from_image(image: image::DynamicImage, filter_mode: wgpu::FilterMode) -> Self {
        let image = image.into_rgba8();
        Self::from_data(
            Size::upx(image.width(), image.height()),
            wgpu::TextureFormat::Rgba8UnormSrgb,
            wgpu::TextureUsages::TEXTURE_BINDING,
            filter_mode,
            image.into_raw(),
        )
    }

    /// Loads this texture to `graphics`, if needed, returning a
    /// [`SharedTexture`].
    #[must_use]
    pub fn upgrade(&self, graphics: &impl sealed::KludgineGraphics) -> SharedTexture {
        let mut last_loaded = self.last_loaded.lock().assert("texture lock poisoned");
        if let Some(last_loaded) = &*last_loaded {
            if last_loaded.0 == graphics.id() {
                return last_loaded.1.clone();
            }
        }

        let mut loaded = self
            .data
            .loaded_by_device
            .lock()
            .assert("texture lock poisoned");

        if let Some(loaded) = loaded.get(&graphics.id()).and_then(Weak::upgrade) {
            return SharedTexture(loaded);
        }

        let wgpu = graphics.device().create_texture_with_data(
            graphics.queue(),
            &wgpu::TextureDescriptor {
                label: None,
                size: self.data.size.into(),
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: self.data.format,
                usage: self.data.usage,
                view_formats: &[],
            },
            &self.data.data,
        );
        let texture = SharedTexture::from(Texture {
            id: self.data.id,
            kludgine: graphics.id(),
            size: self.data.size,
            format: self.data.format,
            data: TextureInstance::from_wgpu(wgpu, false, self.data.filter_mode, graphics),
        });

        loaded.insert(graphics.id(), Arc::downgrade(&texture.0));
        *last_loaded = Some((graphics.id(), texture.clone()));

        texture
    }

    /// The size of the texture.
    #[must_use]
    pub fn size(&self) -> Size<UPx> {
        self.data.size
    }
}

impl Clone for LazyTexture {
    fn clone(&self) -> Self {
        Self {
            data: self.data.clone(),
            last_loaded: Mutex::default(),
        }
    }
}

impl CanRenderTo for LazyTexture {
    fn can_render_to(&self, _kludgine: &Kludgine) -> bool {
        true
    }
}

impl TextureSource for LazyTexture {}

impl sealed::TextureSource for LazyTexture {
    fn id(&self) -> sealed::TextureId {
        self.data.id
    }

    fn is_mask(&self) -> bool {
        // TODO this should be a flag on the texture.
        self.data.format == wgpu::TextureFormat::R8Unorm
    }

    fn bind_group(&self, graphics: &impl sealed::KludgineGraphics) -> Arc<wgpu::BindGroup> {
        self.upgrade(graphics).bind_group(graphics)
    }

    fn default_rect(&self) -> Rect<UPx> {
        self.data.size.into()
    }
}

#[derive(Debug)]
struct LazyTextureData {
    id: sealed::TextureId,
    size: Size<UPx>,
    format: wgpu::TextureFormat,
    usage: wgpu::TextureUsages,
    filter_mode: wgpu::FilterMode,
    loaded_by_device: Mutex<AHashMap<KludgineId, Weak<Texture>>>,
    data: Vec<u8>,
}

/// An image stored on the GPU.
#[derive(Debug)]
pub struct Texture {
    id: sealed::TextureId,
    kludgine: KludgineId,
    size: Size<UPx>,
    format: wgpu::TextureFormat,
    data: TextureInstance,
}

#[derive(Debug)]
struct TextureInstance {
    wgpu: wgpu::Texture,
    view: wgpu::TextureView,
    bind_group: Arc<wgpu::BindGroup>,
}

enum MaybeRef<'a, T> {
    Borrowed(&'a T),
    Owned(T),
}

impl<T> AsRef<T> for MaybeRef<'_, T> {
    fn as_ref(&self) -> &T {
        match self {
            MaybeRef::Borrowed(value) => value,
            MaybeRef::Owned(value) => value,
        }
    }
}

impl TextureInstance {
    fn from_wgpu(
        wgpu: wgpu::Texture,
        multisampled: bool,
        filter_mode: wgpu::FilterMode,
        graphics: &impl sealed::KludgineGraphics,
    ) -> Self {
        let view = wgpu.create_view(&wgpu::TextureViewDescriptor::default());
        let layout = if multisampled {
            MaybeRef::Owned(pipeline::bind_group_layout(graphics.device(), multisampled))
        } else {
            MaybeRef::Borrowed(graphics.binding_layout())
        };
        let bind_group = Arc::new(pipeline::bind_group(
            graphics.device(),
            layout.as_ref(),
            graphics.uniforms(),
            &view,
            match filter_mode {
                wgpu::FilterMode::Nearest => graphics.nearest_sampler(),
                wgpu::FilterMode::Linear => graphics.linear_sampler(),
            },
        ));
        TextureInstance {
            wgpu,
            view,
            bind_group,
        }
    }
}

impl Texture {
    fn from_wgpu(
        wgpu: wgpu::Texture,
        graphics: &impl KludgineGraphics,
        multisampled: bool,
        size: Size<UPx>,
        format: wgpu::TextureFormat,
        filter_mode: wgpu::FilterMode,
    ) -> Self {
        Self {
            id: sealed::TextureId::new_unique_id(),
            kludgine: graphics.id(),
            size,
            format,
            data: TextureInstance::from_wgpu(wgpu, multisampled, filter_mode, graphics),
        }
    }

    pub(crate) fn new_generic(
        graphics: &impl KludgineGraphics,
        multisample_count: u32,
        size: Size<UPx>,
        format: wgpu::TextureFormat,
        usage: wgpu::TextureUsages,
        filter_mode: wgpu::FilterMode,
    ) -> Self {
        let wgpu = graphics.device().create_texture(&wgpu::TextureDescriptor {
            label: None,
            size: size.into(),
            mip_level_count: 1,
            sample_count: multisample_count,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage,
            view_formats: &[],
        });
        Self::from_wgpu(
            wgpu,
            graphics,
            multisample_count > 1,
            size,
            format,
            filter_mode,
        )
    }

    /// Creates a new texture of the given size, format, and usages.
    #[must_use]
    pub fn new(
        graphics: &Graphics<'_>,
        size: Size<UPx>,
        format: wgpu::TextureFormat,
        usage: wgpu::TextureUsages,
        filter_mode: wgpu::FilterMode,
    ) -> Self {
        Self::multisampled(graphics, 1, size, format, usage, filter_mode)
    }

    /// Creates a new texture of the given multisample count, size, format, and usages.
    #[must_use]
    pub fn multisampled(
        graphics: &Graphics<'_>,
        multisample_count: u32,
        size: Size<UPx>,
        format: wgpu::TextureFormat,
        usage: wgpu::TextureUsages,
        filter_mode: wgpu::FilterMode,
    ) -> Self {
        Self::new_generic(
            graphics,
            multisample_count,
            size,
            format,
            usage,
            filter_mode,
        )
    }

    /// Returns a new texture of the given size, format, and usages. The texture
    /// is initialized with `data`. `data` must match `format`.
    #[must_use]
    pub fn new_with_data(
        graphics: &Graphics<'_>,
        size: Size<UPx>,
        format: wgpu::TextureFormat,
        usage: wgpu::TextureUsages,
        filter_mode: wgpu::FilterMode,
        data: &[u8],
    ) -> Self {
        let wgpu = graphics.device().create_texture_with_data(
            graphics.queue(),
            &wgpu::TextureDescriptor {
                label: None,
                size: size.into(),
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format,
                usage,
                view_formats: &[],
            },
            data,
        );
        Self::from_wgpu(wgpu, graphics, false, size, format, filter_mode)
    }

    /// Creates a texture from `image`.
    #[must_use]
    #[cfg(feature = "image")]
    pub fn from_image(
        image: image::DynamicImage,
        filter_mode: wgpu::FilterMode,
        graphics: &Graphics<'_>,
    ) -> Self {
        // TODO is it better to force rgba8, or is it better to avoid the
        // conversion and allow multiple texture formats?
        let image = image.into_rgba8();
        Self::new_with_data(
            graphics,
            Size::upx(image.width(), image.height()),
            wgpu::TextureFormat::Rgba8UnormSrgb,
            wgpu::TextureUsages::TEXTURE_BINDING,
            filter_mode,
            image.as_raw(),
        )
    }

    /// Prepares to render this texture with `size`. The returned graphic will
    /// be oriented around `origin`.
    #[must_use]
    pub fn prepare_sized<Unit>(
        &self,
        origin: Origin<Unit>,
        size: Size<Unit>,
        graphics: &Graphics<'_>,
    ) -> PreparedGraphic<Unit>
    where
        Unit: figures::Unit + From<i32>,
        Point<Unit>: Div<Unit, Output = Point<Unit>> + Neg<Output = Point<Unit>>,
        Vertex<Unit>: bytemuck::Pod,
    {
        let origin = match origin {
            Origin::TopLeft => Point::default(),
            Origin::Center => -(Point::from_vec(size) / Unit::from(2)),
            Origin::Custom(point) => point,
        };
        self.prepare(Rect::new(origin, size), graphics)
    }

    /// Prepares to render this texture at the given location.
    #[must_use]
    pub fn prepare<Unit>(&self, dest: Rect<Unit>, graphics: &Graphics<'_>) -> PreparedGraphic<Unit>
    where
        Unit: figures::Unit,
        Vertex<Unit>: bytemuck::Pod,
    {
        self.prepare_partial(self.size().into(), dest, graphics)
    }

    /// Prepares the `source` area to be rendered at `dest`.
    #[must_use]
    pub fn prepare_partial<Unit>(
        &self,
        source: Rect<UPx>,
        dest: Rect<Unit>,
        graphics: &Graphics<'_>,
    ) -> PreparedGraphic<Unit>
    where
        Unit: figures::Unit,
        Vertex<Unit>: bytemuck::Pod,
    {
        TextureBlit::new(source, dest, Color::WHITE).prepare(Some(self), graphics)
    }

    /// The size of the texture.
    #[must_use]
    pub const fn size(&self) -> Size<UPx> {
        self.size
    }

    /// The format of the texture.
    #[must_use]
    pub const fn format(&self) -> wgpu::TextureFormat {
        self.format
    }

    /// Copies the contents of this texture into `destination`.
    pub fn copy_to_buffer(
        &self,
        destination: wgpu::ImageCopyBuffer<'_>,
        encoder: &mut wgpu::CommandEncoder,
    ) {
        self.copy_rect_to_buffer(self.default_rect(), destination, encoder);
    }

    /// Copies the contents of a portion of this texture into `destination`.
    pub fn copy_rect_to_buffer(
        &self,
        source: Rect<UPx>,
        destination: wgpu::ImageCopyBuffer<'_>,
        encoder: &mut wgpu::CommandEncoder,
    ) {
        encoder.copy_texture_to_buffer(
            wgpu::ImageCopyTexture {
                texture: &self.data.wgpu,
                mip_level: 0,
                origin: source.origin.into(),
                aspect: wgpu::TextureAspect::All,
            },
            destination,
            source.size.into(),
        );
    }

    /// Returns a view over the entire texture.
    #[must_use]
    pub const fn view(&self) -> &wgpu::TextureView {
        &self.data.view
    }
}

/// Loads a texture's bytes into the executable. This macro returns a result
/// containing a [`LazyTexture`].
///
/// This macro takes a single parameter, which is forwarded along to
/// [`include_bytes!`]. The bytes that are loaded are then parsed using
/// [`image::load_from_memory`] and loaded using [`LazyTexture::from_image`].
#[cfg(feature = "image")]
#[macro_export]
macro_rules! include_texture {
    ($path:expr) => {
        $crate::include_texture!($path, $crate::wgpu::FilterMode::Nearest)
    };
    ($path:expr, $filter_mode:expr) => {
        $crate::image::load_from_memory(std::include_bytes!($path))
            .map(|image| $crate::LazyTexture::from_image(image, $filter_mode))
    };
}

/// The origin of a prepared graphic.
#[derive(Default, Clone, Copy, Eq, PartialEq, Debug)]
pub enum Origin<Unit> {
    /// The graphic should be drawn so that the top-left of the graphic appears
    /// at the rendered location. When rotating the graphic, it will rotate
    /// around the top-left.
    #[default]
    TopLeft,
    /// The grapihc should be drawn so that the center of the graphic appears at
    /// the rendered location. When rotating the graphic, it will rotate around
    /// the center.
    Center,
    /// The graphic should be drawn so that the provided relative location
    /// appears at the rendered location. When rotating the graphic, it will
    /// rotate around this point.
    Custom(Point<Unit>),
}

/// A resource that can be checked for surface compatibility.
pub trait CanRenderTo {
    /// Returns `true` if this resource can be rendered into a graphics context
    /// for `kludgine`.
    #[must_use]
    fn can_render_to(&self, kludgine: &Kludgine) -> bool;
}

/// A type that is rendered using a texture.
pub trait TextureSource: CanRenderTo + sealed::TextureSource {}

impl CanRenderTo for Texture {
    fn can_render_to(&self, kludgine: &Kludgine) -> bool {
        self.kludgine == kludgine.id
    }
}

impl TextureSource for Texture {}

impl sealed::TextureSource for Texture {
    fn bind_group(&self, _graphics: &impl sealed::KludgineGraphics) -> Arc<wgpu::BindGroup> {
        self.data.bind_group.clone()
    }

    fn id(&self) -> sealed::TextureId {
        self.id
    }

    fn is_mask(&self) -> bool {
        // TODO this should be a flag on the texture.
        self.format == wgpu::TextureFormat::R8Unorm
    }

    fn default_rect(&self) -> Rect<UPx> {
        self.size().into()
    }
}

/// A cloneable texture.
#[derive(Clone, Debug)]
pub struct SharedTexture(Arc<Texture>);

impl Eq for SharedTexture {}

impl PartialEq for SharedTexture {
    fn eq(&self, other: &Self) -> bool {
        self.0.id == other.0.id
    }
}

impl From<Texture> for SharedTexture {
    fn from(value: Texture) -> Self {
        Self(Arc::new(value))
    }
}

impl Deref for SharedTexture {
    type Target = Texture;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// A texture that can be cloned cheaply.
#[derive(Clone, Debug)]
pub enum ShareableTexture {
    /// A shared texture instance.
    Shared(SharedTexture),
    /// A lazy texture that loads its contents on first use.
    Lazy(LazyTexture),
}

impl ShareableTexture {
    /// Returns the [`SharedTexture`] from this instance, loading it if
    /// necessary.
    pub fn texture(&self, graphics: &impl KludgineGraphics) -> Cow<'_, SharedTexture> {
        match self {
            ShareableTexture::Shared(texture) => Cow::Borrowed(texture),
            ShareableTexture::Lazy(texture) => Cow::Owned(texture.upgrade(graphics)),
        }
    }

    /// The size of the texture.
    #[must_use]
    pub fn size(&self) -> Size<UPx> {
        match self {
            ShareableTexture::Shared(texture) => texture.size(),
            ShareableTexture::Lazy(texture) => texture.size(),
        }
    }
}

impl Eq for ShareableTexture {}

impl PartialEq for ShareableTexture {
    fn eq(&self, other: &Self) -> bool {
        self.id() == other.id()
    }
}

impl CanRenderTo for ShareableTexture {
    fn can_render_to(&self, kludgine: &Kludgine) -> bool {
        match self {
            ShareableTexture::Shared(texture) => texture.can_render_to(kludgine),
            ShareableTexture::Lazy(texture) => texture.can_render_to(kludgine),
        }
    }
}

impl sealed::TextureSource for ShareableTexture {
    fn id(&self) -> sealed::TextureId {
        match self {
            ShareableTexture::Shared(texture) => texture.id(),
            ShareableTexture::Lazy(texture) => texture.id(),
        }
    }

    fn is_mask(&self) -> bool {
        match self {
            ShareableTexture::Shared(texture) => texture.is_mask(),
            ShareableTexture::Lazy(texture) => texture.is_mask(),
        }
    }

    fn bind_group(&self, graphics: &impl sealed::KludgineGraphics) -> Arc<wgpu::BindGroup> {
        match self {
            ShareableTexture::Shared(texture) => texture.bind_group(graphics),
            ShareableTexture::Lazy(texture) => texture.bind_group(graphics),
        }
    }

    fn default_rect(&self) -> Rect<UPx> {
        match self {
            ShareableTexture::Shared(texture) => texture.default_rect(),
            ShareableTexture::Lazy(texture) => texture.default_rect(),
        }
    }
}

impl From<Texture> for ShareableTexture {
    fn from(texture: Texture) -> Self {
        Self::from(SharedTexture::from(texture))
    }
}

impl From<SharedTexture> for ShareableTexture {
    fn from(texture: SharedTexture) -> Self {
        Self::Shared(texture)
    }
}

impl From<LazyTexture> for ShareableTexture {
    fn from(texture: LazyTexture) -> Self {
        Self::Lazy(texture)
    }
}

impl<'a, T> From<&'a T> for ShareableTexture
where
    T: Clone + Into<Self>,
{
    fn from(value: &'a T) -> Self {
        value.clone().into()
    }
}

/// A region of a [`SharedTexture`].
///
/// When this type is drawn, only a region of the source texture will be drawn.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TextureRegion {
    texture: ShareableTexture,
    region: Rect<UPx>,
}

impl TextureRegion {
    /// Returns a reference to this texture that only renders a region of the
    /// texture when drawn.
    #[must_use]
    pub fn new(texture: impl Into<ShareableTexture>, region: Rect<UPx>) -> Self {
        Self {
            texture: texture.into(),
            region,
        }
    }

    /// Returns the size of the region being drawn.
    #[must_use]
    pub const fn size(&self) -> Size<UPx> {
        self.region.size
    }

    /// Prepares to render this texture at the given location.
    #[must_use]
    pub fn prepare<Unit>(&self, dest: Rect<Unit>, graphics: &Graphics<'_>) -> PreparedGraphic<Unit>
    where
        Unit: figures::Unit,
        Vertex<Unit>: bytemuck::Pod,
    {
        self.texture
            .texture(graphics)
            .prepare_partial(self.region, dest, graphics)
    }
}

impl CanRenderTo for TextureRegion {
    fn can_render_to(&self, kludgine: &Kludgine) -> bool {
        self.texture.can_render_to(kludgine)
    }
}

impl TextureSource for TextureRegion {}

impl sealed::TextureSource for TextureRegion {
    fn id(&self) -> sealed::TextureId {
        self.texture.id()
    }

    fn is_mask(&self) -> bool {
        self.texture.is_mask()
    }

    fn bind_group(&self, graphics: &impl sealed::KludgineGraphics) -> Arc<wgpu::BindGroup> {
        self.texture.bind_group(graphics)
    }

    fn default_rect(&self) -> Rect<UPx> {
        self.region
    }
}

impl From<SharedTexture> for TextureRegion {
    fn from(texture: SharedTexture) -> Self {
        Self::from(ShareableTexture::from(texture))
    }
}

impl From<LazyTexture> for TextureRegion {
    fn from(texture: LazyTexture) -> Self {
        Self::from(ShareableTexture::from(texture))
    }
}

impl From<ShareableTexture> for TextureRegion {
    fn from(texture: ShareableTexture) -> Self {
        Self {
            region: texture.default_rect(),
            texture,
        }
    }
}

/// A type that can be any [`TextureSource`] implementation that is provided by
/// Kludgine.
///
/// This type is useful if you are designing a type that supports drawing a
/// configurable texture, but you don't care whether it's a [`Texture`],
/// [`SharedTexture`], [`TextureRegion`], or [`CollectedTexture`].
#[derive(Debug)]
pub enum AnyTexture {
    /// A [`Texture`].
    Texture(Texture),
    /// A [`LazyTexture`].
    Lazy(LazyTexture),
    /// A [`SharedTexture`].
    Shared(SharedTexture),
    /// A [`TextureRegion`].
    Region(TextureRegion),
    /// A [`CollectedTexture`].
    Collected(CollectedTexture),
}

impl From<Texture> for AnyTexture {
    fn from(texture: Texture) -> Self {
        Self::Texture(texture)
    }
}

impl From<LazyTexture> for AnyTexture {
    fn from(texture: LazyTexture) -> Self {
        Self::Lazy(texture)
    }
}

impl From<SharedTexture> for AnyTexture {
    fn from(texture: SharedTexture) -> Self {
        Self::Shared(texture)
    }
}

impl From<TextureRegion> for AnyTexture {
    fn from(texture: TextureRegion) -> Self {
        Self::Region(texture)
    }
}

impl From<CollectedTexture> for AnyTexture {
    fn from(texture: CollectedTexture) -> Self {
        Self::Collected(texture)
    }
}

impl From<ShareableTexture> for AnyTexture {
    fn from(texture: ShareableTexture) -> Self {
        match texture {
            ShareableTexture::Shared(texture) => Self::Shared(texture),
            ShareableTexture::Lazy(texture) => Self::Lazy(texture),
        }
    }
}

impl AnyTexture {
    /// Returns the size of the texture.
    pub fn size(&self) -> Size<UPx> {
        self.default_rect().size
    }
}

impl CanRenderTo for AnyTexture {
    fn can_render_to(&self, kludgine: &Kludgine) -> bool {
        match self {
            AnyTexture::Texture(texture) => texture.can_render_to(kludgine),
            AnyTexture::Lazy(texture) => texture.can_render_to(kludgine),
            AnyTexture::Collected(texture) => texture.can_render_to(kludgine),
            AnyTexture::Shared(texture) => texture.can_render_to(kludgine),
            AnyTexture::Region(texture) => texture.can_render_to(kludgine),
        }
    }
}

impl TextureSource for AnyTexture {}

impl sealed::TextureSource for AnyTexture {
    fn id(&self) -> sealed::TextureId {
        match self {
            AnyTexture::Texture(texture) => texture.id(),
            AnyTexture::Lazy(texture) => texture.id(),
            AnyTexture::Collected(texture) => texture.id(),
            AnyTexture::Shared(texture) => texture.id(),
            AnyTexture::Region(texture) => texture.id(),
        }
    }

    fn is_mask(&self) -> bool {
        match self {
            AnyTexture::Texture(texture) => texture.is_mask(),
            AnyTexture::Lazy(texture) => texture.is_mask(),
            AnyTexture::Collected(texture) => texture.is_mask(),
            AnyTexture::Shared(texture) => texture.is_mask(),
            AnyTexture::Region(texture) => texture.is_mask(),
        }
    }

    fn bind_group(&self, graphics: &impl sealed::KludgineGraphics) -> Arc<wgpu::BindGroup> {
        match self {
            AnyTexture::Texture(texture) => texture.bind_group(graphics),
            AnyTexture::Lazy(texture) => texture.bind_group(graphics),
            AnyTexture::Collected(texture) => texture.bind_group(graphics),
            AnyTexture::Shared(texture) => texture.bind_group(graphics),
            AnyTexture::Region(texture) => texture.bind_group(graphics),
        }
    }

    fn default_rect(&self) -> Rect<UPx> {
        match self {
            AnyTexture::Texture(texture) => texture.default_rect(),
            AnyTexture::Lazy(texture) => texture.default_rect(),
            AnyTexture::Collected(texture) => texture.default_rect(),
            AnyTexture::Shared(texture) => texture.default_rect(),
            AnyTexture::Region(texture) => texture.default_rect(),
        }
    }
}

#[derive(Default)]
struct DefaultHasher(AHasher);

impl BuildHasher for DefaultHasher {
    type Hasher = AHasher;

    fn build_hasher(&self) -> Self::Hasher {
        self.0.clone()
    }
}

#[derive(Default, Debug)]
struct VertexCollection<T> {
    vertices: Vec<Vertex<T>>,
    vertex_index_by_id: HashMap<VertexId, u32, DefaultHasher>,
}

impl<T> VertexCollection<T> {
    fn get_or_insert(&mut self, vertex: Vertex<T>) -> u32
    where
        T: Copy,
        Vertex<T>: Into<Vertex<i32>>,
    {
        *self
            .vertex_index_by_id
            .entry(VertexId(vertex.into()))
            .or_insert_with(|| {
                let index = self
                    .vertices
                    .len()
                    .try_into()
                    .expect("too many drawn verticies");
                self.vertices.push(vertex);
                index
            })
    }
}

#[derive(Eq, PartialEq, Debug, Clone, Copy)]
struct VertexId(Vertex<i32>);

impl hash::Hash for VertexId {
    fn hash<H: hash::Hasher>(&self, state: &mut H) {
        bytemuck::bytes_of(&self.0).hash(state);
    }
}

/// A source of triangle data for a shape.
pub trait ShapeSource<Unit, const TEXTURED: bool>:
    DrawableSource + sealed::ShapeSource<Unit>
{
}

impl<Unit> ShapeSource<Unit, true> for TextureBlit<Unit> where Unit: Add<Output = Unit> + Ord + Copy {}
impl<Unit> DrawableSource for TextureBlit<Unit> where Unit: Add<Output = Unit> + Ord + Copy {}

impl<Unit> sealed::ShapeSource<Unit> for TextureBlit<Unit>
where
    Unit: Add<Output = Unit> + Ord + Copy,
{
    fn vertices(&self) -> &[Vertex<Unit>] {
        &self.verticies
    }

    fn indices(&self) -> &[u32] {
        &[1, 0, 2, 1, 2, 3]
    }
}

#[derive(Clone, Copy, Debug)]
struct TextureBlit<Unit> {
    verticies: [Vertex<Unit>; 4],
}

#[cfg_attr(not(feature = "cosmic-text"), allow(dead_code))]
impl<Unit> TextureBlit<Unit> {
    pub fn new(source: Rect<UPx>, dest: Rect<Unit>, color: Color) -> Self
    where
        Unit: Add<Output = Unit> + Ord + Copy + Default,
    {
        let color = srgb_to_linear(
            color.red_f32(),
            color.green_f32(),
            color.blue_f32(),
            color.alpha_f32(),
        );
        let (dest_top_left, dest_bottom_right) = dest.extents();
        let (source_top_left, source_bottom_right) = source.extents();
        Self {
            verticies: [
                Vertex {
                    location: dest_top_left,
                    texture: source_top_left,
                    color,
                },
                Vertex {
                    location: Point::new(dest_bottom_right.x, dest_top_left.y),
                    texture: Point::new(source_bottom_right.x, source_top_left.y),
                    color,
                },
                Vertex {
                    location: Point::new(dest_top_left.x, dest_bottom_right.y),
                    texture: Point::new(source_top_left.x, source_bottom_right.y),
                    color,
                },
                Vertex {
                    location: dest_bottom_right,
                    texture: source_bottom_right,
                    color,
                },
            ],
        }
    }

    pub const fn top_left(&self) -> &Vertex<Unit> {
        &self.verticies[0]
    }

    // pub const fn top_right(&self) -> &Vertex<Unit> {
    //     &self.verticies[1]
    // }

    // pub const fn bottom_left(&self) -> &Vertex<Unit> {
    //     &self.verticies[2]
    // }

    pub const fn bottom_right(&self) -> &Vertex<Unit> {
        &self.verticies[3]
    }

    pub fn translate_by(&mut self, offset: Point<Unit>)
    where
        Unit: AddAssign + Copy,
    {
        for vertex in &mut self.verticies {
            vertex.location += offset;
        }
    }
}

/// A type that can be drawn in Kludgine.
pub trait DrawableSource {}

/// A drawable source with optional translation, rotation, and scaling.
pub struct Drawable<T, Unit> {
    /// The source to draw.
    pub source: T,
    /// Translate the source before rendering.
    pub translation: Point<Unit>,
    /// Rotate the source before rendering.
    pub rotation: Option<Angle>,
    /// Scale the source before rendering.
    pub scale: Option<f32>,
    /// An opacity multiplier to apply to this drawable.
    pub opacity: Option<f32>,
}

impl<'a, Unit> From<Text<'a, Unit>> for Drawable<Text<'a, Unit>, Unit>
where
    Unit: Default,
{
    fn from(what: Text<'a, Unit>) -> Self {
        Self {
            source: what,
            translation: Point::default(),
            rotation: None,
            scale: None,
            opacity: None,
        }
    }
}

impl<'a, T, Unit> From<&'a T> for Drawable<&'a T, Unit>
where
    T: DrawableSource,
    Unit: Default,
{
    fn from(what: &'a T) -> Self {
        Self {
            source: what,
            translation: Point::default(),
            rotation: None,
            scale: None,
            opacity: None,
        }
    }
}

/// Translation, rotation, and scaling for drawable types.
pub trait DrawableExt<Source, Unit> {
    /// Translates `self` by `point`.
    fn translate_by(self, point: Point<Unit>) -> Drawable<Source, Unit>;
    /// Rotates `self` by `angle`.
    fn rotate_by(self, angle: Angle) -> Drawable<Source, Unit>;
    /// Scales `self` by `factor`.
    fn scale(self, factor: f32) -> Drawable<Source, Unit>;
    /// Renders this drawable with `opacity`, ranged from 0.- to 1.0.
    fn opacity(self, opacity: f32) -> Drawable<Source, Unit>;
}

impl<T, Unit> DrawableExt<T, Unit> for Drawable<T, Unit> {
    fn translate_by(mut self, point: Point<Unit>) -> Drawable<T, Unit> {
        self.translation = point;
        self
    }

    fn rotate_by(mut self, angle: Angle) -> Drawable<T, Unit> {
        self.rotation = Some(angle);
        self
    }

    fn scale(mut self, factor: f32) -> Drawable<T, Unit> {
        self.scale = Some(factor);
        self
    }

    fn opacity(mut self, opacity: f32) -> Drawable<T, Unit> {
        self.opacity = Some(opacity.clamp(0., 1.));
        self
    }
}

impl<T, Unit> DrawableExt<T, Unit> for T
where
    Drawable<T, Unit>: From<T>,
    Unit: Default,
{
    fn translate_by(self, point: Point<Unit>) -> Drawable<T, Unit> {
        Drawable::from(self).translate_by(point)
    }

    fn rotate_by(self, angle: Angle) -> Drawable<T, Unit> {
        Drawable::from(self).rotate_by(angle)
    }

    fn scale(self, factor: f32) -> Drawable<T, Unit> {
        Drawable::from(self).scale(factor)
    }

    fn opacity(self, opacity: f32) -> Drawable<T, Unit> {
        Drawable::from(self).opacity(opacity)
    }
}
