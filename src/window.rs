use crate::internal_prelude::*;
use crate::scene2d::Scene2d;
use glutin::event::{DeviceId, KeyboardInput, VirtualKeyCode};

pub enum CloseResponse {
    RemainOpen,
    Close,
}

#[async_trait]
pub trait Window: Send + Sync + 'static {
    async fn close_requested(&self) -> CloseResponse {
        CloseResponse::Close
    }
    async fn initialize(&mut self) {}
    async fn render_2d(&mut self, _scene: &mut Scene2d) -> KludgineResult<()> {
        Ok(())
    }
    async fn keyboard_event(
        &mut self,
        _device_id: DeviceId,
        _input: KeyboardInput,
    ) -> KludgineResult<()> {
        Ok(())
    }

    async fn key_down(&mut self, _device_id: DeviceId, _key: VirtualKeyCode) -> KludgineResult<()> {
        Ok(())
    }

    async fn key_up(&mut self, _device_id: DeviceId, _key: VirtualKeyCode) -> KludgineResult<()> {
        Ok(())
    }
}
