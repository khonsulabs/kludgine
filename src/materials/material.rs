use super::{solid, textured};
use crate::internal_prelude::*;
use crate::shaders::CompiledProgram;
use crate::texture::{CompiledTexture, Texture};
use cgmath::Vector4;
use std::sync::{Arc, RwLock};

#[derive(Clone)]
pub enum MaterialKind {
    Solid { color: Color },
    Textured { texture: Texture },
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
                storage.compiled = Some(Arc::new(CompiledMaterial {
                    program,
                    color: Some(Vector4::new(
                        color.red as f32 / 255.0,
                        color.blue as f32 / 255.0,
                        color.green as f32 / 255.0,
                        color.alpha as f32 / 255.0,
                    )),
                    texture: None,
                }));
            }
            MaterialKind::Textured { texture } => {
                let program = textured::program().compile()?;
                storage.compiled = Some(Arc::new(CompiledMaterial {
                    program,
                    color: None,
                    texture: Some(texture.compile()),
                }));
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
    pub color: Option<Vector4<f32>>,
    pub texture: Option<Arc<CompiledTexture>>,
}

impl CompiledMaterial {
    pub(crate) fn activate(&self) {
        self.program.activate();
        if let Some(color) = &self.color {
            self.program.set_uniform_vec4("color", &color);
            if color.w < 1.0 {
                unsafe {
                    gl::Enable(gl::BLEND);
                    gl::BlendFunc(gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA);
                }
            } else {
                unsafe {
                    gl::Disable(gl::BLEND);
                }
            }
            assert_eq!(unsafe { gl::GetError() }, 0);
        }
        if let Some(texture) = &self.texture {
            self.program.set_uniform_1i("uniformTexture", 0);
            texture.activate();
        }
    }
}
