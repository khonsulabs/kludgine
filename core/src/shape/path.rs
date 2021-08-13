use figures::{DisplayScale, Displayable};
use lyon_tessellation::path::{builder::PathBuilder as _, PathEvent as LyonPathEvent};

use super::lyon_point;
use crate::{
    math::{Pixels, Point, Scale, Scaled},
    scene::Target,
    shape::{Fill, Stroke},
    Error,
};

/// A point on a [`Path`].
pub type Endpoint<S> = Point<f32, S>;
/// A control point used to create curves.
pub type ControlPoint<S> = Point<f32, S>;

/// An entry in a [`Path`].
#[derive(Debug, Clone, Copy)]
pub enum PathEvent<S> {
    /// Begins a path. Must be at the start.
    Begin {
        /// The location to begin at.
        at: Endpoint<S>,
    },
    /// A straight line segment.
    Line {
        /// The origin of the line.
        from: Endpoint<S>,
        /// The end location of the line.
        to: Endpoint<S>,
    },
    /// A quadratic curve (one control point).
    Quadratic {
        /// The origin of the curve.
        from: Endpoint<S>,
        /// The control point for the curve.
        ctrl: ControlPoint<S>,
        /// The end location of the curve.
        to: Endpoint<S>,
    },
    /// A cubic curve (two control points).
    Cubic {
        /// The origin of the curve.
        from: Endpoint<S>,
        /// The first control point for the curve.
        ctrl1: ControlPoint<S>,
        /// The second control point for the curve.
        ctrl2: ControlPoint<S>,
        /// The end location of the curve.
        to: Endpoint<S>,
    },
    /// Ends the path. Must be the last entry.
    End {
        /// The end location of the path.
        last: Endpoint<S>,
        /// The start location of the path.
        first: Endpoint<S>,
        /// Whether the path should be closed.
        close: bool,
    },
}

impl From<PathEvent<Pixels>> for LyonPathEvent {
    fn from(event: PathEvent<Pixels>) -> Self {
        match event {
            PathEvent::Begin { at } => Self::Begin { at: lyon_point(at) },
            PathEvent::Line { from, to } => Self::Line {
                from: lyon_point(from),
                to: lyon_point(to),
            },
            PathEvent::Quadratic { from, ctrl, to } => Self::Quadratic {
                from: lyon_point(from),
                ctrl: lyon_point(ctrl),
                to: lyon_point(to),
            },
            PathEvent::Cubic {
                from,
                ctrl1,
                ctrl2,
                to,
            } => Self::Cubic {
                from: lyon_point(from),
                ctrl1: lyon_point(ctrl1),
                ctrl2: lyon_point(ctrl2),
                to: lyon_point(to),
            },
            PathEvent::End { last, first, close } => Self::End {
                last: lyon_point(last),
                first: lyon_point(first),
                close,
            },
        }
    }
}

impl<U> PathEvent<U> {
    /// Returns the path event with the new unit. Does not alter the underlying
    /// coordinate data.
    #[must_use]
    pub fn cast_unit<V>(self) -> PathEvent<V> {
        match self {
            Self::Begin { at } => PathEvent::Begin { at: at.cast_unit() },
            Self::Line { from, to } => PathEvent::Line {
                from: from.cast_unit(),
                to: to.cast_unit(),
            },
            Self::Quadratic { from, ctrl, to } => PathEvent::Quadratic {
                from: from.cast_unit(),
                ctrl: ctrl.cast_unit(),
                to: to.cast_unit(),
            },
            Self::Cubic {
                from,
                ctrl1,
                ctrl2,
                to,
            } => PathEvent::Cubic {
                from: from.cast_unit(),
                ctrl1: ctrl1.cast_unit(),
                ctrl2: ctrl2.cast_unit(),
                to: to.cast_unit(),
            },
            Self::End { last, first, close } => PathEvent::End {
                last: last.cast_unit(),
                first: first.cast_unit(),
                close,
            },
        }
    }
}

/// A geometric shape defined by a path.
#[derive(Default, Debug, Clone)]
pub struct Path<S> {
    events: Vec<PathEvent<S>>,
}

impl<U> Path<U> {
    /// Returns the path with the new unit. Does not alter the underlying
    /// coordinate data.
    #[must_use]
    pub fn cast_unit<V>(self) -> Path<V> {
        Path {
            events: self.events.into_iter().map(PathEvent::cast_unit).collect(),
        }
    }
}

impl Path<Scaled> {
    pub(crate) fn translate_and_convert_to_device(
        &self,
        location: Point<f32, Scaled>,
        scene: &Target,
    ) -> Path<Pixels> {
        let effective_scale = scene.scale();
        let location = scene.offset_point(location);
        let mut events = Vec::new();

        for event in &self.events {
            // There's a bug with async-local variables and this analysis. There is no
            // cross-dependency on any of these parameters.
            #[allow(clippy::eval_order_dependence)]
            events.push(match event {
                PathEvent::Begin { at } => PathEvent::Begin {
                    at: Self::convert_point(*at, location, effective_scale),
                },
                PathEvent::Line { from, to } => PathEvent::Line {
                    from: Self::convert_point(*from, location, effective_scale),
                    to: Self::convert_point(*to, location, effective_scale),
                },
                PathEvent::End { first, last, close } => PathEvent::End {
                    first: Self::convert_point(*first, location, effective_scale),
                    last: Self::convert_point(*last, location, effective_scale),
                    close: *close,
                },
                PathEvent::Quadratic { from, ctrl, to } => PathEvent::Quadratic {
                    from: Self::convert_point(*from, location, effective_scale),
                    ctrl: Self::convert_point(*ctrl, location, effective_scale),
                    to: Self::convert_point(*to, location, effective_scale),
                },
                PathEvent::Cubic {
                    from,
                    ctrl1,
                    ctrl2,
                    to,
                } => PathEvent::Cubic {
                    from: Self::convert_point(*from, location, effective_scale),
                    ctrl1: Self::convert_point(*ctrl1, location, effective_scale),
                    ctrl2: Self::convert_point(*ctrl2, location, effective_scale),
                    to: Self::convert_point(*to, location, effective_scale),
                },
            });
        }

        Path { events }
    }

    fn convert_point(
        point: Point<f32, Scaled>,
        location: Point<f32, Scaled>,
        effective_scale: &DisplayScale<f32>,
    ) -> Point<f32, Pixels> {
        (location + point).to_pixels(effective_scale)
    }
}

impl Path<Pixels> {
    pub(crate) fn build(
        &self,
        builder: &mut easygpu_lyon::ShapeBuilder,
        stroke: &Option<Stroke>,
        fill: &Option<Fill>,
    ) -> crate::Result<()> {
        let path = self.as_lyon();
        if let Some(fill) = fill {
            builder.default_color = fill.color.rgba();
            builder
                .fill(&path, &fill.options)
                .map_err(Error::Tessellation)?;
        }

        if let Some(stroke) = stroke {
            builder.default_color = stroke.color.rgba();
            builder
                .stroke(&path, &stroke.options)
                .map_err(Error::Tessellation)?;
        }

        Ok(())
    }

    pub(crate) fn as_lyon(&self) -> lyon_tessellation::path::Path {
        let mut builder = lyon_tessellation::path::Path::builder();
        for &event in &self.events {
            builder.path_event(event.into());
        }
        builder.build()
    }
}

impl<S, T> From<T> for Path<S>
where
    T: IntoIterator<Item = PathEvent<S>>,
{
    fn from(source: T) -> Self {
        Self {
            events: source.into_iter().collect(),
        }
    }
}

/// Builds a [`Path`].
pub struct PathBuilder<S> {
    path: Path<S>,
    start_at: Endpoint<S>,
    current_location: Endpoint<S>,
    close: bool,
}

impl<S> PathBuilder<S> {
    /// Creates a new path with the initial position `start_at`.
    #[must_use]
    pub fn new(start_at: Endpoint<S>) -> Self {
        let events = vec![PathEvent::Begin { at: start_at }];
        Self {
            path: Path::from(events),
            start_at,
            current_location: start_at,
            close: false,
        }
    }

    /// Returns the built path.
    #[must_use]
    pub fn build(mut self) -> Path<S> {
        self.path.events.push(PathEvent::End {
            first: self.start_at,
            last: self.current_location,
            close: self.close,
        });
        self.path
    }

    /// Create a straight line from the current location to `end_at`.
    #[must_use]
    pub fn line_to(mut self, end_at: Endpoint<S>) -> Self {
        self.path.events.push(PathEvent::Line {
            from: self.current_location,
            to: end_at,
        });
        self.current_location = end_at;
        self
    }

    /// Create a quadratic curve from the current location to `end_at` using
    /// `control` as the curve's control point.
    #[must_use]
    pub fn quadratic_curve_to(mut self, control: ControlPoint<S>, end_at: Endpoint<S>) -> Self {
        self.path.events.push(PathEvent::Quadratic {
            from: self.current_location,
            ctrl: control,
            to: end_at,
        });
        self.current_location = end_at;
        self
    }

    /// Create a cubic curve from the current location to `end_at` using
    /// `control1` and `control2` as the curve's control points.
    #[must_use]
    pub fn cubic_curve_to(
        mut self,
        control1: ControlPoint<S>,
        control2: ControlPoint<S>,
        end_at: Endpoint<S>,
    ) -> Self {
        self.path.events.push(PathEvent::Cubic {
            from: self.current_location,
            ctrl1: control1,
            ctrl2: control2,
            to: end_at,
        });
        self.current_location = end_at;
        self
    }

    /// Closes the path, connecting the current location to the shape's starting
    /// location.
    #[must_use]
    pub const fn close(mut self) -> Self {
        self.close = true;
        self
    }
}

impl<Src, Dst> std::ops::Mul<Scale<f32, Src, Dst>> for Path<Src> {
    type Output = Path<Dst>;

    fn mul(self, scale: Scale<f32, Src, Dst>) -> Self::Output {
        Self::Output {
            events: self.events.into_iter().map(|event| event * scale).collect(),
        }
    }
}

impl<Src, Dst> std::ops::Mul<Scale<f32, Src, Dst>> for PathEvent<Src> {
    type Output = PathEvent<Dst>;

    fn mul(self, scale: Scale<f32, Src, Dst>) -> Self::Output {
        match self {
            PathEvent::Begin { at } => Self::Output::Begin { at: at * scale },
            PathEvent::Line { from, to } => Self::Output::Line {
                from: from * scale,
                to: to * scale,
            },
            PathEvent::Quadratic { from, ctrl, to } => Self::Output::Quadratic {
                from: from * scale,
                ctrl: ctrl * scale,
                to: to * scale,
            },
            PathEvent::Cubic {
                from,
                ctrl1,
                ctrl2,
                to,
            } => Self::Output::Cubic {
                from: from * scale,
                ctrl1: ctrl1 * scale,
                ctrl2: ctrl2 * scale,
                to: to * scale,
            },
            PathEvent::End { last, first, close } => Self::Output::End {
                last: last * scale,
                first: first * scale,
                close,
            },
        }
    }
}
