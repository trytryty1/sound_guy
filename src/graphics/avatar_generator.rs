use std::fs;
use cgmath::{Quaternion, Rotation3, Vector3};
use rand::random;
use crate::graphics::model::{InstanceRaw, Mesh, Vertex};
use serde::*;
use wgpu::PrimitiveTopology;
use wgpu::util::DeviceExt;
use crate::graphics;
use crate::graphics::avatar::{Avatar, AvatarModule};
use crate::graphics::model::Instance;
use crate::graphics::model::mesh_generation::*;

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct AvatarData {
    avatar_module_data: Vec<AvatarModuleData>,
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct AvatarModuleData {
    module_name: String,
    visible: bool,
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
    position_z: Option<f32>,
    scale: Option<f32>,
    instance_rotation_function: Option<InstanceRotationFunction>,
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "ShaderUniform")]
pub enum ShaderUniforms {
    Default, Audio, Time,
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "MeshGenFunction")]
pub enum MeshGenFunction {
    Fibonacci, Cube, Loaded {file: String},
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

#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "InstanceRotationFunction")]
pub enum InstanceRotationFunction {
    Default, Sphere {radius: f32},
}

static AVATAR_DATA_PATH: &str = "resources/avatar_settings.json";
pub fn load_avatar_data() -> Result<AvatarData, String> {
    // Load file as string
    let file = match fs::read_to_string(AVATAR_DATA_PATH) {
        Ok(t) => {t}
        Err(_) => {"Could not load file".to_string()}
    };

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
            MeshGenFunction::Fibonacci => {gen_fibonacci_mesh(mesh_data.sample.unwrap_or(25) as u32)},
            MeshGenFunction::Cube => {gen_cube_mesh()},
            MeshGenFunction::Loaded {file} => {load_mesh_from_file(file)}
        };
        color_mesh(mesh_data.mesh_color_function.unwrap_or(MeshColorFunction::Rainbow), &mut mesh);


        // Instances
        let instance_count = instance_data.count.unwrap_or(1);
        let instances = generate_instances
            (instance_data.instance_rotation_function.unwrap_or(InstanceRotationFunction::Default),
             instance_count,
            instance_data.position_x.unwrap_or(0.0),
            instance_data.position_y.unwrap_or(0.0),
            instance_data.position_z.unwrap_or(0.0),
            instance_data.scale.unwrap_or(1.0));
        let instance_data = instances.iter().map(Instance::to_raw).collect::<Vec<_>>();
        let instance_buffer = state.device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("Instance Buffer"),
                contents: bytemuck::cast_slice(&instance_data),
                usage: wgpu::BufferUsages::VERTEX,
            }
        );


        // Load file source
        let shader_source = match fs::read_to_string(shader_data.source_file.unwrap_or("shader.wgsl".to_string())) {
            Ok(t) => {t}
            Err(_) => {"Could not load file".to_string()}
        };

        // Shader
        let shader = state.device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Shader"),
            source: wgpu::ShaderSource::Wgsl(shader_source.into()),
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
            visible: avatar_module_data.visible,
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

fn generate_instances(instance_rotation_function: InstanceRotationFunction, index_count: usize, position_x: f32, position_y: f32, position_z: f32, scale: f32) -> Vec<Instance> {
    let mut instances: Vec<Instance> = Vec::new();
    println!("Scale: {}", scale);
    match instance_rotation_function {
        InstanceRotationFunction::Default => {
            instances.push(Instance {
                position: Vector3 {
                    x: 0.0,
                    y: 0.0,
                    z: 0.0,
                },
                rotation: Quaternion::from_axis_angle(Vector3::new(0.0,0.0,0.0), cgmath::Deg(45.0)),
                scale,
            });
        }
        InstanceRotationFunction::Sphere {radius: sphere_scale} => {
            let points = fibonacci_sphere_points(index_count as u32);

            for (x,y,z) in points.into_iter() {
                let pos_x = x * sphere_scale + position_x;
                let pos_y = y * sphere_scale + position_y;
                let pos_z = z * sphere_scale + position_z;
                instances.push(Instance {
                    position: Vector3 {x:pos_x , y:pos_y, z:pos_z},
                    rotation: Quaternion::from_axis_angle(Vector3::new(0.0,1.0,0.0), cgmath::Deg(random::<f32>() * 360.0)),
                    scale,
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
    mesh.vertices.len();
    for (index, mut vertex) in mesh.vertices.clone().into_iter().enumerate() {
        let r = (vertex.position[0] + 1.0) / 2.0;
        let g = (vertex.position[1] + 1.0) / 2.0;
        let b = (vertex.position[2] + 1.0) / 2.0;
        mesh.vertices[index].color = [r, g, b];
    }
    // Set the center color to black
    mesh.vertices[0].color = [1.0,0.0,1.0];
}

fn color_mesh_solid_color(mesh: &mut Mesh, color: [f32; 3]) {
    for (index, mut vertex) in mesh.vertices.clone().into_iter().enumerate() {
        mesh.vertices[index].color = color.clone();
    }
}


// #######################################
// ####### Mesh generation ###############

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