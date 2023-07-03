use std::marker::PhantomData;
use std::ops::Add;
use std::sync::Arc;

use bytemuck::{Pod, Zeroable};
use lyon_tessellation::{
    FillGeometryBuilder, FillOptions, FillTessellator, FillVertex, FillVertexConstructor,
    GeometryBuilder, GeometryBuilderError, StrokeGeometryBuilder, StrokeVertex,
    StrokeVertexConstructor, VertexId,
};
use wgpu::{BufferUsages, ShaderStages};

use crate::buffer::Buffer;
use crate::math::{Dips, Pixels, Point, Rect, ToFloat, UPixels, Zero};
use crate::{sealed, Color, Graphics, RenderingGraphics, TextureSource};

#[derive(Debug, Clone, PartialEq)]
pub struct Shape<Unit, const TEXTURED: bool> {
    pub(crate) vertices: Vec<Vertex<Unit>>,
    pub(crate) indices: Vec<u16>,
}

impl<Unit, const TEXTURED: bool> Shape<Unit, TEXTURED> {
    const fn new() -> Self {
        Self {
            vertices: Vec::new(),
            indices: Vec::new(),
        }
    }
}

impl<Unit> Shape<Unit, false> {
    pub fn prepare(&self, graphics: &Graphics<'_>) -> PreparedGraphic<Unit>
    where
        Vertex<Unit>: bytemuck::Pod,
    {
        let vertices = Buffer::new(
            &self.vertices,
            BufferUsages::VERTEX | BufferUsages::COPY_DST,
            graphics.device,
        );
        let indices = Buffer::new(
            &self.indices,
            BufferUsages::INDEX | BufferUsages::COPY_DST,
            graphics.device,
        );
        PreparedGraphic {
            vertices,
            indices,
            texture_binding: None,
            _unit: PhantomData,
        }
    }
}

impl<Unit> Shape<Unit, true> {
    pub fn prepare(
        &self,
        texture: &impl TextureSource,
        graphics: &Graphics<'_>,
    ) -> PreparedGraphic<Unit>
    where
        Vertex<Unit>: bytemuck::Pod,
    {
        let vertices = Buffer::new(
            &self.vertices,
            BufferUsages::VERTEX | BufferUsages::COPY_DST,
            graphics.device,
        );
        let indices = Buffer::new(
            &self.indices,
            BufferUsages::INDEX | BufferUsages::COPY_DST,
            graphics.device,
        );
        PreparedGraphic {
            vertices,
            indices,
            texture_binding: Some(texture.bind_group(graphics)),
            _unit: PhantomData,
        }
    }
}

pub(crate) const FLAG_DIPS: u32 = 1 << 0;
pub(crate) const FLAG_SCALE: u32 = 1 << 1;
pub(crate) const FLAG_ROTATE: u32 = 1 << 2;
pub(crate) const FLAG_TRANSLATE: u32 = 1 << 3;
pub(crate) const FLAG_TEXTURED: u32 = 1 << 4;

#[derive(Debug, Clone, Copy, Pod, Zeroable)]
#[repr(C)]
pub(crate) struct PushConstants {
    pub flags: u32,
    pub scale: f32,
    pub rotation: f32,
    pub translation: Point<i32>,
}

#[derive(Debug)]
pub struct PreparedGraphic<Unit> {
    texture_binding: Option<Arc<wgpu::BindGroup>>,
    vertices: Buffer<Vertex<Unit>>,
    indices: Buffer<u16>,
    _unit: PhantomData<Unit>,
}

impl<Unit> PreparedGraphic<Unit>
where
    Unit: Default + Into<i32> + ShaderScalable + Zero,
    Vertex<Unit>: Pod,
{
    pub fn render<'pass>(
        &'pass self,
        origin: Point<Unit>,
        scale: Option<f32>,
        rotation: Option<f32>,
        graphics: &mut RenderingGraphics<'_, 'pass>,
    ) {
        graphics.active_pipeline_if_needed();

        graphics.pass.set_bind_group(
            0,
            self.texture_binding
                .as_deref()
                .unwrap_or(&graphics.state.default_bindings),
            &[],
        );

        graphics.pass.set_vertex_buffer(0, self.vertices.as_slice());
        graphics
            .pass
            .set_index_buffer(self.indices.as_slice(), wgpu::IndexFormat::Uint16);
        let mut flags = Unit::flags();
        if self.texture_binding.is_some() {
            flags |= FLAG_TEXTURED;
        }
        let scale = scale.map_or(1., |scale| {
            flags |= FLAG_SCALE;
            scale
        });
        let rotation = rotation.map_or(0., |scale| {
            flags |= FLAG_ROTATE;
            scale
        });
        if !origin.is_zero() {
            flags |= FLAG_TRANSLATE;
        }

        graphics.pass.set_push_constants(
            ShaderStages::VERTEX | ShaderStages::FRAGMENT,
            0,
            bytemuck::bytes_of(&PushConstants {
                flags,
                scale,
                rotation,
                translation: Point {
                    x: origin.x.into(),
                    y: origin.y.into(),
                },
            }),
        );
        graphics
            .pass
            .draw_indexed(0..self.indices.len() as u32, 0, 0..1);
    }
}

pub trait ShaderScalable: sealed::ShaderScalableSealed {}

impl ShaderScalable for Pixels {}

impl ShaderScalable for Dips {}

impl sealed::ShaderScalableSealed for Pixels {
    fn flags() -> u32 {
        0
    }
}

impl sealed::ShaderScalableSealed for Dips {
    fn flags() -> u32 {
        FLAG_DIPS
    }
}

struct ShapeBuilder<Unit, const TEXTURED: bool> {
    shape: Shape<Unit, TEXTURED>,
    default_color: Color,
}

impl<Unit, const TEXTURED: bool> ShapeBuilder<Unit, TEXTURED>
where
    Unit: ToFloat<Float = f32>,
{
    fn new(default_color: Color) -> Self {
        Self {
            shape: Shape::new(),
            default_color,
        }
    }

    fn new_vertex(
        &mut self,
        position: lyon_tessellation::math::Point,
        attributes: &[f32],
    ) -> Vertex<Unit> {
        assert!(
            attributes.len() == 2,
            "Attributes should be texture coordinate"
        );

        Vertex {
            location: Point::new(Unit::from_float(position.x), Unit::from_float(position.y)),
            texture: Point::new(attributes[0].round(), attributes[1].round()),
            color: self.default_color,
        }
    }

    fn add_vertex(
        &mut self,
        position: lyon_tessellation::math::Point,
        attributes: &[f32],
    ) -> Result<VertexId, GeometryBuilderError> {
        let vertex = self.new_vertex(position, attributes);
        let new_id = VertexId(self.shape.vertices.len() as u32);
        self.shape.vertices.push(vertex);
        if self.shape.vertices.len() > u16::MAX as usize {
            return Err(GeometryBuilderError::TooManyVertices);
        }

        Ok(new_id)
    }
}

impl<Unit> Rect<Unit>
where
    Unit: Add<Output = Unit> + ToFloat<Float = f32> + Ord + Copy,
{
    pub fn fill(&self, color: Color) -> Shape<Unit, false> {
        let (p1, p2) = self.extents();
        let path = PathBuilder::new(p1)
            .line_to(Point::new(p2.x, p1.y))
            .line_to(p2)
            .line_to(Point::new(p1.x, p2.y))
            .close();
        path.fill(color)
    }
}

impl<Unit, const TEXTURED: bool> FillVertexConstructor<Vertex<Unit>>
    for ShapeBuilder<Unit, TEXTURED>
where
    Unit: ToFloat<Float = f32>,
{
    fn new_vertex(&mut self, mut vertex: FillVertex) -> Vertex<Unit> {
        let position = vertex.position();
        let attributes = vertex.interpolated_attributes();
        self.new_vertex(position, attributes)
    }
}

impl<Unit, const TEXTURED: bool> StrokeVertexConstructor<Vertex<Unit>>
    for ShapeBuilder<Unit, TEXTURED>
where
    Unit: ToFloat<Float = f32>,
{
    fn new_vertex(&mut self, mut vertex: StrokeVertex) -> Vertex<Unit> {
        let position = vertex.position();
        let attributes = vertex.interpolated_attributes();
        self.new_vertex(position, attributes)
    }
}

impl<Unit, const TEXTURED: bool> FillGeometryBuilder for ShapeBuilder<Unit, TEXTURED>
where
    Unit: ToFloat<Float = f32>,
{
    fn add_fill_vertex(
        &mut self,
        mut vertex: FillVertex,
    ) -> Result<VertexId, GeometryBuilderError> {
        let position = vertex.position();
        let attributes = vertex.interpolated_attributes();
        self.add_vertex(position, attributes)
    }
}

impl<Unit, const TEXTURED: bool> StrokeGeometryBuilder for ShapeBuilder<Unit, TEXTURED>
where
    Unit: ToFloat<Float = f32>,
{
    fn add_stroke_vertex(
        &mut self,
        mut vertex: StrokeVertex,
    ) -> Result<VertexId, GeometryBuilderError> {
        let position = vertex.position();
        let attributes = vertex.interpolated_attributes();
        self.add_vertex(position, attributes)
    }
}

impl<Unit, const TEXTURED: bool> GeometryBuilder for ShapeBuilder<Unit, TEXTURED>
where
    Unit: ToFloat<Float = f32>,
{
    fn begin_geometry(&mut self) {}

    fn end_geometry(&mut self) {}

    fn add_triangle(&mut self, a: VertexId, b: VertexId, c: VertexId) {
        self.shape.indices.push(a.0 as u16);
        self.shape.indices.push(b.0 as u16);
        self.shape.indices.push(c.0 as u16);
    }

    fn abort_geometry(&mut self) {
        self.shape.vertices.clear();
        self.shape.indices.clear();
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[repr(C)]
pub struct Vertex<Unit> {
    pub location: Point<Unit>,
    pub texture: Point<UPixels>,
    pub color: Color,
}

#[test]
fn vertex_align() {
    assert_eq!(std::mem::size_of::<Vertex<Dips>>(), 20);
}

unsafe impl bytemuck::Pod for Vertex<Pixels> {}
unsafe impl bytemuck::Zeroable for Vertex<Pixels> {}
unsafe impl bytemuck::Pod for Vertex<Dips> {}
unsafe impl bytemuck::Zeroable for Vertex<Dips> {}
unsafe impl bytemuck::Pod for Vertex<i32> {}
unsafe impl bytemuck::Zeroable for Vertex<i32> {}

/// A point on a [`Path`].
pub type Endpoint<Unit> = Point<Unit>;
/// A control point used to create curves.
pub type ControlPoint<Unit> = Point<Unit>;

/// An entry in a [`Path`].
#[derive(Debug, Clone, Copy)]
pub enum PathEvent<Unit> {
    /// Begins a path. Must be at the start.
    Begin {
        /// The location to begin at.
        at: Endpoint<Unit>,
        texture: Point<UPixels>,
    },
    /// A straight line segment.
    Line {
        /// The end location of the line.
        to: Endpoint<Unit>,
        texture: Point<UPixels>,
    },
    /// A quadratic curve (one control point).
    Quadratic {
        /// The control point for the curve.
        ctrl: ControlPoint<Unit>,
        /// The end location of the curve.
        to: Endpoint<Unit>,
        texture: Point<UPixels>,
    },
    /// A cubic curve (two control points).
    Cubic {
        /// The first control point for the curve.
        ctrl1: ControlPoint<Unit>,
        /// The second control point for the curve.
        ctrl2: ControlPoint<Unit>,
        /// The end location of the curve.
        to: Endpoint<Unit>,
        texture: Point<UPixels>,
    },
    /// Ends the path. Must be the last entry.
    End {
        /// Whether the path should be closed.
        close: bool,
    },
}

/// A geometric shape defined by a path.
#[derive(Default, Debug, Clone)]
pub struct Path<Unit, const TEXTURED: bool> {
    events: Vec<PathEvent<Unit>>,
}

impl<Unit, const TEXTURED: bool> FromIterator<PathEvent<Unit>> for Path<Unit, TEXTURED> {
    fn from_iter<T: IntoIterator<Item = PathEvent<Unit>>>(iter: T) -> Self {
        Self {
            events: iter.into_iter().collect(),
        }
    }
}

impl<Unit, const TEXTURED: bool> Path<Unit, TEXTURED>
where
    Unit: ToFloat<Float = f32> + Copy,
{
    fn as_lyon(&self) -> lyon_tessellation::path::Path {
        let mut builder = lyon_tessellation::path::Path::builder_with_attributes(2);
        // TODO: Pre-reserve to minimize allocations?
        for &event in &self.events {
            match event {
                PathEvent::Begin { at, texture } => {
                    builder.begin(at.into(), &[texture.x.into_float(), texture.y.into_float()]);
                }
                PathEvent::Line { to, texture } => {
                    builder.line_to(to.into(), &[texture.x.into_float(), texture.y.into_float()]);
                }
                PathEvent::Quadratic { ctrl, to, texture } => {
                    builder.quadratic_bezier_to(
                        ctrl.into(),
                        to.into(),
                        &[texture.x.into_float(), texture.y.into_float()],
                    );
                }
                PathEvent::Cubic {
                    ctrl1,
                    ctrl2,
                    to,
                    texture,
                } => {
                    builder.cubic_bezier_to(
                        ctrl1.into(),
                        ctrl2.into(),
                        to.into(),
                        &[texture.x.into_float(), texture.y.into_float()],
                    );
                }
                PathEvent::End { close } => builder.end(close),
            }
        }
        builder.build()
    }

    pub fn fill(&self, color: Color) -> Shape<Unit, TEXTURED> {
        let lyon_path = self.as_lyon();
        let mut shape_builder = ShapeBuilder::new(color);
        let mut tesselator = FillTessellator::new();
        tesselator
            .tessellate_with_ids(
                lyon_path.id_iter(),
                &lyon_path,
                Some(&lyon_path),
                &FillOptions::DEFAULT,
                &mut shape_builder,
            )
            .expect("should not fail to tesselat4e a rect");
        shape_builder.shape
    }
}

impl<Unit> From<Point<Unit>> for lyon_tessellation::math::Point
where
    Unit: ToFloat<Float = f32>,
{
    fn from(value: Point<Unit>) -> Self {
        Self::new(value.x.into_float(), value.y.into_float())
    }
}

/// Builds a [`Path`].
pub struct PathBuilder<Unit, const TEXTURED: bool> {
    path: Path<Unit, TEXTURED>,
    current_location: Endpoint<Unit>,
    close: bool,
}

impl<Unit> PathBuilder<Unit, false>
where
    Unit: Copy,
{
    /// Creates a new path with the initial position `start_at`.
    #[must_use]
    pub fn new(start_at: Endpoint<Unit>) -> Self {
        Self {
            path: Path::from_iter([PathEvent::Begin {
                at: start_at,
                texture: Point::default(),
            }]),
            current_location: start_at,
            close: false,
        }
    }

    /// Returns the built path.
    #[must_use]
    pub fn build(mut self) -> Path<Unit, false> {
        self.path.events.push(PathEvent::End { close: self.close });
        self.path
    }

    /// Create a straight line from the current location to `end_at`.
    #[must_use]
    pub fn line_to(mut self, end_at: Endpoint<Unit>) -> Self {
        self.path.events.push(PathEvent::Line {
            to: end_at,
            texture: Point::default(),
        });
        self.current_location = end_at;
        self
    }

    /// Create a quadratic curve from the current location to `end_at` using
    /// `control` as the curve's control point.
    #[must_use]
    pub fn quadratic_curve_to(
        mut self,
        control: ControlPoint<Unit>,
        end_at: Endpoint<Unit>,
    ) -> Self {
        self.path.events.push(PathEvent::Quadratic {
            ctrl: control,
            to: end_at,
            texture: Point::default(),
        });
        self.current_location = end_at;
        self
    }

    /// Create a cubic curve from the current location to `end_at` using
    /// `control1` and `control2` as the curve's control points.
    #[must_use]
    pub fn cubic_curve_to(
        mut self,
        control1: ControlPoint<Unit>,
        control2: ControlPoint<Unit>,
        end_at: Endpoint<Unit>,
    ) -> Self {
        self.path.events.push(PathEvent::Cubic {
            ctrl1: control1,
            ctrl2: control2,
            to: end_at,
            texture: Point::default(),
        });
        self.current_location = end_at;
        self
    }

    /// Closes the path, connecting the current location to the shape's starting
    /// location.
    #[must_use]
    pub fn close(mut self) -> Path<Unit, false> {
        self.close = true;
        self.build()
    }
}

impl<Unit> PathBuilder<Unit, true>
where
    Unit: Copy,
{
    /// Creates a new path with the initial position `start_at`.
    #[must_use]
    pub fn new_textured(start_at: Endpoint<Unit>, texture: Point<UPixels>) -> Self {
        Self {
            path: Path::from_iter([(PathEvent::Begin {
                at: start_at,
                texture,
            })]),
            current_location: start_at,
            close: false,
        }
    }

    /// Returns the built path.
    #[must_use]
    pub fn build(mut self) -> Path<Unit, true> {
        self.path.events.push(PathEvent::End { close: self.close });
        self.path
    }

    /// Create a straight line from the current location to `end_at`.
    #[must_use]
    pub fn line_to(mut self, end_at: Endpoint<Unit>, texture: Point<UPixels>) -> Self {
        self.path.events.push(PathEvent::Line {
            to: end_at,
            texture,
        });
        self.current_location = end_at;
        self
    }

    /// Create a quadratic curve from the current location to `end_at` using
    /// `control` as the curve's control point.
    #[must_use]
    pub fn quadratic_curve_to(
        mut self,
        control: ControlPoint<Unit>,
        end_at: Endpoint<Unit>,
        texture: Point<UPixels>,
    ) -> Self {
        self.path.events.push(PathEvent::Quadratic {
            ctrl: control,
            to: end_at,
            texture,
        });
        self.current_location = end_at;
        self
    }

    /// Create a cubic curve from the current location to `end_at` using
    /// `control1` and `control2` as the curve's control points.
    #[must_use]
    pub fn cubic_curve_to(
        mut self,
        control1: ControlPoint<Unit>,
        control2: ControlPoint<Unit>,
        end_at: Endpoint<Unit>,
        texture: Point<UPixels>,
    ) -> Self {
        self.path.events.push(PathEvent::Cubic {
            ctrl1: control1,
            ctrl2: control2,
            to: end_at,
            texture,
        });
        self.current_location = end_at;
        self
    }

    /// Closes the path, connecting the current location to the shape's starting
    /// location.
    #[must_use]
    pub fn close(mut self) -> Path<Unit, true> {
        self.close = true;
        self.build()
    }
}
