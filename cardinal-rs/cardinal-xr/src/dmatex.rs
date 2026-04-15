use std::os::fd::OwnedFd;

/// Information about an exported DMA-BUF.
pub struct DmaBufInfo {
    pub fd: OwnedFd,
    pub stride: u32,
    pub offset: u32,
    pub modifier: u64,
}

/// Manages a double-buffered texture pair for one module.
pub struct ModuleTextures {
    pub textures: [wgpu::Texture; 2],
    /// Which texture is currently being written (0 or 1).
    pub write_index: usize,
    /// Stardust dmatex IDs for each texture slot.
    pub dmatex_ids: [u64; 2],
    /// DMA-BUF file descriptors kept alive for as long as the textures exist.
    pub _dmabuf_fds: [Option<OwnedFd>; 2],
    pub timeline_point: u64,
    pub width: u32,
    pub height: u32,
}

impl ModuleTextures {
    /// Returns the texture currently being written to.
    pub fn write_texture(&self) -> &wgpu::Texture {
        &self.textures[self.write_index]
    }

    /// Returns the dmatex ID for the texture currently being read (i.e. the
    /// one that is *not* being written).
    pub fn read_dmatex_id(&self) -> u64 {
        self.dmatex_ids[1 - self.write_index]
    }

    /// Swap the write / read indices so the just-written texture becomes
    /// readable and the previously readable texture becomes writable.
    pub fn swap(&mut self) {
        self.write_index = 1 - self.write_index;
    }
}

/// Create a pair of standard wgpu textures suitable for module rendering.
///
/// The textures are `Rgba8Unorm` and support `RENDER_ATTACHMENT`,
/// `TEXTURE_BINDING`, and `COPY_SRC`.  External Vulkan memory flags for
/// DMA-BUF export are a TODO for a later pass once we can test against a
/// running system.
pub fn create_exportable_textures(
    device: &wgpu::Device,
    width: u32,
    height: u32,
) -> [wgpu::Texture; 2] {
    let descriptor = wgpu::TextureDescriptor {
        label: Some("module_texture"),
        size: wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT
            | wgpu::TextureUsages::TEXTURE_BINDING
            | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    };

    [device.create_texture(&descriptor), device.create_texture(&descriptor)]
}

/// Attempt to export a wgpu texture as a DMA-BUF.
///
/// This requires Vulkan HAL interop and is not yet implemented.  The
/// function logs a message and returns `None` so callers can fall back to
/// the CPU readback path.
pub fn export_dmabuf(_device: &wgpu::Device, _texture: &wgpu::Texture) -> Option<DmaBufInfo> {
    eprintln!("cardinal-xr/dmatex: export_dmabuf not yet implemented – DMA-BUF export requires Vulkan HAL interop");
    None
}

/// Create a Vulkan timeline syncobj for GPU–GPU synchronisation.
///
/// Not yet implemented; returns `None` so callers can use a CPU-side fence
/// instead.
pub fn create_timeline_syncobj() -> Option<OwnedFd> {
    None
}

/// Read back the pixels of a 2-D `Rgba8Unorm` texture to the CPU.
///
/// This is the fallback path used when DMA-BUF export is unavailable.
/// Returns a tightly-packed `Vec<u8>` of RGBA8 pixels in row-major order.
pub fn cpu_readback(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    texture: &wgpu::Texture,
    width: u32,
    height: u32,
) -> Vec<u8> {
    // wgpu requires that the bytes-per-row of a buffer copy is a multiple of
    // COPY_BYTES_PER_ROW_ALIGNMENT (256).  Each pixel is 4 bytes (RGBA8).
    let bytes_per_pixel: u32 = 4;
    let unpadded_bytes_per_row = width * bytes_per_pixel;
    let align = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
    let padded_bytes_per_row = (unpadded_bytes_per_row + align - 1) / align * align;
    let buffer_size = (padded_bytes_per_row * height) as u64;

    let staging_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("cpu_readback_staging"),
        size: buffer_size,
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });

    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("cpu_readback_encoder"),
    });

    encoder.copy_texture_to_buffer(
        wgpu::TexelCopyTextureInfo {
            texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        wgpu::TexelCopyBufferInfo {
            buffer: &staging_buffer,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(padded_bytes_per_row),
                rows_per_image: Some(height),
            },
        },
        wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
    );

    queue.submit(std::iter::once(encoder.finish()));

    // Map the staging buffer and wait for the GPU to finish.
    let buffer_slice = staging_buffer.slice(..);
    let (tx, rx) = std::sync::mpsc::channel();
    buffer_slice.map_async(wgpu::MapMode::Read, move |result| {
        tx.send(result).expect("cpu_readback: channel send failed");
    });
    device
        .poll(wgpu::PollType::wait_indefinitely())
        .expect("cpu_readback: device poll failed");
    rx.recv()
        .expect("cpu_readback: channel recv failed")
        .expect("cpu_readback: map_async failed");

    // Strip the row padding before returning.
    let mapped = buffer_slice.get_mapped_range();
    let mut pixels = Vec::with_capacity((unpadded_bytes_per_row * height) as usize);
    for row in 0..height as usize {
        let row_start = row * padded_bytes_per_row as usize;
        let row_end = row_start + unpadded_bytes_per_row as usize;
        pixels.extend_from_slice(&mapped[row_start..row_end]);
    }
    drop(mapped);
    staging_buffer.unmap();

    pixels
}
