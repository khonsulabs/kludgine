use std::fmt::Debug;
use std::ops::{Add, Div, Mul, Neg, Sub};

use figures::units::{Lp, Px, UPx};
use figures::{
    Angle, FloatConversion, FloatOrInt, PixelScaling, Point, Ranged, Rect, ScreenScale, Size, Zero,
};
use lyon_tessellation::geom::Arc;
use lyon_tessellation::{
    FillGeometryBuilder, FillOptions, FillTessellator, FillVertex, FillVertexConstructor,
    GeometryBuilder, GeometryBuilderError, StrokeGeometryBuilder, StrokeTessellator, StrokeVertex,
    StrokeVertexConstructor, VertexId,
};
pub use lyon_tessellation::{LineCap, LineJoin};
use smallvec::SmallVec;

use crate::pipeline::Vertex;
use crate::{
    sealed, Assert, Color, DrawableSource, Graphics, Origin, PreparedGraphic, ShapeSource, Texture,
    TextureSource,
};

/// A tesselated shape.
///
/// This structure contains geometry that has been divided into triangles, ready
/// to upload to the GPU. To render the shape, it must first be
/// [prepared](Self::prepare).
#[derive(Debug, Clone, PartialEq)]
pub struct Shape<Unit, const TEXTURED: bool> {
    pub(crate) vertices: SmallVec<[Vertex<Unit>; 6]>,
    pub(crate) indices: SmallVec<[u32; 20]>,
}

#[test]
fn shape_size() {
    assert_eq!(std::mem::size_of::<Shape<i32, true>>(), 216);
}

impl<Unit, const TEXTURED: bool> Default for Shape<Unit, TEXTURED> {
    fn default() -> Self {
        Self {
            vertices: SmallVec::new(),
            indices: SmallVec::new(),
        }
    }
}

impl<Unit: PixelScaling> Shape<Unit, false> {
    /// Returns a circle that is filled solid with `color`.
    pub fn filled_circle(radius: Unit, color: Color, origin: Origin<Unit>) -> Shape<Unit, false>
    where
        Unit: Default
            + Neg<Output = Unit>
            + Add<Output = Unit>
            + Ord
            + FloatConversion<Float = f32>
            + Copy,
    {
        let center = match origin {
            Origin::TopLeft => Point::new(radius, radius),
            Origin::Center => Point::default(),
            Origin::Custom(pt) => pt,
        };
        let mut shape_builder = ShapeBuilder::new(color);
        let mut tesselator = FillTessellator::new();
        tesselator
            .tessellate_circle(
                lyon_tessellation::math::point(center.x.into_float(), center.y.into_float()),
                radius.into_float(),
                &FillOptions::DEFAULT,
                &mut shape_builder,
            )
            .assert("should not fail to tesselat4e a rect");
        shape_builder.shape
    }

    /// Returns a circle that is stroked with `color` and `options`.
    pub fn stroked_circle(
        radius: Unit,
        color: Color,
        origin: Origin<Unit>,
        options: impl Into<StrokeOptions<Unit>>,
    ) -> Shape<Unit, false>
    where
        Unit: Default
            + Neg<Output = Unit>
            + Add<Output = Unit>
            + Ord
            + FloatConversion<Float = f32>
            + Copy,
    {
        let center = match origin {
            Origin::TopLeft => Point::new(radius, radius),
            Origin::Center => Point::default(),
            Origin::Custom(pt) => pt,
        };
        let mut shape_builder = ShapeBuilder::new(color);
        let mut tesselator = StrokeTessellator::new();
        tesselator
            .tessellate_circle(
                lyon_tessellation::math::point(center.x.into_float(), center.y.into_float()),
                radius.into_float(),
                &options.into().into(),
                &mut shape_builder,
            )
            .assert("should not fail to tesselat4e a rect");
        shape_builder.shape
    }

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

    /// Returns a rectangle that has its outline stroked with `color` and
    /// `options`.
    pub fn stroked_rect(
        rect: Rect<Unit>,
        options: impl Into<StrokeOptions<Unit>>,
    ) -> Shape<Unit, false>
    where
        Unit: Add<Output = Unit> + Ord + FloatConversion<Float = f32> + Copy,
    {
        let (p1, p2) = rect.extents();
        let path = PathBuilder::new(p1)
            .line_to(Point::new(p2.x, p1.y))
            .line_to(p2)
            .line_to(Point::new(p1.x, p2.y))
            .close();
        path.stroke(options)
    }

    /// Returns a rounded rectangle with the specified corner radii that is
    /// filled solid with `color`.
    pub fn filled_round_rect(
        rect: Rect<Unit>,
        corner_radius: impl Into<CornerRadii<Unit>>,
        color: Color,
    ) -> Shape<Unit, false>
    where
        Unit: Add<Output = Unit>
            + Sub<Output = Unit>
            + Div<Output = Unit>
            + Mul<f32, Output = Unit>
            + TryFrom<i32>
            + Ord
            + FloatConversion<Float = f32>
            + Copy,
        Unit::Error: Debug,
    {
        let path = Path::round_rect(rect, corner_radius);
        path.fill(color)
    }

    /// Returns a rounded rectangle with the specified corner radii that has its
    /// outline stroked with `color` and `options`.
    pub fn stroked_round_rect(
        rect: Rect<Unit>,
        corner_radius: impl Into<CornerRadii<Unit>>,
        options: impl Into<StrokeOptions<Unit>>,
    ) -> Shape<Unit, false>
    where
        Unit: Add<Output = Unit>
            + Sub<Output = Unit>
            + Div<Output = Unit>
            + Mul<f32, Output = Unit>
            + TryFrom<i32>
            + Ord
            + FloatConversion<Float = f32>
            + Copy,
        Unit::Error: Debug,
    {
        let path = Path::round_rect(rect, corner_radius);
        path.stroke(options)
    }

    /// Uploads the shape to the GPU.
    #[must_use]
    pub fn prepare(&self, graphics: &Graphics<'_>) -> PreparedGraphic<Unit>
    where
        Unit: Copy,
        Vertex<Unit>: bytemuck::Pod,
    {
        sealed::ShapeSource::prepare(self, Option::<&Texture>::None, graphics)
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
        Unit: Copy,
        Vertex<Unit>: bytemuck::Pod,
    {
        sealed::ShapeSource::prepare(self, Some(texture), graphics)
    }
}

impl<Unit, const TEXTURED: bool> ShapeSource<Unit, TEXTURED> for Shape<Unit, TEXTURED> where
    Unit: Copy
{
}

impl<Unit, const TEXTURED: bool> DrawableSource for Shape<Unit, TEXTURED> where Unit: Copy {}

impl<Unit, const TEXTURED: bool> sealed::ShapeSource<Unit> for Shape<Unit, TEXTURED>
where
    Unit: Copy,
{
    fn vertices(&self) -> &[Vertex<Unit>] {
        &self.vertices
    }

    fn indices(&self) -> &[u32] {
        &self.indices
    }
}

struct ShapeBuilder<Unit, const TEXTURED: bool> {
    shape: Shape<Unit, TEXTURED>,
    default_color: Color,
}

impl<Unit, const TEXTURED: bool> ShapeBuilder<Unit, TEXTURED>
where
    Unit: FloatConversion<Float = f32> + PixelScaling,
{
    fn new(default_color: Color) -> Self {
        Self {
            shape: Shape::default(),
            default_color,
        }
    }

    fn new_vertex(
        &mut self,
        position: lyon_tessellation::math::Point,
        attributes: &[f32],
    ) -> Vertex<Unit> {
        let texture = match attributes.len() {
            0 => Point::default(),
            2 => Point::new(attributes[0], attributes[1]).cast(),
            _ => unreachable!("Attributes should be empty or 2"),
        };

        Vertex {
            location: Point::new(Unit::from_float(position.x), Unit::from_float(position.y)),
            texture,
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
        if self.shape.vertices.len() > u32::MAX as usize {
            return Err(GeometryBuilderError::TooManyVertices);
        }

        Ok(new_id)
    }
}

impl<Unit, const TEXTURED: bool> FillVertexConstructor<Vertex<Unit>>
    for ShapeBuilder<Unit, TEXTURED>
where
    Unit: FloatConversion<Float = f32> + PixelScaling,
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
    Unit: FloatConversion<Float = f32> + PixelScaling,
{
    fn new_vertex(&mut self, mut vertex: StrokeVertex) -> Vertex<Unit> {
        self.new_vertex(vertex.position(), vertex.interpolated_attributes())
    }
}

impl<Unit, const TEXTURED: bool> FillGeometryBuilder for ShapeBuilder<Unit, TEXTURED>
where
    Unit: FloatConversion<Float = f32> + PixelScaling,
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
    Unit: FloatConversion<Float = f32> + figures::PixelScaling,
{
    fn add_stroke_vertex(
        &mut self,
        mut vertex: StrokeVertex,
    ) -> Result<VertexId, GeometryBuilderError> {
        self.add_vertex(vertex.position(), vertex.interpolated_attributes())
    }
}

impl<Unit, const TEXTURED: bool> GeometryBuilder for ShapeBuilder<Unit, TEXTURED>
where
    Unit: FloatConversion<Float = f32>,
{
    fn begin_geometry(&mut self) {}

    fn end_geometry(&mut self) {}

    fn add_triangle(&mut self, a: VertexId, b: VertexId, c: VertexId) {
        self.shape.indices.push(a.0);
        self.shape.indices.push(b.0);
        self.shape.indices.push(c.0);
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
    /// A small-vec of path events. Contains enough stack space to contain the
    /// path of a hexagon, because it's the bestagon.
    events: SmallVec<[PathEvent<Unit>; 7]>,
}

impl<Unit> Path<Unit, false> {
    /// Returns a path for a rounded rectangle with the given corner radii.
    ///
    /// All radius's will be limited to half of the largest side of the
    /// rectangle.
    pub fn round_rect(
        rect: Rect<Unit>,
        corner_radius: impl Into<CornerRadii<Unit>>,
    ) -> Path<Unit, false>
    where
        Unit: Add<Output = Unit>
            + Sub<Output = Unit>
            + Div<Output = Unit>
            + Mul<f32, Output = Unit>
            + TryFrom<i32>
            + Ord
            + FloatConversion<Float = f32>
            + Copy,
        Unit::Error: Debug,
    {
        const C: f32 = 0.551_915_02; // https://spencermortensen.com/articles/bezier-circle/
        let (top_left, bottom_right) = rect.extents();
        let top = top_left.y;
        let bottom = bottom_right.y;
        let left = top_left.x;
        let right = bottom_right.x;

        let min_dimension = if rect.size.width > rect.size.height {
            rect.size.height
        } else {
            rect.size.width
        };
        let radii = corner_radius
            .into()
            .clamped(min_dimension / Unit::try_from(2).assert("two is always convertable"));

        let drift = radii.map(|r| r * C);

        let start = Point::new(left + radii.top_left, top);
        PathBuilder::new(start)
            .line_to(Point::new(right - radii.top_right, top))
            .cubic_curve_to(
                Point::new(right + drift.top_right - radii.top_right, top),
                Point::new(right, top + radii.top_right - drift.top_right),
                Point::new(right, top + radii.top_right),
            )
            .line_to(Point::new(right, bottom - radii.bottom_right))
            .cubic_curve_to(
                Point::new(right, bottom + drift.bottom_right - radii.bottom_right),
                Point::new(right + drift.bottom_right - radii.bottom_right, bottom),
                Point::new(right - radii.bottom_right, bottom),
            )
            .line_to(Point::new(left + radii.bottom_left, bottom))
            .cubic_curve_to(
                Point::new(left + radii.bottom_left - drift.bottom_left, bottom),
                Point::new(left, bottom + drift.bottom_left - radii.bottom_left),
                Point::new(left, bottom - radii.bottom_left),
            )
            .line_to(Point::new(left, top + radii.top_left))
            .cubic_curve_to(
                Point::new(left, top + radii.top_left - drift.top_left),
                Point::new(left + radii.top_left - drift.top_left, top),
                start,
            )
            .close()
    }

    /// Returns a path forming an arc starting at `start` angle of an oval sized
    /// `radii` oriented around `center`. The arc will sweep in a clockwise
    /// direction a rotation of `sweep` angle.
    #[must_use]
    pub fn arc(center: Point<Unit>, radii: Size<Unit>, start: Angle, sweep: Angle) -> Self
    where
        Unit: FloatConversion<Float = f32>,
    {
        let mut events = SmallVec::new();
        Arc {
            center: lyon_tessellation::geom::point(center.x.into_float(), center.y.into_float()),
            radii: lyon_tessellation::geom::vector(
                radii.width.into_float(),
                radii.height.into_float(),
            ),
            start_angle: lyon_tessellation::geom::Angle::degrees(start.into_degrees()),
            sweep_angle: lyon_tessellation::geom::Angle::degrees(sweep.into_degrees()),
            x_rotation: lyon_tessellation::geom::Angle::degrees(0.),
        }
        .for_each_cubic_bezier(&mut |segment| {
            if events.is_empty() {
                events.push(PathEvent::Begin {
                    at: Point::new(segment.from.x, segment.from.y).map(Unit::from_float),
                    texture: Point::ZERO,
                });
            }
            events.push(PathEvent::Cubic {
                ctrl1: Point::new(segment.ctrl1.x, segment.ctrl1.y).map(Unit::from_float),
                ctrl2: Point::new(segment.ctrl2.x, segment.ctrl2.y).map(Unit::from_float),
                to: Point::new(segment.to.x, segment.to.y).map(Unit::from_float),
                texture: Point::ZERO,
            });
        });

        events.push(PathEvent::End {
            close: sweep == Angle::MAX,
        });
        Self { events }
    }
}

#[test]
fn path_size() {
    assert_eq!(std::mem::size_of::<PathEvent<i32>>(), 36);
    // This is a pretty big structure with the inline path events, but it allows
    // drawing most common polygons without heap allocations.
    assert_eq!(std::mem::size_of::<Path<i32, true>>(), 264);
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
    Unit: FloatConversion<Float = f32> + Copy + PixelScaling,
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
            .assert("should not fail to tesselat4e a rect");
        shape_builder.shape
    }

    /// Strokes this path with `color` and `options`.
    ///
    /// If this is a textured image, the sampled texture colors will be
    /// multiplied with this color. To render the image unchanged, use
    /// [`Color::WHITE`].
    #[must_use]
    pub fn stroke(&self, options: impl Into<StrokeOptions<Unit>>) -> Shape<Unit, TEXTURED> {
        let options = options.into();
        let mut shape_builder = ShapeBuilder::new(options.color);
        let lyon_path = self.as_lyon();
        let mut tesselator = StrokeTessellator::new();

        tesselator
            .tessellate_with_ids(
                lyon_path.id_iter(),
                &lyon_path,
                Some(&lyon_path),
                &options.into(),
                &mut shape_builder,
            )
            .assert("should not fail to tesselat4e a rect");
        shape_builder.shape
    }
}

/// Builds a [`Path`].
pub struct PathBuilder<Unit, const TEXTURED: bool> {
    path: Path<Unit, TEXTURED>,
    current_location: Endpoint<Unit>,
    close: bool,
}

impl<Unit> Default for PathBuilder<Unit, false>
where
    Unit: Default + Copy,
{
    fn default() -> Self {
        Self::new(Point::default())
    }
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

/// Options for stroking lines on a path.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StrokeOptions<Unit> {
    /// The color to apply to the stroke.
    pub color: Color,

    /// The width of the line.
    pub line_width: Unit,

    /// See the SVG specification.
    ///
    /// Default value: `LineJoin::Miter`.
    pub line_join: LineJoin,

    /// What cap to use at the start of each sub-path.
    ///
    /// Default value: `LineCap::Butt`.
    pub start_cap: LineCap,

    /// What cap to use at the end of each sub-path.
    ///
    /// Default value: `LineCap::Butt`.
    pub end_cap: LineCap,

    /// See the SVG specification.
    ///
    /// Must be greater than or equal to 1.0.
    /// Default value: `StrokeOptions::DEFAULT_MITER_LIMIT`.
    pub miter_limit: f32,

    /// Maximum allowed distance to the path when building an approximation.
    ///
    /// See [Flattening and tolerance](index.html#flattening-and-tolerance).
    /// Default value: `StrokeOptions::DEFAULT_TOLERANCE`.
    pub tolerance: f32,
}

impl<Unit> Default for StrokeOptions<Unit>
where
    Unit: DefaultStrokeWidth,
{
    fn default() -> Self {
        Self {
            color: Color::WHITE,
            line_width: Unit::default_stroke_width(),
            line_join: lyon_tessellation::StrokeOptions::DEFAULT_LINE_JOIN,
            start_cap: lyon_tessellation::StrokeOptions::DEFAULT_LINE_CAP,
            end_cap: lyon_tessellation::StrokeOptions::DEFAULT_LINE_CAP,
            miter_limit: lyon_tessellation::StrokeOptions::DEFAULT_MITER_LIMIT,
            tolerance: lyon_tessellation::StrokeOptions::DEFAULT_TOLERANCE,
        }
    }
}

impl<Unit> From<Color> for StrokeOptions<Unit>
where
    Unit: DefaultStrokeWidth,
{
    fn from(color: Color) -> Self {
        Self {
            color,
            ..Self::default()
        }
    }
}

impl<Unit> StrokeOptions<Unit> {
    /// Sets the color of this stroke and returns self.
    #[must_use]
    pub fn colored(mut self, color: Color) -> Self {
        self.color = color;
        self
    }

    /// Sets the line join style and returns self.
    #[must_use]
    pub fn line_join(mut self, join: LineJoin) -> Self {
        self.line_join = join;
        self
    }

    /// Sets the line cap style for the start of line segments and returns self.
    #[must_use]
    pub fn start_cap(mut self, cap: LineCap) -> Self {
        self.start_cap = cap;
        self
    }

    /// Sets the line cap style for the end of line segments and returns self.
    #[must_use]
    pub fn end_cap(mut self, cap: LineCap) -> Self {
        self.end_cap = cap;
        self
    }

    /// Sets the miter limit and returns self.
    #[must_use]
    pub fn miter_limit(mut self, limit: f32) -> Self {
        self.miter_limit = limit;
        self
    }
}

impl StrokeOptions<Px> {
    /// Returns the default options with a line width of `px`.
    #[must_use]
    pub fn px_wide(px: impl Into<Px>) -> Self {
        Self {
            line_width: px.into(),
            ..Self::default()
        }
    }
}

impl StrokeOptions<Lp> {
    /// Returns the default options with a line width of `lp`.
    #[must_use]
    pub fn lp_wide(lp: impl Into<Lp>) -> Self {
        Self {
            line_width: lp.into(),
            ..Self::default()
        }
    }

    /// Returns the default options with the line width specified in
    /// millimeters.
    #[must_use]
    pub fn mm_wide(mm: impl Into<FloatOrInt>) -> Self {
        Self::lp_wide(mm.into().into_mm())
    }

    /// Returns the default options with the line width specified in
    /// centimeters.
    #[must_use]
    pub fn cm_wide(cm: impl Into<FloatOrInt>) -> Self {
        Self::lp_wide(cm.into().into_cm())
    }

    /// Returns the default options with the line width specified in
    /// points (1/72 of an inch).
    #[must_use]
    pub fn points_wide(points: impl Into<FloatOrInt>) -> Self {
        Self::lp_wide(points.into().into_points())
    }

    /// Returns the default options with the line width specified in inches.
    #[must_use]
    pub fn inches_wide(inches: impl Into<FloatOrInt>) -> Self {
        Self::lp_wide(inches.into().into_inches())
    }
}

/// Controls the default stroke width for a given unit.
pub trait DefaultStrokeWidth {
    /// Returns the default width of a line stroked in this unit.
    fn default_stroke_width() -> Self;
}

impl DefaultStrokeWidth for figures::units::Lp {
    /// Returns [`Self::points(1)`].
    fn default_stroke_width() -> Self {
        Self::points(1)
    }
}
impl DefaultStrokeWidth for figures::units::Px {
    fn default_stroke_width() -> Self {
        Self::new(1)
    }
}
impl DefaultStrokeWidth for figures::units::UPx {
    fn default_stroke_width() -> Self {
        Self::new(1)
    }
}

impl<Unit> ScreenScale for StrokeOptions<Unit>
where
    Unit: ScreenScale<Px = Px, Lp = Lp, UPx = UPx>,
{
    type Lp = StrokeOptions<Lp>;
    type Px = StrokeOptions<Px>;
    type UPx = StrokeOptions<UPx>;

    fn into_px(self, scale: figures::Fraction) -> Self::Px {
        StrokeOptions {
            color: self.color,
            line_width: self.line_width.into_px(scale),
            line_join: self.line_join,
            start_cap: self.start_cap,
            end_cap: self.end_cap,
            miter_limit: self.miter_limit,
            tolerance: self.tolerance,
        }
    }

    fn from_px(px: Self::Px, scale: figures::Fraction) -> Self {
        Self {
            color: px.color,
            line_width: Unit::from_px(px.line_width, scale),
            line_join: px.line_join,
            start_cap: px.start_cap,
            end_cap: px.end_cap,
            miter_limit: px.miter_limit,
            tolerance: px.tolerance,
        }
    }

    fn into_lp(self, scale: figures::Fraction) -> Self::Lp {
        StrokeOptions {
            color: self.color,
            line_width: self.line_width.into_lp(scale),
            line_join: self.line_join,
            start_cap: self.start_cap,
            end_cap: self.end_cap,
            miter_limit: self.miter_limit,
            tolerance: self.tolerance,
        }
    }

    fn from_lp(lp: Self::Lp, scale: figures::Fraction) -> Self {
        Self {
            color: lp.color,
            line_width: Unit::from_lp(lp.line_width, scale),
            line_join: lp.line_join,
            start_cap: lp.start_cap,
            end_cap: lp.end_cap,
            miter_limit: lp.miter_limit,
            tolerance: lp.tolerance,
        }
    }

    fn into_upx(self, scale: crate::Fraction) -> Self::UPx {
        StrokeOptions {
            color: self.color,
            line_width: self.line_width.into_upx(scale),
            line_join: self.line_join,
            start_cap: self.start_cap,
            end_cap: self.end_cap,
            miter_limit: self.miter_limit,
            tolerance: self.tolerance,
        }
    }

    fn from_upx(upx: Self::UPx, scale: crate::Fraction) -> Self {
        StrokeOptions {
            color: upx.color,
            line_width: Unit::from_upx(upx.line_width, scale),
            line_join: upx.line_join,
            start_cap: upx.start_cap,
            end_cap: upx.end_cap,
            miter_limit: upx.miter_limit,
            tolerance: upx.tolerance,
        }
    }
}

impl<Unit> From<StrokeOptions<Unit>> for lyon_tessellation::StrokeOptions
where
    Unit: FloatConversion<Float = f32>,
{
    fn from(options: StrokeOptions<Unit>) -> Self {
        let StrokeOptions {
            line_width,
            line_join,
            start_cap,
            end_cap,
            miter_limit,
            tolerance,
            color: _color,
        } = options;
        Self::default()
            .with_line_width(line_width.into_float())
            .with_line_join(line_join)
            .with_start_cap(start_cap)
            .with_end_cap(end_cap)
            .with_miter_limit(miter_limit)
            .with_tolerance(tolerance)
    }
}

/// A description of the size to use for each corner radius measurement when
/// rendering a rounded rectangle.
#[derive(Debug, Eq, PartialEq, Clone, Copy)]
pub struct CornerRadii<Unit> {
    /// The radius of the top left rounded corner.
    pub top_left: Unit,
    /// The radius of the top right rounded corner.
    pub top_right: Unit,
    /// The radius of the bottom right rounded corner.
    pub bottom_right: Unit,
    /// The radius of the bottom left rounded corner.
    pub bottom_left: Unit,
}

impl<Unit> CornerRadii<Unit> {
    /// Passes each radius definition to `map` and returns a new set of radii
    /// with the results.
    #[must_use]
    pub fn map<UnitB>(self, mut map: impl FnMut(Unit) -> UnitB) -> CornerRadii<UnitB> {
        CornerRadii {
            top_left: map(self.top_left),
            top_right: map(self.top_right),
            bottom_right: map(self.bottom_right),
            bottom_left: map(self.bottom_left),
        }
    }
}

impl<Unit> ScreenScale for CornerRadii<Unit>
where
    Unit: ScreenScale<Lp = Lp, Px = Px, UPx = UPx>,
{
    type Lp = CornerRadii<Lp>;
    type Px = CornerRadii<Px>;
    type UPx = CornerRadii<UPx>;

    fn into_px(self, scale: figures::Fraction) -> Self::Px {
        self.map(|size| size.into_px(scale))
    }

    fn from_px(px: Self::Px, scale: figures::Fraction) -> Self {
        Self {
            top_left: Unit::from_px(px.top_left, scale),
            top_right: Unit::from_px(px.top_right, scale),
            bottom_right: Unit::from_px(px.bottom_right, scale),
            bottom_left: Unit::from_px(px.bottom_left, scale),
        }
    }

    fn into_upx(self, scale: figures::Fraction) -> Self::UPx {
        self.map(|size| size.into_upx(scale))
    }

    fn from_upx(px: Self::UPx, scale: figures::Fraction) -> Self {
        Self {
            top_left: Unit::from_upx(px.top_left, scale),
            top_right: Unit::from_upx(px.top_right, scale),
            bottom_right: Unit::from_upx(px.bottom_right, scale),
            bottom_left: Unit::from_upx(px.bottom_left, scale),
        }
    }

    fn into_lp(self, scale: figures::Fraction) -> Self::Lp {
        self.map(|size| size.into_lp(scale))
    }

    fn from_lp(lp: Self::Lp, scale: figures::Fraction) -> Self {
        Self {
            top_left: Unit::from_lp(lp.top_left, scale),
            top_right: Unit::from_lp(lp.top_right, scale),
            bottom_right: Unit::from_lp(lp.bottom_right, scale),
            bottom_left: Unit::from_lp(lp.bottom_left, scale),
        }
    }
}

impl<Unit> Zero for CornerRadii<Unit>
where
    Unit: Zero,
{
    const ZERO: Self = Self {
        top_left: Unit::ZERO,
        top_right: Unit::ZERO,
        bottom_right: Unit::ZERO,
        bottom_left: Unit::ZERO,
    };

    fn is_zero(&self) -> bool {
        self.top_left.is_zero()
            && self.top_right.is_zero()
            && self.bottom_right.is_zero()
            && self.bottom_left.is_zero()
    }
}

impl<Unit> CornerRadii<Unit>
where
    Unit: PartialOrd + Copy,
{
    fn clamp_size(size: Unit, clamp: Unit) -> Unit {
        if size > clamp {
            clamp
        } else {
            size
        }
    }

    /// Returns this set of radii clamped so that no corner radius has a width
    /// or height larger than `size`'s.
    #[must_use]
    pub fn clamped(mut self, size: Unit) -> Self {
        self.top_left = Self::clamp_size(self.top_left, size);
        self.top_right = Self::clamp_size(self.top_right, size);
        self.bottom_right = Self::clamp_size(self.bottom_right, size);
        self.bottom_left = Self::clamp_size(self.bottom_left, size);
        self
    }
}

impl<Unit> From<Unit> for CornerRadii<Unit>
where
    Unit: Copy,
{
    fn from(radii: Unit) -> Self {
        Self {
            top_left: radii,
            top_right: radii,
            bottom_right: radii,
            bottom_left: radii,
        }
    }
}
