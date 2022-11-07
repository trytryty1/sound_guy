use crate::graphics::{Mesh, Vertex};

struct Avatar {
    radius: u32,
    detail: u32,
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

impl Avatar {



}