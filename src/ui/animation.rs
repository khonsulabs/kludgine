use async_trait::async_trait;
use std::{collections::VecDeque, sync::Arc, time::Instant};

pub trait Transition: Send + Sync + std::fmt::Debug {
    fn current_percent(&self, elapsed_percent: f32) -> f32;
}

#[derive(Clone, Debug)]
pub struct LinearTransition;

impl Transition for LinearTransition {
    fn current_percent(&self, elapsed_percent: f32) -> f32 {
        elapsed_percent
    }
}

#[async_trait]
pub trait PropertyMutator<T>: Clone + Send + Sync {
    async fn update_property(&self, value: T);
}

#[async_trait]
pub trait PropertyChange<T>: Clone + Send + Sync {
    async fn update(&self, existing: &T, target: &T, elapsed_percent: f32);
}
// pub struct PointChange<T> {
//     target: Point<Points>,
//     existing: Point<Points>,
//     transition: T,
//     mutator: Box<dyn PropertyMutator<T>>,
//     // entity: Entity<E>,
// }

// impl<T> PropertyChange for PointChange<T>
// where
//     T: Transition,
// {
//     fn update(&self, elapsed_percent: f32) {

//     }
// }
#[derive(Clone, Debug)]
pub struct FloatChange<M> {
    pub transition: Arc<Box<dyn Transition>>,
    pub mutator: M,
}

#[async_trait]
impl<M> PropertyChange<f32> for FloatChange<M>
where
    M: PropertyMutator<f32>,
{
    async fn update(&self, existing: &f32, target: &f32, elapsed_percent: f32) {
        let transition_percent = self.transition.current_percent(elapsed_percent);
        let value = (*target - *existing) * transition_percent + *existing;
        self.mutator.update_property(value).await
    }
}

#[derive(Clone, Debug)]
pub struct ChainedFrameTransitioner<A, B> {
    change_a: A,
    change_b: B,
}

#[derive(Debug, Clone)]
pub struct ChainedFrameValue<AV, BV> {
    a: AV,
    b: BV,
}

#[async_trait]
impl<A, B, AV, BV> FrameTransitioner for ChainedFrameTransitioner<A, B>
where
    A: FrameTransitioner<Value = AV>,
    B: FrameTransitioner<Value = BV>,
    AV: Clone + Send + Sync + std::fmt::Debug,
    BV: Clone + Send + Sync + std::fmt::Debug,
{
    type Value = ChainedFrameValue<A::Value, B::Value>;

    fn initialize_from(&mut self, last: &Self::Value) {
        self.change_a.initialize_from(&last.a);
        self.change_b.initialize_from(&last.b);
    }

    async fn transition_between(&mut self, next: &Self::Value, elapsed_percent: f32) {
        self.change_a
            .transition_between(&next.a, elapsed_percent)
            .await;
        self.change_b
            .transition_between(&next.b, elapsed_percent)
            .await;
    }

    fn target(&self) -> Self::Value {
        ChainedFrameValue {
            a: self.change_a.target(),
            b: self.change_b.target(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct PropertyFrameManager<T, P> {
    pub last_value: Option<T>,
    pub target: T,
    pub property_change: P,
}

#[async_trait]
pub trait FrameTransitioner: Clone + Send + Sync {
    type Value: Clone + Send + Sync + std::fmt::Debug;

    fn initialize_from(&mut self, last: &Self::Value);
    fn target(&self) -> Self::Value;

    async fn transition_between(&mut self, next: &Self::Value, elapsed_percent: f32);
}

#[async_trait]
impl<V, P> FrameTransitioner for PropertyFrameManager<V, P>
where
    V: Clone + Send + Sync + std::fmt::Debug,
    P: PropertyChange<V>,
{
    type Value = V;

    fn initialize_from(&mut self, last: &Self::Value) {
        self.last_value = Some(last.clone());
    }

    async fn transition_between(&mut self, next: &Self::Value, elapsed_percent: f32) {
        let last_value = self.last_value.as_ref().unwrap_or(next);
        self.property_change
            .update(last_value, next, elapsed_percent)
            .await
    }

    fn target(&self) -> Self::Value {
        self.target.clone()
    }
}

#[derive(Debug, Clone)]
pub struct AnimationFrame<T>
where
    T: FrameTransitioner,
{
    frame: T,
    instant: Instant,
    next: T::Value,
}

impl<T> AnimationFrame<T>
where
    T: FrameTransitioner,
{
    pub async fn transition(&mut self, elapsed_percent: f32) {
        self.frame
            .transition_between(&self.next, elapsed_percent)
            .await
    }
}

impl<T> AnimationFrame<T>
where
    T: FrameTransitioner,
{
    pub fn new(frame: T, instant: Instant) -> Self {
        Self {
            next: frame.target(),
            frame,
            instant,
        }
    }

    pub fn initialize(&mut self, last: &AnimationFrame<T>) {
        self.frame.initialize_from(&last.next);
    }
}

pub struct AnimationManager<T>
where
    T: FrameTransitioner,
{
    last_frame: AnimationFrame<T>,
    current_frame: Option<AnimationFrame<T>>,
    pending_frames: VecDeque<AnimationFrame<T>>,
}

impl<T> AnimationManager<T>
where
    T: FrameTransitioner,
{
    pub async fn new(initial_frame: T) -> Self {
        let mut last_frame = AnimationFrame::new(initial_frame, Instant::now());
        last_frame.transition(1.).await;
        Self {
            last_frame,
            current_frame: None,
            pending_frames: VecDeque::default(),
        }
    }

    pub fn push_frame(&mut self, frame: T, target: Instant) {
        // Cull any frames that take place after this new frame
        while let Some(peek) = self.pending_frames.back() {
            if peek.instant > target {
                self.pending_frames.pop_back();
            } else {
                break;
            }
        }

        // If this is the first new frame, the animation should start from now, not
        // the last frame's ending
        self.last_frame.instant = Instant::now();
        self.pending_frames
            .push_back(AnimationFrame::new(frame, target));
    }

    fn update_current_frame(&mut self, now: Instant) {
        while !self.pending_frames.is_empty() {
            if let Some(current_frame) = &self.current_frame {
                if current_frame.instant > now {
                    return;
                } else {
                    self.last_frame = self.current_frame.take().unwrap();
                }
            }

            if let Some(mut pending_frame) = self.pending_frames.pop_front() {
                pending_frame.initialize(&self.last_frame);
                self.current_frame = Some(pending_frame);
            }
        }
    }

    pub async fn update(&mut self) {
        let now = Instant::now();
        self.update_current_frame(now);
        if let Some(current_frame) = &mut self.current_frame {
            if let Some(elapsed_since_last_frame) =
                now.checked_duration_since(self.last_frame.instant)
            {
                if let Some(animation_duration) = current_frame
                    .instant
                    .checked_duration_since(self.last_frame.instant)
                {
                    let elapsed_percent =
                        elapsed_since_last_frame.as_secs_f32() / animation_duration.as_secs_f32();
                    current_frame.transition(elapsed_percent).await
                }
            }
        }
    }
}
