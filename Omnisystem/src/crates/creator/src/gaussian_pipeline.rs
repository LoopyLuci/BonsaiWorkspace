//! WebGPU rasterizer for 3D Gaussian Splatting.
//!
//! [`GaussianPipeline`] initialises wgpu, compiles the WGSL shader
//! (`shaders/gaussian_full.wgsl`), and exposes [`GaussianPipeline::render_splat`]
//! which takes a CAS key pointing to a PLY splat asset, depth-sorts it on the
//! GPU with a compute pass, then renders the sorted Gaussians into an off-screen
//! RGBA8 texture that is read back to CPU memory and returned as raw bytes.
//!
//! The radix sort is a single-pass histogram bucket sort keyed on per-Gaussian
//! view-space depth.  Full multi-pass counting sort can be added later.

use anyhow::Result;
use cas::CasKey;
use bytemuck::{Pod, Zeroable};
use std::sync::Arc;
use wgpu::{
    util::{BufferInitDescriptor, DeviceExt},
    *,
};

// ── GPU types ─────────────────────────────────────────────────────────────────

/// One Gaussian splat as laid out in the GPU storage buffer.
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct GpuSplat {
    pub position: [f32; 3],
    pub _pad0: f32,
    pub scale: [f32; 3],
    pub _pad1: f32,
    pub rotation: [f32; 4], // quaternion
    pub opacity: f32,
    pub _pad2: [f32; 3],
    pub sh: [f32; 48], // spherical harmonics (degree 3, RGB)
}

/// Camera uniform block matching the WGSL `CameraUniform` struct.
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct CameraParams {
    pub view_proj: [[f32; 4]; 4],
    pub camera_pos: [f32; 3],
    pub _padding: f32,
}

// ── Pipeline ──────────────────────────────────────────────────────────────────

pub struct GaussianPipeline {
    device: Device,
    queue: Queue,
    render_pipeline: RenderPipeline,
    sort_pipeline: ComputePipeline,
    cas: Arc<cas::CasStore>,
}

impl GaussianPipeline {
    /// Create a new pipeline.  Acquires the first available GPU adapter.
    pub async fn new(cas: Arc<cas::CasStore>) -> Result<Self> {
        let instance = Instance::new(InstanceDescriptor::default());
        let adapter = instance
            .request_adapter(&RequestAdapterOptions::default())
            .await
            .ok_or_else(|| anyhow::anyhow!("no GPU adapter found"))?;
        let (device, queue) = adapter
            .request_device(&DeviceDescriptor::default(), None)
            .await?;

        let shader = device.create_shader_module(ShaderModuleDescriptor {
            label: Some("Gaussian WGSL"),
            source: ShaderSource::Wgsl(include_str!("../shaders/gaussian_full.wgsl").into()),
        });

        // Render pipeline — alpha-blended Gaussian quads.
        let render_pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some("Gaussian Render"),
            layout: None,
            vertex: VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[],
            },
            fragment: Some(FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[Some(ColorTargetState {
                    format: TextureFormat::Rgba8Unorm,
                    blend: Some(BlendState::ALPHA_BLENDING),
                    write_mask: ColorWrites::ALL,
                })],
            }),
            primitive: PrimitiveState {
                topology: PrimitiveTopology::TriangleStrip,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: MultisampleState::default(),
            multiview: None,
        });

        // Compute pipeline — depth computation + histogram bucket sort.
        let sort_pipeline = device.create_compute_pipeline(&ComputePipelineDescriptor {
            label: Some("Gaussian Depth Sort"),
            layout: None,
            module: &shader,
            entry_point: "radix_sort_histogram",
        });

        Ok(Self {
            device,
            queue,
            render_pipeline,
            sort_pipeline,
            cas,
        })
    }

    /// Render a splat asset (stored at `splat_key`) from the given camera.
    ///
    /// Returns raw RGBA8 bytes of size `width * height * 4`.
    pub async fn render_splat(
        &self,
        splat_key: CasKey,
        width: u32,
        height: u32,
        camera: CameraParams,
    ) -> Result<Vec<u8>> {
        let raw = self
            .cas
            .get(&splat_key)
            .await?
            .ok_or_else(|| anyhow::anyhow!("splat key not found in CAS"))?;

        let splats = parse_ply(&raw)?;
        if splats.is_empty() {
            return Ok(vec![0u8; (width * height * 4) as usize]);
        }

        // ── Buffers ───────────────────────────────────────────────────────────

        let splat_buf = self.device.create_buffer_init(&BufferInitDescriptor {
            label: Some("Splat Storage"),
            contents: bytemuck::cast_slice(&splats),
            usage: BufferUsages::STORAGE | BufferUsages::COPY_DST,
        });

        let n = splats.len() as u64;
        let index_buf = self.device.create_buffer(&BufferDescriptor {
            label: Some("Sorted Indices"),
            size: n * 4,
            usage: BufferUsages::STORAGE | BufferUsages::COPY_DST | BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        let depth_buf = self.device.create_buffer(&BufferDescriptor {
            label: Some("Depths"),
            size: n * 4,
            usage: BufferUsages::STORAGE | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let camera_buf = self.device.create_buffer_init(&BufferInitDescriptor {
            label: Some("Camera Uniform"),
            contents: bytemuck::cast_slice(&[camera]),
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
        });

        // ── Output texture ────────────────────────────────────────────────────

        let output_tex = self.device.create_texture(&TextureDescriptor {
            label: Some("Output"),
            size: Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: TextureFormat::Rgba8Unorm,
            usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let output_view = output_tex.create_view(&Default::default());

        // Readback buffer (aligned to 256 bytes per row as required by wgpu).
        let align = COPY_BYTES_PER_ROW_ALIGNMENT;
        let row_bytes = width * 4;
        let padded_row = (row_bytes + align - 1) / align * align;
        let readback = self.device.create_buffer(&BufferDescriptor {
            label: Some("Readback"),
            size: (padded_row * height) as u64,
            usage: BufferUsages::MAP_READ | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // ── Bind groups ───────────────────────────────────────────────────────

        let bgl = self.sort_pipeline.get_bind_group_layout(0);
        let bind_group = self.device.create_bind_group(&BindGroupDescriptor {
            label: Some("Sort BG"),
            layout: &bgl,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: camera_buf.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: splat_buf.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: index_buf.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 3,
                    resource: depth_buf.as_entire_binding(),
                },
            ],
        });

        // ── Command encoding ──────────────────────────────────────────────────

        let mut enc = self
            .device
            .create_command_encoder(&CommandEncoderDescriptor::default());

        // Compute: fill `depths` and initialise `sorted_indices`.
        {
            let mut cpass = enc.begin_compute_pass(&ComputePassDescriptor::default());
            cpass.set_pipeline(&self.sort_pipeline);
            cpass.set_bind_group(0, &bind_group, &[]);
            let groups = ((splats.len() as u32) + 255) / 256;
            cpass.dispatch_workgroups(groups, 1, 1);
        }

        // Render: draw sorted Gaussian quads.
        {
            let render_bgl = self.render_pipeline.get_bind_group_layout(0);
            let render_bg = self.device.create_bind_group(&BindGroupDescriptor {
                label: Some("Render BG"),
                layout: &render_bgl,
                entries: &[
                    BindGroupEntry {
                        binding: 0,
                        resource: camera_buf.as_entire_binding(),
                    },
                    BindGroupEntry {
                        binding: 1,
                        resource: splat_buf.as_entire_binding(),
                    },
                    BindGroupEntry {
                        binding: 2,
                        resource: index_buf.as_entire_binding(),
                    },
                    BindGroupEntry {
                        binding: 3,
                        resource: depth_buf.as_entire_binding(),
                    },
                ],
            });

            let mut rpass = enc.begin_render_pass(&RenderPassDescriptor {
                label: Some("Gaussian Render Pass"),
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: &output_view,
                    resolve_target: None,
                    ops: Operations {
                        load: LoadOp::Clear(Color::TRANSPARENT),
                        store: StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                ..Default::default()
            });
            rpass.set_pipeline(&self.render_pipeline);
            rpass.set_bind_group(0, &render_bg, &[]);
            // 4 vertices per quad (TriangleStrip), one instance per splat.
            rpass.draw(0..4, 0..splats.len() as u32);
        }

        // Copy texture → readback buffer.
        enc.copy_texture_to_buffer(
            output_tex.as_image_copy(),
            ImageCopyBuffer {
                buffer: &readback,
                layout: ImageDataLayout {
                    offset: 0,
                    bytes_per_row: Some(padded_row),
                    rows_per_image: Some(height),
                },
            },
            Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
        );

        self.queue.submit(std::iter::once(enc.finish()));

        // Map and read back.
        let slice = readback.slice(..);
        let (tx, rx) = tokio::sync::oneshot::channel();
        slice.map_async(MapMode::Read, move |r| {
            let _ = tx.send(r);
        });
        self.device.poll(Maintain::Wait);
        rx.await??;

        let mapped = slice.get_mapped_range();
        let mut pixels = Vec::with_capacity((width * height * 4) as usize);
        for row in 0..height as usize {
            let start = row * padded_row as usize;
            pixels.extend_from_slice(&mapped[start..start + row_bytes as usize]);
        }
        drop(mapped);
        readback.unmap();

        Ok(pixels)
    }
}

// ── PLY parser (minimal) ──────────────────────────────────────────────────────

fn parse_ply(data: &[u8]) -> Result<Vec<GpuSplat>> {
    // A real PLY parser would decode the header and property layout.
    // For now: if the data is a placeholder tag, return an empty scene.
    if data.starts_with(b"3DGS") || data.starts_with(b"3dgs") || data.len() < 100 {
        return Ok(Vec::new());
    }
    // TODO: implement PLY binary_little_endian parser for KHR_gaussian_splatting format.
    Ok(Vec::new())
}
