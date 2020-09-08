use crate::{
    math::{Pixels, Point, Points},
    scene::SceneTarget,
    shape::{Fill, Stroke},
    KludgineError, KludgineResult,
};
use lyon_tessellation::path::{builder::PathBuilder as _, PathEvent as LyonPathEvent};

pub type Endpoint<S> = Point<S>;
pub type ControlPoint<S> = Point<S>;

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

impl Into<lyon_tessellation::math::Point> for Point<Pixels> {
    fn into(self) -> lyon_tessellation::math::Point {
        lyon_tessellation::math::point(self.x.to_f32(), self.y.to_f32())
    }
}

impl Into<LyonPathEvent> for PathEvent<Pixels> {
    fn into(self) -> LyonPathEvent {
        match self {
            Self::Begin { at } => LyonPathEvent::Begin { at: at.into() },
            Self::Line { from, to } => LyonPathEvent::Line {
                from: from.into(),
                to: to.into(),
            },
            Self::Quadratic { from, ctrl, to } => LyonPathEvent::Quadratic {
                from: from.into(),
                ctrl: ctrl.into(),
                to: to.into(),
            },
            Self::Cubic {
                from,
                ctrl1,
                ctrl2,
                to,
            } => LyonPathEvent::Cubic {
                from: from.into(),
                ctrl1: ctrl1.into(),
                ctrl2: ctrl2.into(),
                to: to.into(),
            },
            Self::End { last, first, close } => LyonPathEvent::End {
                last: last.into(),
                first: first.into(),
                close,
            },
        }
    }
}

#[derive(Default, Debug, Clone)]
pub struct Path<S> {
    events: Vec<PathEvent<S>>,
}

impl Path<Points> {
    pub(crate) async fn translate_and_convert_to_device(
        &self,
        location: Point<Points>,
        scene: &SceneTarget,
    ) -> Path<Pixels> {
        let effective_scale = scene.effective_scale_factor().await;
        let mut events = Vec::new();

        for event in &self.events {
            // There's a bug with async-local variables and this analysis. There is no cross-dependency on any of these parameters.
            #[allow(clippy::eval_order_dependence)]
            events.push(match event {
                PathEvent::Begin { at } => PathEvent::Begin {
                    at: Self::convert_point(*at, location, scene, effective_scale).await,
                },
                PathEvent::Line { from, to } => PathEvent::Line {
                    from: Self::convert_point(*from, location, scene, effective_scale).await,
                    to: Self::convert_point(*to, location, scene, effective_scale).await,
                },
                PathEvent::End { first, last, close } => PathEvent::End {
                    first: Self::convert_point(*first, location, scene, effective_scale).await,
                    last: Self::convert_point(*last, location, scene, effective_scale).await,
                    close: *close,
                },
                PathEvent::Quadratic { from, ctrl, to } => PathEvent::Quadratic {
                    from: Self::convert_point(*from, location, scene, effective_scale).await,
                    ctrl: Self::convert_point(*ctrl, location, scene, effective_scale).await,
                    to: Self::convert_point(*to, location, scene, effective_scale).await,
                },
                PathEvent::Cubic {
                    from,
                    ctrl1,
                    ctrl2,
                    to,
                } => PathEvent::Cubic {
                    from: Self::convert_point(*from, location, scene, effective_scale).await,
                    ctrl1: Self::convert_point(*ctrl1, location, scene, effective_scale).await,
                    ctrl2: Self::convert_point(*ctrl2, location, scene, effective_scale).await,
                    to: Self::convert_point(*to, location, scene, effective_scale).await,
                },
            })
        }

        Path { events }
    }

    async fn convert_point(
        point: Point<Points>,
        location: Point<Points>,
        scene: &SceneTarget,
        effective_scale: f32,
    ) -> Point<Pixels> {
        scene
            .user_to_device_point(location + point)
            .await
            .to_pixels(effective_scale)
    }
}

impl Path<Pixels> {
    pub fn build(
        &self,
        builder: &mut rgx_lyon::ShapeBuilder,
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
