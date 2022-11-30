use std::fs;
use cgmath::{Quaternion, Rotation3, Vector3};
use crate::graphics::model::{InstanceRaw, Mesh, Vertex};
use serde::*;
use wgpu::PrimitiveTopology;
use wgpu::util::DeviceExt;
use crate::graphics;
use crate::graphics::{avatar, texture};
use crate::graphics::avatar::{Avatar, AvatarModule};
use crate::graphics::model::Instance;

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct AvatarData {
    avatar_module_data: Vec<AvatarModuleData>,
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct AvatarModuleData {
    module_name: String,
    shader_data: ShaderData,
    mesh_generation: MeshData,
    instancing: InstanceData,
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct ShaderData {
    shader_uniform: Option<Vec<String>>,
    source_file: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct MeshData {
    mesh_gen_function: Option<MeshGenFunction>,
    mesh_render_type: Option<MeshRenderType>,
    sample: Option<usize>,
    mesh_color_function: Option<MeshColorFunction>,
    size: Option<f32>,
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct InstanceData {
    count: Option<usize>,
    position_x: Option<f32>,
    position_y: Option<f32>,
    rotation: Option<InstanceRotationFunction>,
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "ShaderUniform")]
pub enum ShaderUniforms {
    Default, Audio, Time,
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "MeshGenFunction")]
pub enum MeshGenFunction {
    Fibonacci,
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "MeshRenderType")]
pub enum MeshRenderType {
    Lines, Triangles, Points
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "MeshColorFunction")]
pub enum MeshColorFunction {
    Rainbow, Black, White,
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "InstanceRotationFunction")]
pub enum InstanceRotationFunction {
    Default, Sphere,
}

static AVATAR_DATA_PATH: &str = "avatar_settings.json";
pub fn load_avatar_data() -> Result<AvatarData, String> {
    // Load file as string
    let file = match fs::read_to_string(AVATAR_DATA_PATH) {
        Ok(t) => {t}
        Err(_) => {"Could not load file".to_string()}
    };

    println!("Settings: {}", file);

    let json : AvatarData = serde_json::from_str(&file).expect("JSON was not well-formatted");
    return Ok(json);
}

pub fn build_avatar(avatar_data: AvatarData, state: &graphics::State) -> Avatar {
    let mut avatar_modules : Vec<AvatarModule> = Vec::new();
    for avatar_module_data in avatar_data.avatar_module_data {
        println!("Starting avatar module creation of {:?}", avatar_module_data.module_name);

        let shader_data = avatar_module_data.shader_data;
        let mesh_data = avatar_module_data.mesh_generation;
        let instance_data = avatar_module_data.instancing;

        // Create mesh
        let mut mesh = match mesh_data.mesh_gen_function.unwrap_or(MeshGenFunction::Fibonacci) {
            MeshGenFunction::Fibonacci => {gen_fibonacci_mesh()}
        };
        color_mesh(mesh_data.mesh_color_function.unwrap_or(MeshColorFunction::Rainbow), &mut mesh);


        // Instances
        let instance_count = instance_data.count.unwrap_or(1);
        let instances = generate_instances
            (instance_data.rotation.unwrap_or(InstanceRotationFunction::Default), instance_count);
        let instance_data = instances.iter().map(Instance::to_raw).collect::<Vec<_>>();
        let instance_buffer = state.device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("Instance Buffer"),
                contents: bytemuck::cast_slice(&instance_data),
                usage: wgpu::BufferUsages::VERTEX,
            }
        );

        // Shader
        let shader = state.device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shader.wgsl").into()),
        });

        // Render Pipeline
        let render_pipeline_layout =
            state.device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout"),
                bind_group_layouts: &[&state.default_bind_group.default_bind_group_layout],
                push_constant_ranges: &[],
            });

        let primitive_topology = get_primitive_topology(mesh_data.mesh_render_type.unwrap_or(MeshRenderType::Lines));

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
                topology: primitive_topology,
                front_face: wgpu::FrontFace::Ccw,
                ..Default::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: crate::graphics::texture::Texture::DEPTH_FORMAT,
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

        let vertex_buffer = state.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Vertex Buffer"),
            contents: bytemuck::cast_slice(&mesh.vertices[..]),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let index_buffer = state.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Index Buffer"),
            contents: bytemuck::cast_slice(&mesh.indices[..]),
            usage: wgpu::BufferUsages::INDEX,
        });
        
        avatar_modules.push(AvatarModule {
            module_name: avatar_module_data.module_name,
            render_pipeline,
            vertex_buffer,
            index_buffer,
            instance_buffer,
            index_count: mesh.indices.len() as u16,
            mesh,
            instance_count: instance_count as u16,
        });
    }
    Avatar {
        avatar_modules,
    }
}

fn get_primitive_topology(render_type: MeshRenderType) -> PrimitiveTopology {
    match render_type {
        MeshRenderType::Lines => {PrimitiveTopology::LineList}
        MeshRenderType::Triangles => {PrimitiveTopology::TriangleList}
        MeshRenderType::Points => {PrimitiveTopology::PointList}
    }
}

fn generate_instances(instance_rotation_function: InstanceRotationFunction, index_count: usize) -> Vec<Instance> {
    let mut instances: Vec<Instance> = Vec::new();
    match instance_rotation_function {
        InstanceRotationFunction::Default => {
            instances.push(Instance {
                position: Vector3 {
                    x: 0.0,
                    y: 0.0,
                    z: 0.0,
                },
                rotation: Quaternion::from_axis_angle(Vector3::new(0.0,0.0,0.0), cgmath::Deg(45.0)),
            });
        }
        InstanceRotationFunction::Sphere => {
            let points = fibonacci_sphere_points(index_count as u32);
            let mut instances: Vec<Instance> = Vec::new();

            for (x,y,z) in points.into_iter() {
                instances.push(Instance {
                    position: cgmath::Vector3 {x , y, z},
                    rotation: cgmath::Quaternion::from_axis_angle(Vector3::new(0.0,0.0,0.0), cgmath::Deg(45.0))
                });
            }
        }
    }

    return instances;
}

fn color_mesh(color_function: MeshColorFunction, mesh: &mut Mesh) {
    match color_function {
        MeshColorFunction::Rainbow => {
            color_mesh_rainbow(mesh);
        }
        MeshColorFunction::Black => {
            color_mesh_solid_color(mesh, [0.0,0.0,0.0])
        }
        MeshColorFunction::White => {
            color_mesh_solid_color(mesh, [1.0,1.0,1.0])
        }
    }
}

fn color_mesh_rainbow(mesh: &mut Mesh) {
    let vertex_count = mesh.vertices.len();
    for (index, mut vertex) in mesh.vertices.clone().into_iter().enumerate() {
        mesh.vertices[index].color = [f32::sin(index as f32),f32::cos(index as f32),f32::sin((1.0 - index as f32 / vertex_count as f32) as f32)];
    }
}

fn color_mesh_solid_color(mesh: &mut Mesh, color: [f32; 3]) {
    for (index, mut vertex) in mesh.vertices.clone().into_iter().enumerate() {
        mesh.vertices[index].color = color.clone();
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
    let samples = 1100;

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

fn gen_triangle_mesh() -> Mesh {

    let mut vertices: Vec<Vertex> = Vec::new();
    let mut indices: Vec<u16> = Vec::new();
    let size: f32 = 0.03;

    vertices.push(Vertex {
        position: [size, size, size],
        color: [1.0,0.0,0.0],
        index: 0.9
    });
    vertices.push(Vertex {
        position: [size, 0.00, size],
        color: [0.0,1.0,0.0],
        index: 0.6
    });
    vertices.push(Vertex {
        position: [0.00, size, size],
        color: [0.0,0.0,1.0],
        index: 0.3
    });
    vertices.push(Vertex {
        position: [size, size, 0.0],
        color: [0.0,0.0,1.0],
        index: 0.3
    });
    vertices.push(Vertex {
        position: [size, 0.0, 0.0],
        color: [0.0,0.0,1.0],
        index: 0.3
    });

    indices.push(0);
    indices.push(1);
    indices.push(2);
    indices.push(0);
    indices.push(1);
    indices.push(3);
    indices.push(0);
    indices.push(2);
    indices.push(3);

    Mesh {
        vertices,
        indices,
    }
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


#[cfg(test)]
pub mod test {
    use crate::graphics::avatar_generator::{AvatarData, build_avatar, load_avatar_data};

    #[test]
    fn test_load_avatar_data() {
        match load_avatar_data() {
            Ok(t) => {

            }
            Err(e) => {
                panic!("Could not load file")
            }
        }
    }
}