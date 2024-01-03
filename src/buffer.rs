use std::marker::PhantomData;
use std::mem::size_of;
use std::ops::Deref;

use wgpu::util::DeviceExt;

/// A GPU-managed memory buffer.
///
/// This type uses `bytemuck::Pod` to access the bytes of `T` when copying
/// to/from the gpu.
#[derive(Debug)]
pub struct Buffer<T> {
    /// The wgpu bufffer handle.
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
    /// Returns a new buffer containing `contents`.
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
            _phantom: PhantomData,
        }
    }

    /// Updates a portion of this buffer using `new_data`.
    ///
    /// # Panics
    ///
    /// This function will panic if the written data goes beyond the bounds of
    /// the buffer.
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

    /// Returns the current valid length of this buffer.
    pub const fn len(&self) -> usize {
        self.used
    }

    /// Returns the entire contents of this buffer as a [`wgpu::BufferSlice`].
    pub fn as_slice(&self) -> wgpu::BufferSlice<'_> {
        self.wgpu.slice(0..self.size() as u64)
    }

    /// Returns the number of bytes contained in this buffer.
    pub const fn size(&self) -> usize {
        size_of::<T>() * self.len()
    }
}

/// A GPU-buffer that tries to minimize copying when uploading new data.
#[derive(Debug)]
pub struct DiffableBuffer<T> {
    buffer: Buffer<T>,
    usage: wgpu::BufferUsages,
    data: Vec<T>,
}

impl<T> DiffableBuffer<T>
where
    T: bytemuck::Pod + Clone + Eq,
{
    /// Returns a new buffer containing `contents`.
    pub fn new(contents: &[T], usage: wgpu::BufferUsages, device: &wgpu::Device) -> Self {
        let usage = usage | wgpu::BufferUsages::COPY_DST;
        let buffer = Buffer::new(contents, usage, device);
        Self {
            buffer,
            usage,
            data: contents.to_vec(),
        }
    }

    /// Updates the contenst of this buffer with `new_contents`.
    ///
    /// This function attempts to strike a balance between copying only data
    /// that has changed and minimizing the number of individual copy commands
    /// issued to `queue`.
    pub fn update(&mut self, new_contents: &[T], device: &wgpu::Device, queue: &wgpu::Queue) {
        if new_contents.len() <= self.buffer.len() {
            let mut index = 0;
            let mut cant_align = false;

            while index < new_contents.len() {
                if new_contents[index] != self.data[index] {
                    let mut start_index = index;
                    // We found a changed element. Find where we should stop.
                    let mut last_changed = start_index;
                    while index < new_contents.len() {
                        if new_contents[index] == self.data[index] {
                            // We found a matching element that we don't need to
                            // update. We might want to overwrite it anyways,
                            // however, to minimize the number of GPU writes we're
                            // performing.
                            if last_changed - start_index >= 16 {
                                break;
                            }
                        } else {
                            last_changed = index;
                        }
                        index += 1;
                    }

                    if (size_of::<T>() * start_index) as u64 % wgpu::COPY_BUFFER_ALIGNMENT != 0 {
                        if start_index > 0
                            && (size_of::<T>() * (start_index - 1)) as u64
                                % wgpu::COPY_BUFFER_ALIGNMENT
                                == 0
                        {
                            start_index -= 1;
                        } else {
                            cant_align = true;
                            break;
                        }
                    }

                    if (size_of::<T>() * (last_changed + 1 - start_index)) as u64
                        % wgpu::COPY_BUFFER_ALIGNMENT
                        != 0
                    {
                        if last_changed + 1 < self.len()
                            && (size_of::<T>() * (last_changed + 2 - start_index)) as u64
                                % wgpu::COPY_BUFFER_ALIGNMENT
                                == 0
                        {
                            // Extend the copy range by 1
                            last_changed += 1;
                        } else {
                            // What weird alignment is this? Internally Vertex
                            // is aligned to 4 bytes, and u16 is the only
                            // odd-man out.
                            cant_align = true;
                            break;
                        }
                    }

                    // Update the changed range in the buffers.
                    let copy_range = &new_contents[start_index..=last_changed];
                    self.buffer.update(start_index, copy_range, queue);
                    self.data[start_index..=last_changed].copy_from_slice(copy_range);
                }
                index += 1;
            }

            // If we were able to do delta updates without alignment issues, we
            // can avoid creating the new buffer.
            if !cant_align {
                return;
            }
        }

        // We need to grow to store the new data, or we had alignment issues
        // when trying to do a delta update.
        self.buffer = Buffer::new(new_contents, self.usage, device);
        self.data.clear();
        self.data.extend_from_slice(new_contents);
    }
}

impl<T> Deref for DiffableBuffer<T> {
    type Target = Buffer<T>;

    fn deref(&self) -> &Self::Target {
        &self.buffer
    }
}
