use std::mem;
use wgpu::{BindGroup, Buffer, Queue, RenderPass, RenderPipeline};
use wgpu::util::DeviceExt;
use crate::AUDIO_IN;
use crate::graphics::model::{Mesh, Vertex};
use crate::graphics::renderer::{RenderBatch};
use crate::graphics::{State, texture};
use cgmath::prelude::*;
use cgmath::{Quaternion, Vector3};

pub(crate) struct Avatar {
    mesh: Mesh,
    render_pipeline: RenderPipeline,
    vertex_buffer: Buffer,
    index_buffer: Buffer,
    audio_bind_group: BindGroup,
    audio_in_buffer: Buffer,
}

pub(crate) struct AvatarOuter {
    mesh: Mesh,
    render_pipeline: RenderPipeline,
    vertex_buffer: Buffer,
    index_buffer: Buffer,
    audio_bind_group: BindGroup,
    audio_in_buffer: Buffer,
    instance_buffer: Buffer,
    index_count: u16,
}

// TODO: move this method to a more appropriate place
struct Instance {
    position: cgmath::Vector3<f32>,
    rotation: cgmath::Quaternion<f32>,
}

// TODO: move this method to a more appropriate place
#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct InstanceRaw {
    model: [[f32;4]; 4],
}

impl Instance {
    fn to_raw(&self) -> InstanceRaw {
        InstanceRaw {
            model: (cgmath::Matrix4::from_translation(self.position) * cgmath::Matrix4::from(self.rotation)).into(),
        }
    }
}

impl InstanceRaw {
    fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        use std::mem;
        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<InstanceRaw>() as wgpu::BufferAddress,
            // We need to switch from using a step mode of Vertex to Instance
            // This means that our shaders will only change to use the next
            // instance when the shader starts processing a new instance
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    // While our vertex shader only uses locations 0, and 1 now, in later tutorials we'll
                    // be using 2, 3, and 4, for Vertex. We'll start at slot 5 not conflict with them later
                    shader_location: 5,
                    format: wgpu::VertexFormat::Float32x4,
                },
                // A mat4 takes up 4 vertex slots as it is technically 4 vec4s. We need to define a slot
                // for each vec4. We'll have to reassemble the mat4 in
                // the shader.
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 4]>() as wgpu::BufferAddress,
                    shader_location: 6,
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 8]>() as wgpu::BufferAddress,
                    shader_location: 7,
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 12]>() as wgpu::BufferAddress,
                    shader_location: 8,
                    format: wgpu::VertexFormat::Float32x4,
                },
            ],
        }
    }
}

impl AvatarOuter {
    pub fn build_avatar_outer(state: &State) -> AvatarOuter {

        let index_count: u16 = 5000;

        let audio_bind_group_layout =
            state.device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
                label: Some("audio_in_bind_group_layout"),
            });

        let audio_buffer = state.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("audio_in"),
            contents: &[0,0,0,0],
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let audio_bind_group = state.device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &audio_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: audio_buffer.as_entire_binding(),
            }],
            label: Some("audio_in_bind_group"),
        });


        // ############################################
        // ############# Instancing ###################
        let mesh = fibonacci_sphere_points(index_count.into());
        let mut instances: Vec<Instance> = Vec::new();
        for (x,y,z) in mesh.into_iter() {
            instances.push(Instance {
                position: cgmath::Vector3 {x , y, z},
                rotation: cgmath::Quaternion::from_axis_angle(Vector3::new(0.0,0.0,0.0), cgmath::Deg(45.0))
            });
        }

        // Create the instance buffer
        let instance_data = instances.iter().map(Instance::to_raw).collect::<Vec<_>>();
        let instance_buffer = state.device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("Instance Buffer"),
                contents: bytemuck::cast_slice(&instance_data),
                usage: wgpu::BufferUsages::VERTEX,
            }
        );


        let shader = state.device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shader_avatar_outer.wgsl").into()),
        });

        let render_pipeline_layout =
            state.device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout"),
                bind_group_layouts: &[&state.default_bind_group.default_bind_group_layout, &audio_bind_group_layout],
                push_constant_ranges: &[],
            });

        let render_pipeline = state.device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[Vertex::desc(), InstanceRaw::desc()],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format: state.config.format,
                    blend: Some(wgpu::BlendState {
                        color: wgpu::BlendComponent::REPLACE,
                        alpha: wgpu::BlendComponent::REPLACE,
                    }),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                front_face: wgpu::FrontFace::Ccw,
                ..Default::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: texture::Texture::DEPTH_FORMAT,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Less, // 1.
                stencil: wgpu::StencilState::default(), // 2.
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            // If the pipeline will be used with a multiview render pass, this
            // indicates how many array layers the attachments will have.
            multiview: None,
        });

        let model = gen_fibonacci_mesh();

        let vertex_buffer = state.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Vertex Buffer"),
            contents: bytemuck::cast_slice(&model.vertices[..]),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let index_buffer = state.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Index Buffer"),
            contents: bytemuck::cast_slice(&model.indices[..]),
            usage: wgpu::BufferUsages::INDEX,
        });

        AvatarOuter {
            mesh: model,
            render_pipeline,
            vertex_buffer,
            index_buffer,
            audio_bind_group,
            audio_in_buffer: audio_buffer,
            instance_buffer,
            index_count,
        }
    }
}

impl Avatar {
    pub fn build_avatar(state: &State) -> Avatar {

        let audio_bind_group_layout =
            state.device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
                label: Some("audio_in_bind_group_layout"),
            });

        let audio_buffer = state.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("audio_in"),
            contents: &[0,0,0,0],
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let audio_bind_group = state.device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &audio_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: audio_buffer.as_entire_binding(),
            }],
            label: Some("audio_in_bind_group"),
        });

        let shader = state.device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shader.wgsl").into()),
        });

        let render_pipeline_layout =
            state.device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout"),
                bind_group_layouts: &[&state.default_bind_group.default_bind_group_layout, &audio_bind_group_layout],
                push_constant_ranges: &[],
            });

        let render_pipeline = state.device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[Vertex::desc()],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format: state.config.format,
                    blend: Some(wgpu::BlendState {
                        color: wgpu::BlendComponent::REPLACE,
                        alpha: wgpu::BlendComponent::OVER,
                    }),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::LineList,
                front_face: wgpu::FrontFace::Ccw,
                ..Default::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: texture::Texture::DEPTH_FORMAT,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Less, // 1.
                stencil: wgpu::StencilState::default(), // 2.
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            // If the pipeline will be used with a multiview render pass, this
            // indicates how many array layers the attachments will have.
            multiview: None,
        });

        let model = gen_fibonacci_mesh();

        let vertex_buffer = state.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Vertex Buffer"),
            contents: bytemuck::cast_slice(&model.vertices[..]),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let index_buffer = state.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Index Buffer"),
            contents: bytemuck::cast_slice(&model.indices[..]),
            usage: wgpu::BufferUsages::INDEX,
        });

        Avatar {
            mesh: model,
            render_pipeline,
            vertex_buffer,
            index_buffer,
            audio_bind_group,
            audio_in_buffer: audio_buffer,
        }
    }

}

impl RenderBatch for AvatarOuter {

    fn get_pipeline(&self) -> Option<&RenderPipeline> {
        Some(&self.render_pipeline)
    }

    fn bind_group<'a, 'b>(&'a self, render_pass: &'b mut RenderPass<'a>, default_bind_group: &'a BindGroup) where 'a: 'b {
        render_pass.set_bind_group(0, &default_bind_group, &[]);
        render_pass.set_bind_group(1, &self.audio_bind_group, &[]);
    }

    fn get_vertex_buffer(&self) -> &Buffer {
        &self.vertex_buffer
    }

    fn get_index_buffer(&self) -> &Buffer {
        &self.index_buffer
    }

    fn get_vertices(&self) -> &[Vertex] {
        &self.mesh.vertices[..]
    }

    fn get_indices(&self) -> &[u16] {
        &self.mesh.indices[..]
    }

    fn get_indices_count(&self) -> u32 {
        self.mesh.indices.len() as u32
    }

    fn write_buffer(&mut self, queue: &mut Queue) {
        unsafe {
            queue.write_buffer(
                &self.audio_in_buffer,
                0,
                &(AUDIO_IN).to_ne_bytes(),
            );
        }
    }

    fn get_instance_buffer(&self) -> Option<&Buffer> {
        Some(&self.instance_buffer)
    }

    fn get_instance_count(&self) -> Option<u16> {
        Some(self.index_count)
    }
}

impl RenderBatch for Avatar {

    fn get_pipeline(&self) -> Option<&RenderPipeline> {
        Some(&self.render_pipeline)
    }

    fn bind_group<'a, 'b>(&'a self, render_pass: &'b mut RenderPass<'a>, default_bind_group: &'a BindGroup) where 'a: 'b {
        render_pass.set_bind_group(0, &default_bind_group, &[]);
        render_pass.set_bind_group(1, &self.audio_bind_group, &[]);
    }

    fn get_vertex_buffer(&self) -> &Buffer {
        &self.vertex_buffer
    }

    fn get_index_buffer(&self) -> &Buffer {
        &self.index_buffer
    }

    fn get_vertices(&self) -> &[Vertex] {
        &self.mesh.vertices[..]
    }

    fn get_indices(&self) -> &[u16] {
        &self.mesh.indices[..]
    }

    fn get_indices_count(&self) -> u32 {
        self.mesh.indices.len() as u32
    }

    fn write_buffer(&mut self, queue: &mut Queue) {
        unsafe {
            queue.write_buffer(
                &self.audio_in_buffer,
                0,
                &(AUDIO_IN).to_ne_bytes(),
            );
        }
    }

    fn get_instance_buffer(&self) -> Option<&Buffer> {
        None
    }

    fn get_instance_count(&self) -> Option<u16> {
        None
    }
}

// #######################################
// ####### Mesh generation ###############
pub fn gen_outer_mesh() -> Mesh {
    let samples = 50;

    let points = fibonacci_sphere_points(samples);

    let mut vertices: Vec<Vertex> = Vec::new();
    let mut indices: Vec<u16> = Vec::new();

    for (index, (x, y , z)) in points.into_iter().enumerate() {
        let r:f32 = (x + 1.0)/2.0;
        let g:f32 = (y + 1.0)/2.0;
        let b:f32 = (z + 1.0)/2.0;

        vertices.push(Vertex {position: [x*1.5, y*1.5, z*1.5],
            color:[r,g,b],
            index: if index % 11 == 0 {1.0} else {0.0}});

        indices.push(0);
        indices.push(index as u16);

    }

    Mesh::new(vertices, indices)

}


pub fn gen_fibonacci_mesh() -> Mesh { //-> &[Vertex] {
    let samples = 250;

    let points = fibonacci_sphere_points(samples);

    let mut vertices: Vec<Vertex> = Vec::new();
    let mut indices: Vec<u16> = Vec::new();

    // Add the center vertices
    vertices.push(Vertex {position:[0.0,0.0,0.0], color:[0.0,0.0,0.0], index:0f32});

    for (index, (x, y , z)) in points.into_iter().enumerate() {
        let r:f32 = (x + 1.0)/2.0;
        let g:f32 = (y + 1.0)/2.0;
        let b:f32 = (z + 1.0)/2.0;

        vertices.push(Vertex {position: [x, y, z],
            color:[r,g,b],
            index: if index % 11 == 0 {1.0} else {0.0}});

        indices.push(0);
        indices.push(index as u16);

    }

    Mesh::new(vertices, indices)
}

fn triangle_sphere() {

}

fn fibonacci_sphere_points(samples: u32) -> Vec<(f32, f32, f32)> {

    let mut points: Vec<(f32, f32, f32)> = Vec::new();
    let phi = std::f32::consts::PI * (3.0 - f32::sqrt(5.0));

    for i in 0..samples {
        let y = 1.0 - (i as f32 / ((samples as f32 - 1.0) as f32)) * 2.0;
        let radius = f32::sqrt(1.0 - y * y);

        let theta = phi * i as f32;

        let x = f32::cos(theta) * radius;
        let z = f32::sin(theta) * radius;

        points.push((x, y, z));
    }

    return points;
}