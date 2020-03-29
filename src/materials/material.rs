use super::{solid, textured};
use crate::internal_prelude::*;
use crate::shaders::{CompiledProgram, Program};
use crate::texture::Texture;
use cgmath::Vector4;
use std::sync::{Arc, RwLock};

pub trait SimpleMaterial: Sync + Send {
    fn program(&self) -> KludgineResult<Program>;
    fn activate(&self, program: &CompiledProgram) -> KludgineResult<()>;
}

#[derive(Clone)]
pub enum MaterialKind {
    Solid {
        color: Color,
    },
    Textured {
        texture: Texture,
    },
    Custom {
        custom_material: Arc<Box<dyn SimpleMaterial>>,
    },
}

impl<T> From<T> for MaterialKind
where
    T: SimpleMaterial + Sized + 'static,
{
    fn from(from: T) -> Self {
        MaterialKind::Custom {
            custom_material: Arc::new(Box::new(from)),
        }
    }
}

pub(crate) struct MaterialStorage {
    pub kind: MaterialKind,
    compiled: Option<Arc<CompiledMaterial>>,
}

#[derive(Clone)]
pub struct Material {
    storage: Arc<RwLock<MaterialStorage>>,
}

impl From<MaterialKind> for Material {
    fn from(kind: MaterialKind) -> Self {
        let storage = Arc::new(RwLock::new(MaterialStorage {
            kind,
            compiled: None,
        }));
        Self { storage }
    }
}

impl Material {
    pub(crate) fn compile(&self) -> KludgineResult<Arc<CompiledMaterial>> {
        // Optimize for reading already compiled materials, try to acquire just for reading.
        {
            let storage = self.storage.read().expect("Error locking storage for read");
            if let Some(compiled) = storage.compiled.as_ref() {
                return Ok(compiled.clone());
            }
        }
        // Now we lock for compilation. Since we know this is a method that must be called from the
        // render thread, there's no way for the compiled variable to be assigned between the above early return and this.
        let mut storage = self
            .storage
            .write()
            .expect("Error locking storage for compilation");

        match &storage.kind {
            MaterialKind::Solid { color } => {
                let program = solid::program().compile()?;
                let simple_material = Arc::new(solid::simple_material(Vector4::new(
                    color.red as f32 / 255.0,
                    color.blue as f32 / 255.0,
                    color.green as f32 / 255.0,
                    color.alpha as f32 / 255.0,
                )));
                storage.compiled = Some(Arc::new(CompiledMaterial {
                    program,
                    simple_material,
                }));
            }
            MaterialKind::Textured { texture } => {
                let program = textured::program().compile()?;
                let simple_material = Arc::new(textured::simple_material(texture.compile()));
                storage.compiled = Some(Arc::new(CompiledMaterial {
                    program,
                    simple_material,
                }));
            }
            MaterialKind::Custom { custom_material } => {
                let program = custom_material.program()?.compile()?;
                storage.compiled = Some(Arc::new(CompiledMaterial {
                    program,
                    simple_material: custom_material.clone(),
                }))
            }
        }
        assert_eq!(unsafe { gl::GetError() }, 0);

        Ok(storage
            .compiled
            .as_ref()
            .expect("Reached end of compilation without a valid output")
            .clone())
    }
}

#[derive(Clone)]
pub(crate) struct CompiledMaterial {
    pub program: Arc<CompiledProgram>,
    pub simple_material: Arc<Box<dyn SimpleMaterial>>,
}

impl CompiledMaterial {
    pub(crate) fn activate(&self) -> KludgineResult<()> {
        self.program.activate();
        self.simple_material.activate(self.program.as_ref())
    }
}
