use image::DynamicImage;
use std::os::raw::c_void;
use std::sync::{Arc, RwLock};

#[derive(Clone)]
pub struct Texture {
    pub(crate) storage: Arc<RwLock<TextureStorage>>,
}

impl From<DynamicImage> for Texture {
    fn from(img: DynamicImage) -> Self {
        Self {
            storage: Arc::new(RwLock::new(TextureStorage {
                image: img,
                compiled: None,
            })),
        }
    }
}

impl Texture {
    pub(crate) fn compile(&self) -> Arc<CompiledTexture> {
        let mut texture = self
            .storage
            .write()
            .expect("Error locking texture for write");
        let mut texture_id = 0u32;
        // TODO: Adjust this to allow for textures to know if there's an alpha channel or not, and properly enable/disable blending.
        // FOr now we are unoptimally making all textures RGBA, which is non-optimal.
        let image = texture.image.to_rgba();
        unsafe {
            gl::GenTextures(1, &mut texture_id);
            gl::ActiveTexture(gl::TEXTURE0);
            gl::BindTexture(gl::TEXTURE_2D, texture_id);
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_S, gl::REPEAT as i32);
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_T, gl::REPEAT as i32);
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, gl::LINEAR as i32);
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, gl::LINEAR as i32);
            gl::TexImage2D(
                gl::TEXTURE_2D,
                0,
                gl::RGBA as i32,
                image.width() as i32,
                image.height() as i32,
                0,
                gl::RGBA,
                gl::UNSIGNED_BYTE,
                image.as_ptr() as *const c_void,
            );
            gl::GenerateMipmap(gl::TEXTURE_2D);
            assert_eq!(gl::GetError(), 0);
        }
        let compiled = Arc::new(CompiledTexture { texture_id });
        texture.compiled = Some(compiled.clone());
        compiled
    }
}

pub(crate) struct TextureStorage {
    image: DynamicImage,
    compiled: Option<Arc<CompiledTexture>>,
}

#[derive(Clone, Debug)]
pub(crate) struct CompiledTexture {
    texture_id: u32,
}

impl Drop for CompiledTexture {
    fn drop(&mut self) {
        unsafe {
            gl::DeleteTextures(1, &self.texture_id);
            self.texture_id = 0;
        }
    }
}

impl CompiledTexture {
    pub fn activate(&self) {
        unsafe {
            gl::ActiveTexture(gl::TEXTURE0);
            gl::BindTexture(gl::TEXTURE_2D, self.texture_id);
        }
    }
}
