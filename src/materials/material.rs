use super::{solid, textured};
use crate::internal_prelude::*;
use crate::shaders::{CompiledProgram, Program};
use crate::texture::Texture;
use cgmath::Vector4;

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
        custom_material: KludgineHandle<dyn SimpleMaterial>,
    },
}

impl<T> From<KludgineHandle<T>> for MaterialKind
where
    T: SimpleMaterial + Sized + 'static,
{
    fn from(custom_material: KludgineHandle<T>) -> Self {
        MaterialKind::Custom { custom_material }
    }
}

pub(crate) struct MaterialStorage {
    pub kind: MaterialKind,
    compiled: Option<Arc<CompiledMaterial>>,
}

#[derive(Clone)]
pub struct Material {
    storage: KludgineHandle<MaterialStorage>,
}

impl From<MaterialKind> for Material {
    fn from(kind: MaterialKind) -> Self {
        let storage = KludgineHandle::wrap(MaterialStorage {
            kind,
            compiled: None,
        });
        Self { storage }
    }
}

impl<T> From<KludgineHandle<T>> for Material
where
    T: SimpleMaterial + Sized + 'static,
{
    fn from(simple_material: KludgineHandle<T>) -> Self {
        let kind: MaterialKind = simple_material.into();
        kind.into()
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

        let simple_material = match &storage.kind {
            MaterialKind::Solid { color } => solid::simple_material(Vector4::new(
                color.red as f32 / 255.0,
                color.blue as f32 / 255.0,
                color.green as f32 / 255.0,
                color.alpha as f32 / 255.0,
            )),
            MaterialKind::Textured { texture } => textured::simple_material(texture.compile()),
            MaterialKind::Custom { custom_material } => custom_material.clone(),
        };
        let program = {
            let material = simple_material.read().expect("Error reading material");
            material.program()?.compile()?
        };
        storage.compiled = Some(Arc::new(CompiledMaterial {
            program,
            simple_material,
        }));
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
    pub simple_material: KludgineHandle<dyn SimpleMaterial>,
}

impl CompiledMaterial {
    pub(crate) fn activate(&self) -> KludgineResult<()> {
        self.program.activate();
        let material = self.simple_material.read().expect("Error reading material");
        material.activate(self.program.as_ref())
    }
}
