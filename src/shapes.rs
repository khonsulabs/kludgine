use std::ops::Add;

use figures::traits::FloatConversion;
use figures::units::UPx;
use figures::{Point, Rect};
use lyon_tessellation::{
    FillGeometryBuilder, FillOptions, FillTessellator, FillVertex, FillVertexConstructor,
    GeometryBuilder, GeometryBuilderError, StrokeGeometryBuilder, StrokeVertex,
    StrokeVertexConstructor, VertexId,
};

use crate::buffer::Buffer;
use crate::pipeline::{PreparedCommand, Vertex};
use crate::{Color, Graphics, PreparedGraphic, TextureSource};

/// A tesselated shape.
///
/// This structure contains geometry that has been divided into triangles, ready
/// to upload to the GPU. To render the shape, it must first be
/// [prepared](Self::prepare).
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
    /// Returns a rectangle that is filled solid with `color`.
    pub fn filled_rect(rect: Rect<Unit>, color: Color) -> Shape<Unit, false>
    where
        Unit: Add<Output = Unit> + Ord + FloatConversion<Float = f32> + Copy,
    {
        let (p1, p2) = rect.extents();
        let path = PathBuilder::new(p1)
            .line_to(Point::new(p2.x, p1.y))
            .line_to(p2)
            .line_to(Point::new(p1.x, p2.y))
            .close();
        path.fill(color)
    }

    /// Uploads the shape to the GPU.
    #[must_use]
    pub fn prepare(&self, graphics: &Graphics<'_>) -> PreparedGraphic<Unit>
    where
        Vertex<Unit>: bytemuck::Pod,
    {
        let vertices = Buffer::new(
            &self.vertices,
            wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            graphics.device,
        );
        let indices = Buffer::new(
            &self.indices,
            wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
            graphics.device,
        );
        PreparedGraphic {
            vertices,
            indices,
            commands: vec![PreparedCommand {
                indices: 0..self
                    .indices
                    .len()
                    .try_into()
                    .expect("too many drawn indices"),
                is_mask: false,
                binding: None,
            }],
        }
    }
}

impl<Unit> Shape<Unit, true> {
    /// Uploads the shape to the GPU, applying `texture` to the polygons.
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
            wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            graphics.device,
        );
        let indices = Buffer::new(
            &self.indices,
            wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
            graphics.device,
        );
        PreparedGraphic {
            vertices,
            indices,
            commands: vec![PreparedCommand {
                indices: 0..self
                    .indices
                    .len()
                    .try_into()
                    .expect("too many drawn indices"),
                is_mask: false,
                binding: Some(texture.bind_group()),
            }],
        }
    }
}

struct ShapeBuilder<Unit, const TEXTURED: bool> {
    shape: Shape<Unit, TEXTURED>,
    default_color: Color,
}

impl<Unit, const TEXTURED: bool> ShapeBuilder<Unit, TEXTURED>
where
    Unit: FloatConversion<Float = f32>,
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
        let new_id = VertexId(
            self.shape
                .vertices
                .len()
                .try_into()
                .map_err(|_| GeometryBuilderError::TooManyVertices)?,
        );
        self.shape.vertices.push(vertex);
        if self.shape.vertices.len() > u16::MAX as usize {
            return Err(GeometryBuilderError::TooManyVertices);
        }

        Ok(new_id)
    }
}

impl<Unit, const TEXTURED: bool> FillVertexConstructor<Vertex<Unit>>
    for ShapeBuilder<Unit, TEXTURED>
where
    Unit: FloatConversion<Float = f32>,
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
    Unit: FloatConversion<Float = f32>,
{
    fn new_vertex(&mut self, mut vertex: StrokeVertex) -> Vertex<Unit> {
        let position = vertex.position();
        let attributes = vertex.interpolated_attributes();
        self.new_vertex(position, attributes)
    }
}

impl<Unit, const TEXTURED: bool> FillGeometryBuilder for ShapeBuilder<Unit, TEXTURED>
where
    Unit: FloatConversion<Float = f32>,
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
    Unit: FloatConversion<Float = f32>,
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
    Unit: FloatConversion<Float = f32>,
{
    fn begin_geometry(&mut self) {}

    fn end_geometry(&mut self) {}

    fn add_triangle(&mut self, a: VertexId, b: VertexId, c: VertexId) {
        self.shape
            .indices
            .push(a.0.try_into().expect("checked in new_vertex"));
        self.shape
            .indices
            .push(b.0.try_into().expect("checked in new_vertex"));
        self.shape
            .indices
            .push(c.0.try_into().expect("checked in new_vertex"));
    }

    fn abort_geometry(&mut self) {
        self.shape.vertices.clear();
        self.shape.indices.clear();
    }
}

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
        /// The texture coordinate for this path event.
        texture: Point<UPx>,
    },
    /// A straight line segment.
    Line {
        /// The end location of the line.
        to: Endpoint<Unit>,
        /// The texture coordinate for this path event.
        texture: Point<UPx>,
    },
    /// A quadratic curve (one control point).
    Quadratic {
        /// The control point for the curve.
        ctrl: ControlPoint<Unit>,
        /// The end location of the curve.
        to: Endpoint<Unit>,
        /// The texture coordinate for this path event.
        texture: Point<UPx>,
    },
    /// A cubic curve (two control points).
    Cubic {
        /// The first control point for the curve.
        ctrl1: ControlPoint<Unit>,
        /// The second control point for the curve.
        ctrl2: ControlPoint<Unit>,
        /// The end location of the curve.
        to: Endpoint<Unit>,
        /// The texture coordinate for this path event.
        texture: Point<UPx>,
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
    Unit: FloatConversion<Float = f32> + Copy,
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

    /// Fills this path with `color`.
    ///
    /// If this is a textured image, the sampled texture colors will be
    /// multiplied with this color. To render the image unchanged, use
    /// [`Color::WHITE`].
    #[must_use]
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

/// Builds a [`Path`].
pub struct PathBuilder<Unit, const TEXTURED: bool> {
    path: Path<Unit, TEXTURED>,
    current_location: Endpoint<Unit>,
    close: bool,
}

impl<Unit, const TEXTURED: bool> From<Path<Unit, TEXTURED>> for PathBuilder<Unit, TEXTURED>
where
    Unit: Default,
{
    fn from(mut path: Path<Unit, TEXTURED>) -> Self {
        path.events.clear();
        Self {
            path,
            current_location: Point::default(),
            close: false,
        }
    }
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

    /// Clears this builder to a state as if it had just been created with
    /// [`new()`](Self::new).
    pub fn reset(&mut self, start_at: Endpoint<Unit>) {
        self.current_location = start_at;
        let begin = PathEvent::Begin {
            at: start_at,
            texture: Point::default(),
        };
        if self.path.events.is_empty() {
            self.path.events.push(begin);
        } else {
            self.path.events.truncate(1);
            self.path.events[0] = begin;
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
    pub fn new_textured(start_at: Endpoint<Unit>, texture: Point<UPx>) -> Self {
        Self {
            path: Path::from_iter([(PathEvent::Begin {
                at: start_at,
                texture,
            })]),
            current_location: start_at,
            close: false,
        }
    }

    /// Clears this builder to a state as if it had just been created with
    /// [`new_textured()`](Self::new_textured).
    pub fn reset(&mut self, start_at: Endpoint<Unit>, texture: Point<UPx>) {
        self.current_location = start_at;
        let begin = PathEvent::Begin {
            at: start_at,
            texture,
        };
        if self.path.events.is_empty() {
            self.path.events.push(begin);
        } else {
            self.path.events.truncate(1);
            self.path.events[0] = begin;
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
    pub fn line_to(mut self, end_at: Endpoint<Unit>, texture: Point<UPx>) -> Self {
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
        texture: Point<UPx>,
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
        texture: Point<UPx>,
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
