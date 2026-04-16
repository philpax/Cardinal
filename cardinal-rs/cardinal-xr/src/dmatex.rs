//! DMA-BUF texture export from wgpu and import into Stardust XR.
//!
//! Creates wgpu textures backed by Vulkan external memory (DMA-BUF),
//! exports the DMA-BUF file descriptors, and imports them into Stardust
//! via the dmatex protocol for zero-copy GPU texture sharing.

use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};

use ash::vk;

/// Information about an exported DMA-BUF.
pub struct DmaBufInfo {
    pub fd: OwnedFd,
    pub stride: u32,
    pub offset: u32,
    pub drm_format_modifier: u64,
}

/// A single texture backed by exportable Vulkan memory.
pub struct ExportableTexture {
    /// The wgpu texture (wraps the Vulkan image).
    pub texture: wgpu::Texture,
    /// The exported DMA-BUF fd.
    pub dmabuf: DmaBufInfo,
    /// The raw Vulkan device memory (kept for lifetime management).
    _vk_memory: vk::DeviceMemory,
}

/// Manages a double-buffered pair of DMA-BUF textures for one module,
/// along with the timeline syncobj for GPU synchronization.
pub struct ModuleTextures {
    pub textures: [ExportableTexture; 2],
    /// Which texture is currently being written (0 or 1).
    pub write_index: usize,
    /// Stardust dmatex IDs for each texture slot.
    pub dmatex_ids: [u64; 2],
    /// Current timeline synchronization point.
    pub timeline_point: u64,
    /// Timeline syncobj fd (shared with Stardust server).
    pub syncobj_fd: OwnedFd,
    pub width: u32,
    pub height: u32,
}

impl ModuleTextures {
    /// The texture that Cardinal should render to this frame.
    pub fn write_texture(&self) -> &wgpu::Texture {
        &self.textures[self.write_index].texture
    }

    /// The dmatex ID of the texture Stardust should read (the one not being written).
    pub fn read_dmatex_id(&self) -> u64 {
        self.dmatex_ids[1 - self.write_index]
    }

    /// Swap write/read indices and bump timeline point.
    pub fn swap(&mut self) {
        self.write_index = 1 - self.write_index;
        self.timeline_point += 1;
    }
}

/// DRM fourcc code for ABGR8888 (matches Rgba8Unorm memory layout).
/// R in byte 0, G in byte 1, B in byte 2, A in byte 3.
const DRM_FORMAT_ABGR8888: u32 = 0x34324241;

/// Create a wgpu texture backed by Vulkan external memory that can be
/// exported as a DMA-BUF fd.
///
/// Returns the wgpu::Texture, the DMA-BUF info, and the raw VkDeviceMemory.
///
/// # Safety
/// Requires the Vulkan device to support VK_KHR_external_memory_fd and
/// VK_EXT_external_memory_dma_buf extensions.
unsafe fn create_exportable_texture(
    device: &wgpu::Device,
    width: u32,
    height: u32,
) -> Result<ExportableTexture, Box<dyn std::error::Error>> {
    // Access the raw Vulkan device through wgpu's HAL layer.
    let hal_device_guard = unsafe {
        device
            .as_hal::<wgpu::hal::vulkan::Api>()
            .ok_or("Failed to get Vulkan HAL device")?
    };

    let raw_device: &ash::Device = hal_device_guard.raw_device();
    let physical_device = hal_device_guard.raw_physical_device();
    let instance = hal_device_guard.shared_instance().raw_instance();

    // Load the external memory fd extension.
    let ext_memory_fd =
        ash::khr::external_memory_fd::Device::new(instance, raw_device);

    // 1. Create VkImage with external memory flags.
    let mut external_memory_info = vk::ExternalMemoryImageCreateInfo::default()
        .handle_types(vk::ExternalMemoryHandleTypeFlags::DMA_BUF_EXT);

    let image_create_info = vk::ImageCreateInfo::default()
        .image_type(vk::ImageType::TYPE_2D)
        .format(vk::Format::R8G8B8A8_UNORM)
        .extent(vk::Extent3D {
            width,
            height,
            depth: 1,
        })
        .mip_levels(1)
        .array_layers(1)
        .samples(vk::SampleCountFlags::TYPE_1)
        .tiling(vk::ImageTiling::OPTIMAL) // OPTIMAL for render attachment support
        .usage(
            vk::ImageUsageFlags::COLOR_ATTACHMENT
                | vk::ImageUsageFlags::SAMPLED
                | vk::ImageUsageFlags::TRANSFER_SRC,
        )
        .sharing_mode(vk::SharingMode::EXCLUSIVE)
        .initial_layout(vk::ImageLayout::UNDEFINED)
        .push_next(&mut external_memory_info);

    let vk_image = unsafe { raw_device.create_image(&image_create_info, None)? };

    // 2. Get memory requirements.
    let mem_requirements = unsafe { raw_device.get_image_memory_requirements(vk_image) };

    // Find a memory type that supports device-local + host-visible
    // (needed for LINEAR tiling), with export capability.
    let memory_properties = unsafe {
        instance.get_physical_device_memory_properties(physical_device)
    };

    let memory_type_index = find_memory_type(
        &memory_properties,
        mem_requirements.memory_type_bits,
        vk::MemoryPropertyFlags::DEVICE_LOCAL,
    )
    .ok_or("No suitable memory type for DMA-BUF export")?;

    // 3. Allocate memory with export flags.
    let mut export_info = vk::ExportMemoryAllocateInfo::default()
        .handle_types(vk::ExternalMemoryHandleTypeFlags::DMA_BUF_EXT);

    let mut dedicated_info = vk::MemoryDedicatedAllocateInfo::default().image(vk_image);

    let alloc_info = vk::MemoryAllocateInfo::default()
        .allocation_size(mem_requirements.size)
        .memory_type_index(memory_type_index as u32)
        .push_next(&mut export_info)
        .push_next(&mut dedicated_info);

    let vk_memory = unsafe { raw_device.allocate_memory(&alloc_info, None)? };

    // 4. Bind memory to image.
    unsafe { raw_device.bind_image_memory(vk_image, vk_memory, 0)? };

    // 5. Export memory as DMA-BUF fd.
    let get_fd_info = vk::MemoryGetFdInfoKHR::default()
        .memory(vk_memory)
        .handle_type(vk::ExternalMemoryHandleTypeFlags::DMA_BUF_EXT);

    let raw_fd = unsafe { ext_memory_fd.get_memory_fd(&get_fd_info)? };
    let dmabuf_fd = unsafe { OwnedFd::from_raw_fd(raw_fd) };

    // 6. Query the DRM format modifier that was actually chosen.
    // With OPTIMAL tiling, we can't use get_image_subresource_layout.
    // The stride is not meaningful for OPTIMAL images — the server will
    // determine it from the modifier. We pass 0 for stride/offset.

    // DRM format modifier: with OPTIMAL tiling + DMA-BUF export, the driver
    // chooses the modifier internally. We report DRM_FORMAT_MOD_INVALID (which
    // tells the server to detect it from the DMA-BUF itself) or 0 (LINEAR).
    // For simplicity, we use 0 (LINEAR) which most drivers support.
    let drm_format_modifier = 0u64;

    // 7. Wrap as a wgpu HAL texture, then as a wgpu texture.
    let hal_desc = wgpu::hal::TextureDescriptor {
        label: Some("dmatex_module_texture"),
        size: wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu_types::TextureUses::COLOR_TARGET
            | wgpu_types::TextureUses::RESOURCE
            | wgpu_types::TextureUses::COPY_SRC,
        memory_flags: wgpu_hal::MemoryFlags::empty(),
        view_formats: vec![],
    };

    // When this texture is dropped, we need to clean up the Vulkan resources.
    let device_clone = raw_device.clone();
    let image_for_drop = vk_image;
    let memory_for_drop = vk_memory;
    let drop_callback: wgpu::hal::DropCallback = Box::new(move || {
        unsafe {
            device_clone.destroy_image(image_for_drop, None);
            device_clone.free_memory(memory_for_drop, None);
        }
    });

    let hal_texture = unsafe {
        hal_device_guard.texture_from_raw(
            vk_image,
            &hal_desc,
            Some(drop_callback),
            wgpu_hal::vulkan::TextureMemory::External,
        )
    };

    let wgpu_desc = wgpu::TextureDescriptor {
        label: Some("dmatex_module_texture"),
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

    let wgpu_texture = unsafe {
        device.create_texture_from_hal::<wgpu::hal::vulkan::Api>(hal_texture, &wgpu_desc)
    };

    Ok(ExportableTexture {
        texture: wgpu_texture,
        dmabuf: DmaBufInfo {
            fd: dmabuf_fd,
            stride: width * 4, // Best guess for OPTIMAL; server derives from modifier
            offset: 0,
            drm_format_modifier,
        },
        _vk_memory: vk_memory,
    })
}

fn find_memory_type(
    properties: &vk::PhysicalDeviceMemoryProperties,
    type_bits: u32,
    required_flags: vk::MemoryPropertyFlags,
) -> Option<usize> {
    for i in 0..properties.memory_type_count as usize {
        if (type_bits & (1 << i)) != 0
            && properties.memory_types[i]
                .property_flags
                .contains(required_flags)
        {
            return Some(i);
        }
    }
    None
}

/// Create a pair of DMA-BUF-exportable textures for double-buffered rendering.
///
/// Returns None if the Vulkan device doesn't support DMA-BUF export.
pub fn create_exportable_texture_pair(
    device: &wgpu::Device,
    width: u32,
    height: u32,
) -> Option<[ExportableTexture; 2]> {
    let t0 = unsafe { create_exportable_texture(device, width, height) };
    let t1 = unsafe { create_exportable_texture(device, width, height) };

    match (t0, t1) {
        (Ok(t0), Ok(t1)) => Some([t0, t1]),
        (Err(e), _) | (_, Err(e)) => {
            eprintln!("cardinal-xr/dmatex: failed to create exportable textures: {e}");
            None
        }
    }
}

/// Open the DRM render node that matches the wgpu device's physical device.
pub fn open_matching_drm_render_node(device: &wgpu::Device) -> Option<timeline_syncobj::render_node::DrmRenderNode> {
    // Query the physical device's DRM properties to find the right render node
    let render_minor = unsafe {
        let hal_guard = device.as_hal::<wgpu::hal::vulkan::Api>()?;
        let instance = hal_guard.shared_instance().raw_instance();
        let physical_device = hal_guard.raw_physical_device();

        let mut drm_props = vk::PhysicalDeviceDrmPropertiesEXT::default();
        let mut props2 = vk::PhysicalDeviceProperties2::default().push_next(&mut drm_props);
        instance.get_physical_device_properties2(physical_device, &mut props2);

        if drm_props.has_render != 0 {
            Some(drm_props.render_minor as u64)
        } else {
            None
        }
    };

    if let Some(minor) = render_minor {
        eprintln!("cardinal-xr/dmatex: wgpu device uses DRM render node minor {minor}");
        match timeline_syncobj::render_node::DrmRenderNode::new(minor) {
            Ok(node) => return Some(node),
            Err(e) => eprintln!("cardinal-xr/dmatex: failed to open renderD{minor}: {e}"),
        }
    }

    // Fallback: try all render nodes
    eprintln!("cardinal-xr/dmatex: falling back to scanning render nodes");
    for i in 128..144 {
        if let Ok(node) = timeline_syncobj::render_node::DrmRenderNode::new(i) {
            eprintln!("cardinal-xr/dmatex: opened renderD{i}");
            return Some(node);
        }
    }
    eprintln!("cardinal-xr/dmatex: no DRM render node found");
    None
}

/// A DRM timeline syncobj for GPU synchronization.
pub struct SyncobjState {
    /// The timeline syncobj (for signaling and waiting).
    pub syncobj: timeline_syncobj::timeline_syncobj::TimelineSyncObj,
    /// An exported fd that can be sent to other processes.
    pub fd: OwnedFd,
    /// Whether the TIMELINE export flag was used (true = dmatex path works).
    /// False on NVIDIA where TIMELINE export fails with EINVAL.
    pub timeline_exported: bool,
}

/// Create a DRM timeline syncobj and return it with an exported fd.
pub fn create_timeline_syncobj(render_node: &timeline_syncobj::render_node::DrmRenderNode) -> Option<SyncobjState> {
    let syncobj = match timeline_syncobj::timeline_syncobj::TimelineSyncObj::create(render_node) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("cardinal-xr/dmatex: failed to create timeline syncobj: {e}");
            return None;
        }
    };

    // Signal point 0 to initialize the timeline before exporting.
    // Some drivers need at least one timeline operation before TIMELINE export works.
    if let Err(e) = unsafe { syncobj.signal(0) } {
        eprintln!("cardinal-xr/dmatex: failed to signal initial point 0: {e}");
    }

    // Export with TIMELINE flag. Falls back to plain export if unsupported
    // (e.g. older kernels missing required DRM features).
    let (fd, timeline_exported) = match syncobj.export() {
        Ok(fd) => (fd, true),
        Err(e) => {
            eprintln!("cardinal-xr/dmatex: TIMELINE export failed ({e}), trying plain export");
            eprintln!("cardinal-xr/dmatex: dmatex path will be disabled — server needs TIMELINE syncobj");
            match export_syncobj_fd(render_node, &syncobj) {
                Some(fd) => (fd, false),
                None => return None,
            }
        }
    };

    Some(SyncobjState { syncobj, fd, timeline_exported })
}

/// Export a syncobj as a plain fd (without TIMELINE flag).
/// Some drivers don't support the TIMELINE flag on HANDLE_TO_FD.
fn export_syncobj_fd(
    render_node: &timeline_syncobj::render_node::DrmRenderNode,
    syncobj: &timeline_syncobj::timeline_syncobj::TimelineSyncObj,
) -> Option<OwnedFd> {
    use std::os::fd::AsFd;

    // Get the raw handle from the syncobj — it's a newtype(u32)
    let handle = unsafe { syncobj.get_raw_handle() };
    // Transmute the newtype to u32 (they have the same repr)
    let handle_u32: u32 = unsafe { std::mem::transmute(handle) };

    #[repr(C)]
    struct DrmSyncobjHandleToFd {
        handle: u32,
        flags: u32,
        fd: i32,
        pad: u32,
        point: u64,
    }

    let mut req = DrmSyncobjHandleToFd {
        handle: handle_u32,
        flags: 0, // No TIMELINE flag — plain syncobj fd
        fd: -1,
        pad: 0,
        point: 0,
    };

    let render_fd = render_node.as_fd();

    // DRM_IOCTL_SYNCOBJ_HANDLE_TO_FD
    let ret = unsafe {
        libc::ioctl(
            render_fd.as_raw_fd(),
            0xC018_64C1_u64 as libc::c_ulong,
            &mut req as *mut DrmSyncobjHandleToFd,
        )
    };
    if ret != 0 {
        eprintln!("cardinal-xr/dmatex: syncobj HANDLE_TO_FD (no TIMELINE) failed: {}", std::io::Error::last_os_error());
        return None;
    }

    Some(unsafe { OwnedFd::from_raw_fd(req.fd) })
}

/// Signal a timeline syncobj at a specific point.
pub fn signal_timeline(syncobj: &timeline_syncobj::timeline_syncobj::TimelineSyncObj, point: u64) -> bool {
    match unsafe { syncobj.signal(point) } {
        Ok(()) => true,
        Err(e) => {
            eprintln!("cardinal-xr/dmatex: failed to signal timeline at point {point}: {e}");
            false
        }
    }
}

/// The DRM format fourcc code for our textures (ABGR8888 = Rgba8Unorm).
pub fn drm_format() -> u32 {
    DRM_FORMAT_ABGR8888
}

/// Read back the pixels of a 2-D `Rgba8Unorm` texture to the CPU.
///
/// This is the fallback path used when DMA-BUF export is unavailable.
pub fn cpu_readback(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    texture: &wgpu::Texture,
    width: u32,
    height: u32,
) -> Vec<u8> {
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
