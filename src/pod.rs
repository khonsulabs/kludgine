//! Unsafe [`bytemuck::Pod`] implementations.
//!
//! # Safety
//!
//! Bytemuck prevents deriving `Pod` on any type that contains generics, because
//! it can't ensure that the generic types are tagged `repr(c)`. These
//! implementations are all safe because the types being wrapped all are
//! `repr(c)` and only contain u32/f32/i32.
#![allow(unsafe_code)]

use crate::math::{Dips, Pixels, Point};
use crate::pipeline::Vertex;

unsafe impl bytemuck::Pod for Point<Pixels> {}
unsafe impl bytemuck::Zeroable for Point<Pixels> {}
unsafe impl bytemuck::Pod for Point<Dips> {}
unsafe impl bytemuck::Zeroable for Point<Dips> {}
unsafe impl bytemuck::Pod for Point<i32> {}
unsafe impl bytemuck::Zeroable for Point<i32> {}
unsafe impl bytemuck::Pod for Point<f32> {}
unsafe impl bytemuck::Zeroable for Point<f32> {}

unsafe impl bytemuck::Pod for Vertex<Pixels> {}
unsafe impl bytemuck::Zeroable for Vertex<Pixels> {}
unsafe impl bytemuck::Pod for Vertex<Dips> {}
unsafe impl bytemuck::Zeroable for Vertex<Dips> {}
unsafe impl bytemuck::Pod for Vertex<i32> {}
unsafe impl bytemuck::Zeroable for Vertex<i32> {}
