pub mod material;
mod solid;
mod textured;

pub use material::Material;

pub mod prelude {
    pub(crate) use super::material::CompiledMaterial;
    pub use super::material::{Material, MaterialKind, SimpleMaterial};
}
