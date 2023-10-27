#![allow(missing_docs, clippy::missing_panics_doc)] // This file is a work in progress.

use std::fmt::Debug;
use std::ops::{Index, IndexMut};
use std::panic::{AssertUnwindSafe, UnwindSafe};

use alot::{LotId, OrderedLots};

use crate::figures::units::Px;
use crate::figures::{IntoSigned, Point, Rect, Size};
use crate::render::Renderer;
use crate::shapes::Shape;
use crate::text::Text;
use crate::{AnyTexture, Assert, Color};

pub const TILE_SIZE: Px = Px(32);

pub fn draw(layers: &impl Layers, focus: TileMapFocus, zoom: f32, graphics: &mut Renderer<'_, '_>) {
    let effective_zoom = graphics.scale().into_f32() * zoom;

    let offset = match focus {
        TileMapFocus::Point(focus) => focus,
        TileMapFocus::Object { layer, id } => layers
            .layer(layer)
            .assert("invalid focus layer")
            .find_object(id)
            .assert("focus not found"),
    };
    let offset = Point::new(offset.x * effective_zoom, offset.y * effective_zoom);

    let visible_size = graphics.clip_rect().size.into_signed();
    let visible_region = Rect::new(offset - visible_size / 2, visible_size);
    let tile_size = TILE_SIZE * effective_zoom;
    let top_left = first_tile(visible_region.origin, tile_size);
    let bottom_right = last_tile(visible_region.origin + visible_region.size, tile_size);

    for index in 0.. {
        let Some(layer) = layers.layer(index) else {
            break;
        };
        layer.render(
            &top_left,
            &bottom_right,
            tile_size,
            &visible_region,
            effective_zoom,
            graphics,
        );
    }

    graphics.draw_text(
        Text::<Px>::new(
            &format!(
                "Tile {}: Vis: {},{},{},{} - TL: {},{} / {},{}",
                tile_size,
                visible_region.origin.x,
                visible_region.origin.y,
                visible_region.size.width,
                visible_region.size.height,
                top_left.index.x,
                top_left.index.y,
                top_left.tile_offset.x,
                top_left.tile_offset.y
            ),
            Color::RED,
        ),
        Point::default(),
        None,
        None,
    );
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
            let floored = pos - offset;

            (-offset, floored)
        } else {
            (-remainder, pos - remainder)
        };
        let index =
            isize::try_from(floored.0 / tile_size.0).expect("tile size out of range of isize");
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
        let index = (pos.0 + pos.0.signum() * (tile_size.0 - 1)) / tile_size.0;
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

pub trait Layers: Debug + UnwindSafe + Send + 'static {
    fn layer(&self, index: usize) -> Option<&dyn Layer>;
}

impl<T> Layers for T
where
    T: Layer,
{
    fn layer(&self, index: usize) -> Option<&dyn Layer> {
        (index == 0).then_some(self)
    }
}

macro_rules! impl_layers_for_tuples {
    ($($type:ident : $index:tt),+) => {
        impl<$($type),+> Layers for ($($type),+) where $(
            $type: Debug + UnwindSafe + Send + Layer + 'static
        ),+ {
            fn layer(&self, index: usize) -> Option<&dyn Layer> {
                match index {
                    $($index => Some(&self.$index),)+
                    _ => None,
                }
            }
        }
    };
}

impl_layers_for_tuples!(T1: 0, T2: 1);

pub trait Layer: Debug + UnwindSafe + Send + 'static {
    fn render(
        &self,
        top_left: &TileOffset,
        bottom_right: &TileOffset,
        tile_size: Px,
        visible_rect: &Rect<Px>,
        zoom: f32,
        context: &mut Renderer<'_, '_>,
    );

    fn find_object(&self, _object: ObjectId) -> Option<Point<Px>> {
        None
    }
}

#[derive(Debug)]
pub struct Tiles {
    tiles: AssertUnwindSafe<Vec<TileKind>>,
    width: usize,
    height: usize,
}

impl Tiles {
    pub fn new(width: usize, height: usize, tiles: impl IntoIterator<Item = TileKind>) -> Self {
        let tiles = Vec::from_iter(tiles);
        assert_eq!(tiles.len(), width * height);
        Self {
            tiles: AssertUnwindSafe(tiles),
            width,
            height,
        }
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

impl Layer for Tiles {
    fn render(
        &self,
        top_left: &TileOffset,
        bottom_right: &TileOffset,
        tile_size: Px,
        _visible_rect: &Rect<Px>,
        _zoom: f32,
        context: &mut Renderer<'_, '_>,
    ) {
        let (Ok(right), Ok(bottom)) = (
            usize::try_from(bottom_right.index.x),
            usize::try_from(bottom_right.index.y),
        ) else {
            return;
        };

        let (x, left) = if let Ok(left) = usize::try_from(top_left.index.x) {
            (
                top_left.tile_offset.x + tile_size * isize_to_i32(top_left.index.x),
                left,
            )
        } else {
            let tile_offset = tile_size * isize_to_i32(-top_left.index.x);
            (top_left.tile_offset.x + tile_offset, 0)
        };
        let (mut y, top) = if let Ok(top) = usize::try_from(top_left.index.y) {
            (
                top_left.tile_offset.y + tile_size * isize_to_i32(top_left.index.y),
                top,
            )
        } else {
            let tile_offset =
                tile_size * i32::try_from(-top_left.index.y).expect("offset out of range");
            (top_left.tile_offset.y + tile_offset, 0)
        };

        let right = right.min(self.width - 1);
        let bottom = bottom.min(self.height - 1);

        if left <= right && top <= bottom {
            for y_index in top..=bottom {
                let mut x = x;
                for x_index in left..=right {
                    let tile_rect = Rect::new(Point::new(x, y), Size::squared(tile_size));
                    match &self.tiles[y_index * self.width + x_index] {
                        TileKind::Texture(texture) => {
                            // TODO aspect-fit rather than fill.
                            context.draw_texture(texture, tile_rect);
                        }
                        TileKind::Color(color) => context.draw_shape(
                            &Shape::filled_rect(tile_rect, *color),
                            Point::default(),
                            None,
                            None,
                        ),
                    };
                    x += tile_size;
                }
                y += tile_size;
            }
        }
    }
}

#[derive(Debug)]
pub enum TileKind {
    Texture(AnyTexture),
    Color(Color),
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
    fn render(
        &self,
        _top_left: &TileOffset,
        _bottom_right: &TileOffset,
        _tile_size: Px,
        visible_rect: &Rect<Px>,
        zoom: f32,
        context: &mut Renderer<'_, '_>,
    ) {
        for obj in &self.objects {
            let center =
                Point::new(obj.position().x * zoom, obj.position().y * zoom) - visible_rect.origin;

            obj.render(center, zoom, context);
        }
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

impl Default for TileMapFocus {
    fn default() -> Self {
        Self::Point(Point::default())
    }
}

pub trait Object: Debug + UnwindSafe + Send + 'static {
    fn position(&self) -> Point<Px>;
    fn render(&self, center: Point<Px>, zoom: f32, context: &mut Renderer<'_, '_>);
}
