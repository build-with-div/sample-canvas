//! A subset of three.js 0.187.0's `WebGPURenderer`
//! (`src/renderers/webgpu/`, `src/renderers/common/`) on wgpu: the
//! subset that draws a `Scene` of `Mesh`es with `MeshBasicMaterial` into a
//! `RenderTarget` and reads its pixels back
//! (`readRenderTargetPixelsAsync`).
//!
//! It keeps what decides the pixels:
//! - the scene background as the clear color;
//! - the opaque render list's order (`painterSortStable`);
//! - the pipeline state;
//! - the uniforms, as `f32`;
//! - the WGSL that three.js r187's node builder generates for a
//!   `MeshBasicMaterial` with a color and no map.
//!
//! The node system (TSL) that generates that WGSL is not ported. The two
//! shaders in `src/wgsl` are its output, recorded from three.js in
//! Chromium.
//!
//! Frustum culling is not ported either. It only skips meshes that would
//! draw no pixels.
//!
//! The renderer takes a wgpu device rather than creating one. Div has no
//! `navigator.gpu` to share a device from, so the caller creates it
//! (`// div-gap: W17`, gap 14). It renders into a `RenderTarget` only: Div
//! has no `GPUCanvasContext` to present a canvas texture with (W17, gap
//! 13).

use three::math::{CoordinateSystem, Matrix4};
use three::{Index, PerspectiveCamera, Scene};
use wgpu::util::DeviceExt;

/// The vertex shader three.js generates for `MeshBasicMaterial`.
pub const MESH_BASIC_VERTEX: &str = include_str!("wgsl/mesh-basic.vertex.wgsl");
/// The fragment shader three.js generates for `MeshBasicMaterial` with a
/// color, no map and opacity 1.
pub const MESH_BASIC_FRAGMENT: &str = include_str!("wgsl/mesh-basic.fragment.wgsl");

/// `new RenderTarget(width, height)`: an `rgba8unorm` color texture
/// (`UnsignedByteType`, `RGBAFormat`, no color space conversion) with a
/// `depth24plus` depth buffer.
pub struct RenderTarget {
    pub width: u32,
    pub height: u32,
    color: wgpu::Texture,
    depth: wgpu::Texture,
}

/// The color format of a `RenderTarget`'s texture.
pub const TARGET_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;
const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth24Plus;

impl RenderTarget {
    pub fn new(device: &wgpu::Device, width: u32, height: u32) -> RenderTarget {
        // three.js gives both textures usage 23: COPY_SRC | COPY_DST |
        // TEXTURE_BINDING | RENDER_ATTACHMENT.
        let usage = wgpu::TextureUsages::COPY_SRC
            | wgpu::TextureUsages::COPY_DST
            | wgpu::TextureUsages::TEXTURE_BINDING
            | wgpu::TextureUsages::RENDER_ATTACHMENT;
        let texture = |format| {
            device.create_texture(&wgpu::TextureDescriptor {
                label: None,
                size: wgpu::Extent3d {
                    width,
                    height,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format,
                usage,
                view_formats: &[],
            })
        };
        RenderTarget {
            width,
            height,
            color: texture(TARGET_FORMAT),
            depth: texture(DEPTH_FORMAT),
        }
    }
}

/// One entry of the render list: the mesh's index in `scene.children` and
/// its sort keys.
struct RenderItem {
    mesh: usize,
    id: u32,
    z: f64,
}

pub struct WebGpuRenderer {
    device: wgpu::Device,
    queue: wgpu::Queue,
    bind_group_layout: wgpu::BindGroupLayout,
    pipeline: wgpu::RenderPipeline,
}

/// Each element as `f32`, as `Float32Array` stores it.
fn matrix_f32(m: &Matrix4) -> [f32; 16] {
    m.elements.map(|e| e as f32)
}

fn f32_bytes(values: &[f32]) -> Vec<u8> {
    values.iter().flat_map(|v| v.to_le_bytes()).collect()
}

/// `Vector4.applyMatrix4(m)`.
fn apply_matrix4(v: [f64; 4], m: &Matrix4) -> [f64; 4] {
    let e = &m.elements;
    let [x, y, z, w] = v;
    [
        e[0] * x + e[4] * y + e[8] * z + e[12] * w,
        e[1] * x + e[5] * y + e[9] * z + e[13] * w,
        e[2] * x + e[6] * y + e[10] * z + e[14] * w,
        e[3] * x + e[7] * y + e[11] * z + e[15] * w,
    ]
}

impl WebGpuRenderer {
    /// `new WebGPURenderer({ device })` and `init()`.
    // div-gap: W17 navigator.gpu. three.js requests its own adapter and
    // device; Div has none to give it, so the caller passes one.
    pub fn new(device: wgpu::Device, queue: wgpu::Queue) -> WebGpuRenderer {
        // One layout serves both groups: a uniform buffer at binding 0,
        // visible to every stage (visibility 7).
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: None,
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX_FRAGMENT | wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[Some(&bind_group_layout), Some(&bind_group_layout)],
            immediate_size: 0,
        });
        let vertex = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("vertex"),
            source: wgpu::ShaderSource::Wgsl(MESH_BASIC_VERTEX.into()),
        });
        let fragment = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("fragment"),
            source: wgpu::ShaderSource::Wgsl(MESH_BASIC_FRAGMENT.into()),
        });
        // `renderPipeline_MeshBasicMaterial`: positions only, back faces
        // culled, counter-clockwise front faces, `less-equal` depth.
        let attributes = [wgpu::VertexAttribute {
            format: wgpu::VertexFormat::Float32x3,
            offset: 0,
            shader_location: 0,
        }];
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("renderPipeline_MeshBasicMaterial"),
            layout: Some(&layout),
            vertex: wgpu::VertexState {
                module: &vertex,
                entry_point: Some("main"),
                compilation_options: Default::default(),
                buffers: &[Some(wgpu::VertexBufferLayout {
                    array_stride: 12,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &attributes,
                })],
            },
            fragment: Some(wgpu::FragmentState {
                module: &fragment,
                entry_point: Some("main"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: TARGET_FORMAT,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                unclipped_depth: false,
                polygon_mode: wgpu::PolygonMode::Fill,
                conservative: false,
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: DEPTH_FORMAT,
                depth_write_enabled: Some(true),
                depth_compare: Some(wgpu::CompareFunction::LessEqual),
                stencil: Default::default(),
                bias: Default::default(),
            }),
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview_mask: None,
            cache: None,
        });
        WebGpuRenderer {
            device,
            queue,
            bind_group_layout,
            pipeline,
        }
    }

    pub fn device(&self) -> &wgpu::Device {
        &self.device
    }

    fn uniform_bind_group(&self, contents: &[u8]) -> wgpu::BindGroup {
        let buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: None,
                contents,
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            });
        self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &self.bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: buffer.as_entire_binding(),
            }],
        })
    }

    /// `setRenderTarget(target)` and `render(scene, camera)`.
    pub fn render(&self, scene: &mut Scene, camera: &mut PerspectiveCamera, target: &RenderTarget) {
        // `scene.matrixWorldAutoUpdate` and the camera's, for a camera
        // with no parent.
        scene.update_matrix_world();
        camera.update_matrix_world(false);
        // The renderer projects into WebGPU's clip space.
        if camera.coordinate_system != CoordinateSystem::WebGpu {
            camera.coordinate_system = CoordinateSystem::WebGpu;
            camera.update_projection_matrix();
        }

        // The opaque render list (StandardDrawingPipeline.project): each
        // mesh's bounding sphere center, in clip space, gives its `z`.
        let mut proj_screen = Matrix4::default();
        proj_screen.multiply_matrices(&camera.projection_matrix, &camera.matrix_world_inverse);
        let mut list: Vec<RenderItem> = scene
            .children
            .iter()
            .enumerate()
            .map(|(i, mesh)| {
                let c = mesh.geometry.bounding_sphere_center();
                let world = apply_matrix4([c.x, c.y, c.z, 1.0], &mesh.matrix_world);
                RenderItem {
                    mesh: i,
                    id: mesh.id,
                    z: apply_matrix4(world, &proj_screen)[2],
                }
            })
            .collect();
        // `painterSortStable`: equal group and render orders, then `z`,
        // then `id`.
        list.sort_by(|a, b| a.z.partial_cmp(&b.z).unwrap().then(a.id.cmp(&b.id)));

        // The `render` group: projection, then view.
        let mut render = matrix_f32(&camera.projection_matrix).to_vec();
        render.extend(matrix_f32(&camera.matrix_world_inverse));
        let render_group = self.uniform_bind_group(&f32_bytes(&render));

        // `Background`: a color background clears to that color, alpha 1.
        let clear = match scene.background {
            Some(c) => wgpu::Color {
                r: c.r,
                g: c.g,
                b: c.b,
                a: 1.0,
            },
            None => wgpu::Color {
                r: 0.0,
                g: 0.0,
                b: 0.0,
                a: 1.0,
            },
        };

        let mut draws = Vec::new();
        for item in &list {
            let mesh = &scene.children[item.mesh];
            let m = &mesh.material;
            // The `object` group: color, opacity, then the world matrix.
            let mut object = vec![
                m.color.r as f32,
                m.color.g as f32,
                m.color.b as f32,
                m.opacity as f32,
            ];
            object.extend(matrix_f32(&mesh.matrix_world));
            let group = self.uniform_bind_group(&f32_bytes(&object));
            let geometry = &mesh.geometry;
            let vertices = self
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: None,
                    contents: &f32_bytes(&geometry.position),
                    usage: wgpu::BufferUsages::VERTEX
                        | wgpu::BufferUsages::COPY_DST
                        | wgpu::BufferUsages::COPY_SRC,
                });
            let indices = self
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: None,
                    contents: &geometry.index.bytes(),
                    usage: wgpu::BufferUsages::INDEX
                        | wgpu::BufferUsages::COPY_DST
                        | wgpu::BufferUsages::COPY_SRC,
                });
            let format = match geometry.index {
                Index::U16(_) => wgpu::IndexFormat::Uint16,
                Index::U32(_) => wgpu::IndexFormat::Uint32,
            };
            draws.push((
                group,
                vertices,
                indices,
                format,
                geometry.index.len() as u32,
            ));
        }

        let color = target.color.create_view(&Default::default());
        let depth = target.depth.create_view(&Default::default());
        let mut encoder = self.device.create_command_encoder(&Default::default());
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: None,
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &color,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(clear),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &depth,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, &render_group, &[]);
            for (group, vertices, indices, format, count) in &draws {
                pass.set_bind_group(1, group, &[]);
                pass.set_index_buffer(indices.slice(..), *format);
                pass.set_vertex_buffer(0, vertices.slice(..));
                pass.draw_indexed(0..*count, 0, 0..1);
            }
        }
        self.queue.submit([encoder.finish()]);
    }

    /// `readRenderTargetPixelsAsync(target, 0, 0, width, height)`: the
    /// target's RGBA bytes, row by row from the top, without padding.
    pub fn read_render_target_pixels(&self, target: &RenderTarget) -> Vec<u8> {
        let (width, height) = (target.width, target.height);
        let unpadded = width * 4;
        let bytes_per_row = unpadded.div_ceil(256) * 256;
        let buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: u64::from(bytes_per_row * height),
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let mut encoder = self.device.create_command_encoder(&Default::default());
        encoder.copy_texture_to_buffer(
            target.color.as_image_copy(),
            wgpu::TexelCopyBufferInfo {
                buffer: &buffer,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(bytes_per_row),
                    rows_per_image: None,
                },
            },
            wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
        );
        self.queue.submit([encoder.finish()]);
        let slice = buffer.slice(..);
        slice.map_async(wgpu::MapMode::Read, |result| {
            result.expect("map the readback buffer")
        });
        self.device
            .poll(wgpu::PollType::wait_indefinitely())
            .expect("wait for the readback");
        let mapped = slice.get_mapped_range().expect("read the readback buffer");
        let mut pixels = Vec::with_capacity((unpadded * height) as usize);
        for row in mapped.chunks_exact(bytes_per_row as usize) {
            pixels.extend_from_slice(&row[..unpadded as usize]);
        }
        pixels
    }
}
