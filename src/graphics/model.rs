use std::fs::File;
use std::io::BufReader;


#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vertex {
    pub(crate) position: [f32; 3],
    pub(crate) color: [f32; 3],
    pub(crate) index: f32,
}



// TODO: move this method to a more appropriate place
pub(crate) struct Instance {
    pub(crate) position: cgmath::Vector3<f32>,
    pub(crate) rotation: cgmath::Quaternion<f32>,
}

// TODO: move this method to a more appropriate place
#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub(crate) struct InstanceRaw {
    model: [[f32;4]; 4],
}

impl Instance {
    pub(crate) fn to_raw(&self) -> InstanceRaw {
        InstanceRaw {
            model: (cgmath::Matrix4::from_translation(self.position) * cgmath::Matrix4::from(self.rotation)).into(),
        }
    }
}

impl InstanceRaw {
    pub(crate) fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
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


impl Vertex {
    pub(crate) fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        use std::mem;
        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x3,
                },
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x3,
                },
                wgpu::VertexAttribute {
                    offset: mem::size_of::<f32>() as wgpu::BufferAddress,
                    shader_location: 2,
                    format: wgpu::VertexFormat::Float32,
                },
            ],
        }
    }
}

pub struct Mesh {
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u16>,
}

impl Mesh {

    pub(crate) fn new(vertices: Vec<Vertex>, indices: Vec<u16>) -> Self {
        Self {
            vertices,
            indices,
        }
    }

    pub fn new_empty() -> Self{
        let vertices: Vec<Vertex> = Vec::new();
        let indices: Vec<u16> = Vec::new();

        Self {
            vertices,
            indices,
        }
    }
}

pub mod mesh_generation {
    use std::fs::File;
    use std::io::BufReader;
    use obj::{load_obj, Obj};
    use crate::graphics::model::{Mesh, Vertex};

    pub fn load_mesh_from_file(file_path: String) -> Mesh {
        let input = BufReader::new(File::open(file_path).unwrap());
        let dome: Obj = load_obj(input).unwrap();

        let mut vertices: Vec<Vertex> = Vec::new();
        let mut indices: Vec<u16> = Vec::new();

        let vertice_count = vertices.len();

        for (index, vertice) in dome.vertices.into_iter().enumerate() {
            let r = vertice.position[0] * (index as f32 / vertice_count as f32);
            let g = vertice.position[1] * (index as f32 / vertice_count as f32);
            let b = vertice.position[2] * (index as f32 / vertice_count as f32);
            vertices.push(Vertex {
                position: vertice.position,
                color: [r,g,b],
                index: index as f32/ vertice_count as f32,
            });
        }

        for index in dome.indices {
            indices.push(index);
        }

        Mesh {
            vertices,
            indices,
        }
    }

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


    pub fn gen_fibonacci_mesh(samples: u32) -> Mesh {

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

    pub fn gen_cube_mesh() -> Mesh {
        let indice_list = [
            //Top
            2, 6, 7,
            2, 3, 7,

            //Bottom
            0, 4, 5,
            0, 1, 5,

            //Left
            0, 2, 6,
            0, 4, 6,

            //Right
            1, 3, 7,
            1, 5, 7,

            //Front
            0, 2, 3,
            0, 1, 3,

            //Back
            4, 6, 7,
            4, 5, 7
        ];

        let vertice_position_list = [
            (-1, -1,  1), //0
            (1, -1,  1), //1
            (-1,  1,  1), //2
            (1,  1,  1), //3
            (-1, -1, -1), //4
            (1, -1, -1), //5
            (-1,  1, -1), //6
            (1,  1, -1),  //7
        ];

        let mut vertices: Vec<Vertex> = Vec::new();
        let mut indices: Vec<u16> = Vec::new();

        for index in indice_list {
            indices.push(index);
        }

        for (index, (x, y, z)) in vertice_position_list.into_iter().enumerate() {
            vertices.push(Vertex {
                position: [x as f32, y as f32, z as f32],
                color: [(x as f32 + 1.0) / 2.0, (y as f32 + 1.0) / 2.0, (z as f32 + 1.0) / 2.0],
                index: index as f32 / vertice_position_list.len() as f32,
            })
        }

        return Mesh {
            vertices,
            indices,
        }

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

    pub fn fibonacci_sphere_points(samples: u32) -> Vec<(f32, f32, f32)> {

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

}