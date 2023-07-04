use std::marker::PhantomData;
use std::mem::size_of;

use wgpu::util::DeviceExt;

#[derive(Debug)]
pub struct Buffer<T> {
    pub wgpu: wgpu::Buffer,
    used: usize,
    count: usize,
    // usage: wgpu::BufferUsages,
    _phantom: PhantomData<T>,
}

impl<T> Buffer<T>
where
    T: bytemuck::Pod,
{
    pub fn new(contents: &[T], usage: wgpu::BufferUsages, device: &wgpu::Device) -> Self {
        let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: None,
            contents: bytemuck::cast_slice(contents),
            usage,
        });
        Self {
            wgpu: buffer,
            used: contents.len(),
            count: contents.len(),
            // usage,
            _phantom: PhantomData,
        }
    }

    pub fn update(&self, offset: usize, new_data: &[T], queue: &wgpu::Queue) {
        assert!(offset + new_data.len() <= self.count);
        queue.write_buffer(
            &self.wgpu,
            (size_of::<T>() * offset) as u64,
            bytemuck::cast_slice(new_data),
        );
    }

    // pub fn extend(
    //     &mut self,
    //     new_data: &[T],
    //     device: &wgpu::Device,
    //     queue: &wgpu::Queue,
    //     commands: &mut wgpu::CommandEncoder,
    // ) {
    //     let new_len = self.used + new_data.len();
    //     if new_len > self.count {
    //         // reallocate the buffer
    //         let new_size = new_len * 2;
    //         let new_buffer = device.create_buffer(&wgpu::BufferDescriptor {
    //             label: None,
    //             size: (size_of::<T>() * new_size) as u64,
    //             usage: self.usage,
    //             mapped_at_creation: false,
    //         });
    //         // Copy the existing buffer's data
    //         commands.copy_buffer_to_buffer(
    //             &self.wgpu,
    //             0,
    //             &new_buffer,
    //             0,
    //             (size_of::<T>() * self.used) as u64,
    //         );
    //         self.wgpu = new_buffer;
    //     }
    //     // Copy the new data into the buffer.
    //     let copy_start = self.used;
    //     self.used = new_len;
    //     self.update(copy_start, new_data, queue);
    // }

    pub const fn len(&self) -> usize {
        self.used
    }

    pub fn as_slice(&self) -> wgpu::BufferSlice<'_> {
        self.wgpu.slice(0..self.size() as u64)
    }

    pub const fn size(&self) -> usize {
        size_of::<T>() * self.len()
    }
}
