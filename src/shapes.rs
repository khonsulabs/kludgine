use std::ops::{Add, Neg};

use figures::units::UPx;
use figures::{FloatConversion, Point, Rect};
use lyon_tessellation::{
    FillGeometryBuilder, FillOptions, FillTessellator, FillVertex, FillVertexConstructor,
    GeometryBuilder, GeometryBuilderError, StrokeGeometryBuilder, StrokeTessellator, StrokeVertex,
    StrokeVertexConstructor, VertexId,
};
pub use lyon_tessellation::{LineCap, LineJoin};
use smallvec::SmallVec;

use crate::pipeline::Vertex;
use crate::{
    sealed, Color, Graphics, Origin, PreparedGraphic, ShapeSource, Texture, TextureSource,
};

/// A tesselated shape.
///
/// This structure contains geometry that has been divided into triangles, ready
/// to upload to the GPU. To render the shape, it must first be
/// [prepared](Self::prepare).
#[derive(Debug, Clone, PartialEq)]
pub struct Shape<Unit, const TEXTURED: bool> {
    pub(crate) vertices: SmallVec<[Vertex<Unit>; 6]>,
    pub(crate) indices: SmallVec<[u16; 20]>,
}

#[test]
fn shape_size() {
    assert_eq!(std::mem::size_of::<Shape<i32, true>>(), 176);
}

impl<Unit, const TEXTURED: bool> Default for Shape<Unit, TEXTURED> {
    fn default() -> Self {
        Self {
            vertices: SmallVec::new(),
            indices: SmallVec::new(),
        }
    }
}

impl<Unit> Shape<Unit, false> {
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
            .expect("should not fail to tesselat4e a rect");
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
        color: Color,
        options: StrokeOptions<Unit>,
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
        path.stroke(color, options)
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

impl<Unit, const TEXTURED: bool> sealed::ShapeSource<Unit> for Shape<Unit, TEXTURED>
where
    Unit: Copy,
{
    fn vertices(&self) -> &[Vertex<Unit>] {
        &self.vertices
    }

    fn indices(&self) -> &[u16] {
        &self.indices
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
            2 => Point::new(attributes[0].round(), attributes[1].round()).cast(),
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
    /// A small-vec of path events. Contains enough stack space to contain the
    /// path of a hexagon, because it's the bestagon.
    events: SmallVec<[PathEvent<Unit>; 7]>,
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

    /// Strokes this path with `color` and `options`.
    ///
    /// If this is a textured image, the sampled texture colors will be
    /// multiplied with this color. To render the image unchanged, use
    /// [`Color::WHITE`].
    #[must_use]
    pub fn stroke(&self, color: Color, options: StrokeOptions<Unit>) -> Shape<Unit, TEXTURED> {
        let lyon_path = self.as_lyon();
        let mut shape_builder = ShapeBuilder::new(color);
        let mut tesselator = StrokeTessellator::new();

        tesselator
            .tessellate_with_ids(
                lyon_path.id_iter(),
                &lyon_path,
                Some(&lyon_path),
                &options.into(),
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

/// Options for stroking lines on a path.
pub struct StrokeOptions<Unit> {
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
            line_width: Unit::default_stroke_width(),
            line_join: lyon_tessellation::StrokeOptions::DEFAULT_LINE_JOIN,
            start_cap: lyon_tessellation::StrokeOptions::DEFAULT_LINE_CAP,
            end_cap: lyon_tessellation::StrokeOptions::DEFAULT_LINE_CAP,
            miter_limit: lyon_tessellation::StrokeOptions::DEFAULT_MITER_LIMIT,
            tolerance: Default::default(),
        }
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
        Self(1)
    }
}
impl DefaultStrokeWidth for figures::units::UPx {
    fn default_stroke_width() -> Self {
        Self(1)
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
