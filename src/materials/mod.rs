pub mod material;
pub mod solid;

pub use material::Material;

pub mod prelude {
    pub(crate) use super::material::CompiledMaterial;
    pub use super::material::{Material, MaterialKind};
}
