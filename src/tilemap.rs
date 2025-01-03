#![allow(missing_docs, clippy::missing_panics_doc)] // This file is a work in progress.

use std::fmt::Debug;
use std::ops::{Deref, DerefMut, Index, IndexMut};
use std::time::Duration;

use alot::{LotId, OrderedLots};
use figures::{Fraction, Ranged, Zero};
use intentional::Cast;

use crate::drawing::Renderer;
use crate::figures::units::Px;
use crate::figures::{IntoSigned, Point, Rect, Size};
use crate::shapes::{PathBuilder, Shape, StrokeOptions};
use crate::sprite::Sprite;
use crate::text::Text;
use crate::{AnyTexture, Assert, Color, DrawableExt};

pub const TILE_SIZE: Px = Px::new(32);

// At the time of writing, this is is used to translate from
// tilemap coords to world coords
// TB: 2023-11-14
#[must_use]
pub fn translate_coordinates(
    coordinate: Point<Px>,
    scale: Fraction,
    zoom: f32,
    size: Size<Px>,
) -> Point<Px> {
    let center = Point::new(size.width / 2, size.height / 2);
    let coordinate = coordinate - center;
    let effective_zoom = scale.into_f32() * zoom;
    Point::new(coordinate.x / effective_zoom, coordinate.y / effective_zoom)
}

pub fn draw(
    layers: &mut impl Layers,
    focus: TileMapFocus,
    zoom: f32,
    elapsed: Duration,
    graphics: &mut Renderer<'_, '_>,
) -> Option<Duration> {
    let effective_zoom = graphics.scale().into_f32() * zoom;
    let mut remaining_until_next_frame = None;

    let world_coordinate = focus.world_coordinate(layers);
    let offset = world_coordinate * effective_zoom;

    let visible_size = graphics.clip_rect().size.into_signed();
    let visible_region = Rect::new(offset - visible_size / 2, visible_size);
    let tile_size = TILE_SIZE * effective_zoom;
    let top_left = first_tile(visible_region.origin, tile_size);
    let bottom_right = last_tile(visible_region.origin + visible_region.size, tile_size);

    let mut context = LayerContext {
        top_left,
        bottom_right,
        tile_size,
        origin: Point::from(visible_size) / 2 - world_coordinate,
        visible_rect: visible_region,
        zoom,
        elapsed,
        renderer: graphics,
    };
    for index in 0.. {
        let Some(layer) = layers.layer_mut(index) else {
            break;
        };
        remaining_until_next_frame =
            minimum_duration(remaining_until_next_frame, layer.render(&mut context));
    }

    remaining_until_next_frame
}

pub struct TileOffset {
    index: Point<isize>,
    tile_offset: Point<Px>,
}

fn first_tile(pos: Point<Px>, tile_size: Px) -> TileOffset {
    fn coord_info(pos: Px, tile_size: Px) -> (isize, Px) {
        let remainder = pos % tile_size;
        let (offset, floored) = if pos < 0 {
            // Remainder is negative here.
            let offset = tile_size + remainder;
            let floored = pos - tile_size - offset;

            (-offset, floored)
        } else {
            (-remainder, pos - remainder)
        };
        let index =
            isize::try_from((floored / tile_size).get()).expect("tile size out of range of isize");
        (index, offset)
    }

    let (x_index, x) = coord_info(pos.x, tile_size);
    let (y_index, y) = coord_info(pos.y, tile_size);
    TileOffset {
        index: Point::new(x_index, y_index),
        tile_offset: Point::new(x, y),
    }
}

fn last_tile(pos: Point<Px>, tile_size: Px) -> TileOffset {
    fn coord_info(pos: Px, tile_size: Px) -> (isize, Px) {
        let index = (pos.get() + pos.get().signum() * (tile_size.get() - 1)) / tile_size.get();
        let floored = tile_size * index;
        let index = isize::try_from(index).expect("tile size out of range of isize");
        (index, floored)
    }

    let (x_index, x) = coord_info(pos.x, tile_size);
    let (y_index, y) = coord_info(pos.y, tile_size);
    TileOffset {
        index: Point::new(x_index, y_index),
        tile_offset: Point::new(x, y),
    }
}

pub trait Layers: Debug + Send + 'static {
    fn layer(&self, index: usize) -> Option<&dyn Layer>;
    fn layer_mut(&mut self, index: usize) -> Option<&mut dyn Layer>;
}

impl<T> Layers for T
where
    T: Layer,
{
    fn layer(&self, index: usize) -> Option<&dyn Layer> {
        (index == 0).then_some(self)
    }

    fn layer_mut(&mut self, index: usize) -> Option<&mut dyn Layer> {
        (index == 0).then_some(self)
    }
}

macro_rules! impl_layers_for_tuples {
    ($($type:ident : $index:tt),+) => {
        impl<$($type),+> Layers for ($($type,)+) where $(
            $type: Debug +  Send + Layer + 'static
        ),+ {
            fn layer(&self, index: usize) -> Option<&dyn Layer> {
                match index {
                    $($index => Some(&self.$index),)+
                    _ => None,
                }
            }

            fn layer_mut(&mut self, index: usize) -> Option<&mut dyn Layer> {
                match index {
                    $($index => Some(&mut self.$index),)+
                    _ => None,
                }
            }
        }
    };
}

impl_layers_for_tuples!(T0: 0);
impl_layers_for_tuples!(T0: 0, T1: 1);
impl_layers_for_tuples!(T0: 0, T1: 1, T2: 2);
impl_layers_for_tuples!(T0: 0, T1: 1, T2: 2, T3: 3);
impl_layers_for_tuples!(T0: 0, T1: 1, T2: 2, T3: 3, T4: 4);
impl_layers_for_tuples!(T0: 0, T1: 1, T2: 2, T3: 3, T4: 4, T5: 5);
impl_layers_for_tuples!(T0: 0, T1: 1, T2: 2, T3: 3, T4: 4, T5: 5, T6: 6);

pub struct LayerContext<'render, 'ctx, 'pass> {
    top_left: TileOffset,
    bottom_right: TileOffset,
    tile_size: Px,
    origin: Point<Px>,
    visible_rect: Rect<Px>,
    zoom: f32,
    elapsed: Duration,
    renderer: &'render mut Renderer<'ctx, 'pass>,
}

impl LayerContext<'_, '_, '_> {
    #[must_use]
    pub const fn top_left(&self) -> &TileOffset {
        &self.top_left
    }

    #[must_use]
    pub const fn bottom_right(&self) -> &TileOffset {
        &self.bottom_right
    }

    #[must_use]
    pub const fn tile_size(&self) -> Px {
        self.tile_size
    }

    #[must_use]
    pub const fn origin(&self) -> Point<Px> {
        self.origin
    }

    #[must_use]
    pub const fn visible_rect(&self) -> Rect<Px> {
        self.visible_rect
    }

    #[must_use]
    pub const fn elapsed(&self) -> Duration {
        self.elapsed
    }

    #[must_use]
    pub const fn zoom(&self) -> f32 {
        self.zoom
    }
}

impl<'ctx, 'pass> Deref for LayerContext<'_, 'ctx, 'pass> {
    type Target = Renderer<'ctx, 'pass>;

    fn deref(&self) -> &Self::Target {
        self.renderer
    }
}

impl DerefMut for LayerContext<'_, '_, '_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.renderer
    }
}

pub trait Layer: Debug + Send + 'static {
    fn render(&mut self, context: &mut LayerContext<'_, '_, '_>) -> Option<Duration>;

    fn find_object(&self, _object: ObjectId) -> Option<Point<Px>> {
        None
    }
}

fn isize_to_i32(value: isize) -> i32 {
    i32::try_from(value).unwrap_or_else(|_| {
        if value.is_negative() {
            i32::MIN
        } else {
            i32::MAX
        }
    })
}

impl<Source> Layer for Source
where
    Source: TileSource,
{
    fn render(&mut self, context: &mut LayerContext<'_, '_, '_>) -> Option<Duration> {
        let minimum_tile = self.minimum_tile();
        if minimum_tile.x > context.bottom_right().index.x
            || minimum_tile.y > context.bottom_right().index.y
        {
            return None;
        }

        let mut remaining_until_next_frame = None;
        let (x, left) = if context.top_left().index.x >= minimum_tile.x {
            (context.top_left().tile_offset.x, context.top_left().index.x)
        } else {
            let tile_offset =
                context.tile_size() * isize_to_i32(minimum_tile.x - context.top_left().index.x);
            (context.top_left().tile_offset.x + tile_offset, 0)
        };
        let (mut y, top) = if context.top_left().index.y >= minimum_tile.y {
            (context.top_left().tile_offset.y, context.top_left().index.y)
        } else {
            let tile_offset =
                context.tile_size() * isize_to_i32(minimum_tile.y - context.top_left().index.y);
            (context.top_left().tile_offset.y + tile_offset, 0)
        };

        let maximum_tile = self.maximum_tile();
        let right = context.bottom_right().index.x.min(maximum_tile.x) - 1;
        let bottom = context.bottom_right().index.y.min(maximum_tile.y) - 1;

        if left <= right && top <= bottom {
            for y_index in top..=bottom {
                let mut x = x;
                for x_index in left..=right {
                    let tile_rect = Rect::new(Point::new(x, y), Size::squared(context.tile_size()));
                    remaining_until_next_frame = minimum_duration(
                        remaining_until_next_frame,
                        self.render(Point::new(x_index, y_index), tile_rect, context),
                    );
                    x += context.tile_size();
                }
                y += context.tile_size();
            }
        }

        remaining_until_next_frame
    }
}

#[derive(Debug)]
pub struct TileArray<Tiles> {
    pub width: usize,
    pub tiles: Tiles,
}

impl<Tiles> TileArray<Tiles>
where
    Tiles: TileList,
{
    pub fn new(width: usize, tiles: Tiles) -> Self {
        assert!(tiles.len() % width == 0);
        Self { width, tiles }
    }
}

#[allow(clippy::len_without_is_empty)]
pub trait TileList: IndexMut<usize, Output = TileKind> + Send + Debug + 'static {
    fn len(&self) -> usize;
}

impl TileList for Vec<TileKind> {
    fn len(&self) -> usize {
        self.len()
    }
}

impl<const N: usize> TileList for [TileKind; N] {
    fn len(&self) -> usize {
        N
    }
}

pub trait TileSource: Send + Debug + 'static {
    fn minimum_tile(&self) -> Point<isize> {
        Point::MIN
    }
    fn maximum_tile(&self) -> Point<isize> {
        Point::MAX
    }

    fn render(
        &mut self,
        coordinate: Point<isize>,
        rect: Rect<Px>,
        context: &mut LayerContext<'_, '_, '_>,
    ) -> Option<Duration>;
}

impl<Tiles> TileSource for TileArray<Tiles>
where
    Tiles: TileList,
{
    fn minimum_tile(&self) -> Point<isize> {
        Point::ZERO
    }

    fn maximum_tile(&self) -> Point<isize> {
        Point::new(self.width, self.tiles.len() / self.width).map(Cast::cast)
    }

    fn render(
        &mut self,
        coordinate: Point<isize>,
        rect: Rect<Px>,
        context: &mut LayerContext<'_, '_, '_>,
    ) -> Option<Duration> {
        self.tiles[coordinate.y.cast::<usize>() * self.width + coordinate.x.cast::<usize>()]
            .render(rect, context)
    }
}

fn minimum_duration(
    min_duration: Option<Duration>,
    duration: Option<Duration>,
) -> Option<Duration> {
    match (min_duration, duration) {
        (Some(min_remaining), Some(remaining)) if remaining < min_remaining => Some(remaining),
        (None, remaining) => remaining,
        (min_remaining, _) => min_remaining,
    }
}

#[derive(Debug)]
pub enum TileKind {
    Texture(AnyTexture),
    Sprite(Sprite),
    Color(Color),
}

impl TileKind {
    pub fn render(
        &mut self,
        tile_rect: Rect<Px>,
        context: &mut LayerContext<'_, '_, '_>,
    ) -> Option<Duration> {
        match self {
            TileKind::Texture(texture) => {
                // TODO support other scaling options like
                // aspect-fit rather than fill.
                context.draw_texture(texture, tile_rect, 1.);
                None
            }
            TileKind::Color(color) => {
                context.draw_shape(&Shape::filled_rect(tile_rect, *color));
                None
            }
            TileKind::Sprite(sprite) => {
                if let Ok(frame) = sprite.get_frame(Some(context.elapsed())) {
                    context.draw_texture(&frame, tile_rect, 1.);
                    sprite.remaining_frame_duration().ok().flatten()
                } else {
                    // TODO show a broken image?
                    None
                }
            }
        }
    }
}

#[derive(Debug)]
pub struct ObjectLayer<O> {
    objects: OrderedLots<O>,
}

impl<O> Default for ObjectLayer<O> {
    fn default() -> Self {
        Self::new()
    }
}

impl<O> ObjectLayer<O> {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            objects: OrderedLots::new(),
        }
    }

    pub fn push(&mut self, object: O) -> ObjectId {
        ObjectId(self.objects.push(object))
    }

    #[must_use]
    pub fn get(&self, id: ObjectId) -> Option<&O> {
        self.objects.get(id.0)
    }

    pub fn get_mut(&mut self, id: ObjectId) -> Option<&mut O> {
        self.objects.get_mut(id.0)
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.objects.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.objects.is_empty()
    }

    pub fn get_nth(&mut self, index: usize) -> Option<&O> {
        self.objects.get_by_index(index)
    }

    pub fn get_nth_mut(&mut self, index: usize) -> Option<&mut O> {
        self.objects.get_mut_by_index(index)
    }
}

impl<O> Index<ObjectId> for ObjectLayer<O> {
    type Output = O;

    fn index(&self, id: ObjectId) -> &Self::Output {
        &self.objects[id.0]
    }
}

impl<O> IndexMut<ObjectId> for ObjectLayer<O> {
    fn index_mut(&mut self, id: ObjectId) -> &mut Self::Output {
        &mut self.objects[id.0]
    }
}

impl<O> Index<usize> for ObjectLayer<O> {
    type Output = O;

    fn index(&self, index: usize) -> &Self::Output {
        &self.objects[index]
    }
}

impl<O> IndexMut<usize> for ObjectLayer<O> {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.objects[index]
    }
}

impl<O> Layer for ObjectLayer<O>
where
    O: Object,
{
    fn render(&mut self, context: &mut LayerContext<'_, '_, '_>) -> Option<Duration> {
        let mut min_duration = None;
        for obj in &self.objects {
            let center = context.origin + obj.position();

            min_duration =
                minimum_duration(min_duration, obj.render(center, context.zoom(), context));
        }
        min_duration
    }

    fn find_object(&self, object: ObjectId) -> Option<Point<Px>> {
        Some(self[object].position())
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct ObjectId(LotId);

#[derive(Debug)]
pub struct ObjectInfo<O> {
    pub position: Point<Px>,
    pub object: O,
}

#[derive(Debug, Clone, Copy)]
pub enum TileMapFocus {
    Point(Point<Px>),
    Object { layer: usize, id: ObjectId },
}

impl TileMapFocus {
    // Get the world coordinate of the selected focus.
    // Zoom in / out etc. will not change the world coordinate.
    // TB: 2023-11-14
    pub fn world_coordinate(self, layers: &impl Layers) -> Point<Px> {
        match self {
            TileMapFocus::Point(focus) => focus,
            TileMapFocus::Object { layer, id } => layers
                .layer(layer)
                .assert("invalid focus layer")
                .find_object(id)
                .assert("focus not found"),
        }
    }
}

impl Default for TileMapFocus {
    fn default() -> Self {
        Self::Point(Point::default())
    }
}

pub trait Object: Debug + Send + 'static {
    fn position(&self) -> Point<Px>;
    fn render(
        &self,
        center: Point<Px>,
        zoom: f32,
        context: &mut Renderer<'_, '_>,
    ) -> Option<Duration>;
}

#[derive(Debug)]
pub struct DebugGrid;

impl TileSource for DebugGrid {
    fn render(
        &mut self,
        coordinate: Point<isize>,
        rect: Rect<Px>,
        context: &mut LayerContext<'_, '_, '_>,
    ) -> Option<Duration> {
        context.set_font_size(rect.size.height / 4);
        context.set_line_height(rect.size.height / 4);
        let color = Color::new(255, 255, 255, 64);
        context.draw_text(
            Text::new(&format!("{},{}", coordinate.x, coordinate.y), color)
                .translate_by(rect.origin),
        );
        context.draw_shape(
            &PathBuilder::new(Point::new(rect.origin.x, rect.origin.y + rect.size.height))
                .line_to(rect.origin)
                .line_to(Point::new(rect.origin.x + rect.size.width, rect.origin.y))
                .build()
                .stroke(StrokeOptions::default().colored(color)),
        );
        None
    }
}
