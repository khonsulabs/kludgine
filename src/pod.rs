//! Unsafe [`bytemuck::Pod`] implementations.
//!
//! # Safety
//!
//! Bytemuck prevents deriving `Pod` on any type that contains generics, because
//! it can't ensure that the generic types are tagged `repr(c)`. These
//! implementations are all safe because the types being wrapped all are
//! `repr(c)` and only contain u32/f32/i32.
#![allow(unsafe_code)]

use figures::units::{Dips, Px};

use crate::pipeline::Vertex;

unsafe impl bytemuck::Pod for Vertex<Px> {}
unsafe impl bytemuck::Zeroable for Vertex<Px> {}
unsafe impl bytemuck::Pod for Vertex<Dips> {}
unsafe impl bytemuck::Zeroable for Vertex<Dips> {}
unsafe impl bytemuck::Pod for Vertex<i32> {}
unsafe impl bytemuck::Zeroable for Vertex<i32> {}
