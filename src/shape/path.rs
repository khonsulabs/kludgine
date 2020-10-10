use crate::{
    math::{Point, Raw, Scale, Scaled, ScreenScale},
    scene::Scene,
    shape::{Fill, Stroke},
    KludgineError, KludgineResult,
};
use lyon_tessellation::path::{builder::PathBuilder as _, PathEvent as LyonPathEvent};

pub type Endpoint<S> = Point<f32, S>;
pub type ControlPoint<S> = Point<f32, S>;

#[derive(Debug, Clone, Copy)]
pub enum PathEvent<S> {
    Begin {
        at: Endpoint<S>,
    },
    Line {
        from: Endpoint<S>,
        to: Endpoint<S>,
    },
    Quadratic {
        from: Endpoint<S>,
        ctrl: ControlPoint<S>,
        to: Endpoint<S>,
    },
    Cubic {
        from: Endpoint<S>,
        ctrl1: ControlPoint<S>,
        ctrl2: ControlPoint<S>,
        to: Endpoint<S>,
    },
    End {
        last: Endpoint<S>,
        first: Endpoint<S>,
        close: bool,
    },
}

impl Into<LyonPathEvent> for PathEvent<Raw> {
    fn into(self) -> LyonPathEvent {
        match self {
            Self::Begin { at } => LyonPathEvent::Begin { at: at.cast_unit() },
            Self::Line { from, to } => LyonPathEvent::Line {
                from: from.cast_unit(),
                to: to.cast_unit(),
            },
            Self::Quadratic { from, ctrl, to } => LyonPathEvent::Quadratic {
                from: from.cast_unit(),
                ctrl: ctrl.cast_unit(),
                to: to.cast_unit(),
            },
            Self::Cubic {
                from,
                ctrl1,
                ctrl2,
                to,
            } => LyonPathEvent::Cubic {
                from: from.cast_unit(),
                ctrl1: ctrl1.cast_unit(),
                ctrl2: ctrl2.cast_unit(),
                to: to.cast_unit(),
            },
            Self::End { last, first, close } => LyonPathEvent::End {
                last: last.cast_unit(),
                first: first.cast_unit(),
                close,
            },
        }
    }
}

#[derive(Default, Debug, Clone)]
pub struct Path<S> {
    events: Vec<PathEvent<S>>,
}

impl Path<Scaled> {
    pub(crate) async fn translate_and_convert_to_device(
        &self,
        location: Point<f32, Scaled>,
        scene: &Scene,
    ) -> Path<Raw> {
        let effective_scale = scene.scale_factor().await;
        let mut events = Vec::new();

        for event in &self.events {
            // There's a bug with async-local variables and this analysis. There is no cross-dependency on any of these parameters.
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
            })
        }

        Path { events }
    }

    fn convert_point(
        point: Point<f32, Scaled>,
        location: Point<f32, Scaled>,
        effective_scale: ScreenScale,
    ) -> Point<f32, Raw> {
        (location + point.to_vector()) * effective_scale
    }
}

impl Path<Raw> {
    pub fn build(
        &self,
        builder: &mut easygpu_lyon::ShapeBuilder,
        stroke: &Option<Stroke>,
        fill: &Option<Fill>,
    ) -> KludgineResult<()> {
        let path = self.as_lyon();
        if let Some(fill) = fill {
            builder.default_color = fill.color.rgba();
            builder
                .fill(&path, &fill.options)
                .map_err(KludgineError::TessellationError)?;
        }

        if let Some(stroke) = stroke {
            builder.default_color = stroke.color.rgba();
            builder
                .stroke(&path, &stroke.options)
                .map_err(KludgineError::TessellationError)?;
        }

        Ok(())
    }

    pub(crate) fn as_lyon(&self) -> lyon_tessellation::path::Path {
        let mut builder = lyon_tessellation::path::Path::builder();
        for &event in &self.events {
            builder.path_event(event.into())
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

pub struct PathBuilder<S> {
    path: Path<S>,
    start_at: Endpoint<S>,
    current_location: Endpoint<S>,
    close: bool,
}

impl<S> PathBuilder<S>
where
    S: Copy,
{
    pub fn new(start_at: Endpoint<S>) -> Self {
        let events = vec![PathEvent::Begin { at: start_at }];
        Self {
            path: Path::from(events),
            start_at,
            current_location: start_at,
            close: false,
        }
    }

    pub fn build(mut self) -> Path<S> {
        self.path.events.push(PathEvent::End {
            first: self.start_at,
            last: self.current_location,
            close: self.close,
        });
        self.path
    }

    pub fn line_to(mut self, end_at: Endpoint<S>) -> Self {
        self.path.events.push(PathEvent::Line {
            from: self.current_location,
            to: end_at,
        });
        self.current_location = end_at;
        self
    }

    pub fn quadratic_curve_to(mut self, control: ControlPoint<S>, end_at: Endpoint<S>) -> Self {
        self.path.events.push(PathEvent::Quadratic {
            from: self.current_location,
            ctrl: control,
            to: end_at,
        });
        self.current_location = end_at;
        self
    }

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

    pub fn close(mut self) -> Self {
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
