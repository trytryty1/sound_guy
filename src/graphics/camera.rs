use std::time::Duration;
use cgmath::{EuclideanSpace, InnerSpace, Matrix4, MetricSpace, perspective, Point3, Quaternion, Rad, Rotation, Vector2, Vector3, VectorSpace, Zero};
use cgmath::num_traits::{FloatConst, Pow};
use rand::random;
use winit::dpi::PhysicalPosition;
use winit::event::{ElementState, KeyboardInput, MouseScrollDelta, VirtualKeyCode, WindowEvent};
use crate::AUDIO_IN;
use crate::graphics::camera;

pub const OPENGL_TO_WGPU_MATRIX: cgmath::Matrix4<f32> = cgmath::Matrix4::new(
    1.0, 0.0, 0.0, 0.0,
    0.0, 1.0, 0.0, 0.0,
    0.0, 0.0, 0.5, 0.0,
    0.0, 0.0, 0.5, 1.0,
);

pub struct Camera {
    pub position: Point3<f32>,
    yaw: Rad<f32>,
    pitch: Rad<f32>,
}

impl Camera {
    pub fn new<
        V: Into<Point3<f32>>,
        Y: Into<Rad<f32>>,
        P: Into<Rad<f32>>,
    >(
        position: V,
        yaw: Y,
        pitch: P,
    ) -> Self {
        Self {
            position: position.into(),
            yaw: yaw.into(),
            pitch: pitch.into(),
        }
    }

    pub fn calc_matrix(&self) -> Matrix4<f32> {
        let (sin_pitch, cos_pitch) = self.pitch.0.sin_cos();
        let (sin_yaw, cos_yaw) = self.yaw.0.sin_cos();

        Matrix4::look_to_rh(
            self.position,
            Vector3::new(
                self.position.x * -1.0,
                self.position.y * -1.0,
                self.position.z * -1.0,
            ).normalize(),
            Vector3::unit_y(),
        )
    }
}

pub struct Projection {
    aspect: f32,
    fovy: Rad<f32>,
    znear: f32,
    zfar: f32,
}

impl Projection {
    pub fn new<F: Into<Rad<f32>>>(
        width: u32,
        height: u32,
        fovy: F,
        znear: f32,
        zfar: f32,
    ) -> Self {
        Self {
            aspect: width as f32 / height as f32,
            fovy: fovy.into(),
            znear,
            zfar,
        }
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        self.aspect = width as f32 / height as f32;
    }

    pub fn calc_matrix(&self) -> Matrix4<f32> {
        OPENGL_TO_WGPU_MATRIX * perspective(self.fovy, self.aspect, self.znear, self.zfar)
    }
}

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct CameraUniform {
    view_proj: [[f32; 4]; 4],
    view_position: [f32; 4],
}

impl CameraUniform {
    pub fn new() -> Self {
        use cgmath::SquareMatrix;
        Self {
            view_proj: cgmath::Matrix4::identity().into(),
            view_position: [0.0; 4],
        }
    }


    pub(crate) fn update_view_proj(&mut self, camera: &camera::Camera, projection: &camera::Projection) {
        self.view_position = camera.position.to_homogeneous().into();
        self.view_proj = (projection.calc_matrix() * camera.calc_matrix()).into();
    }
}

pub struct CameraController {
    camera_target: Vector3<f32>,

    radius: f32,
    total_time: f32,
    speed: f32,
    sensitivity: f32,

    camera_rotation: bool,
}

impl CameraController {
    pub fn new(speed: f32, sensitivity: f32, camera_rotation: bool) -> Self {
        Self {
            camera_target: Vector3::new(1.0, 1.0, 1.0),
            radius: 4.0,
            total_time: 0.0,
            speed,
            sensitivity,
            camera_rotation,
        }

    }

    pub fn process_keyboard(&mut self, key: VirtualKeyCode, state: ElementState) -> bool{
        let amount = if state == ElementState::Pressed { 1.0 } else { 0.0 };
        match key {
            VirtualKeyCode::W | VirtualKeyCode::Up => {
                self.radius -= self.speed;
                true
            }
            VirtualKeyCode::S | VirtualKeyCode::Down => {
                self.radius += self.speed;
                true
            }
            VirtualKeyCode::A | VirtualKeyCode::Left => {
                true
            }
            VirtualKeyCode::D | VirtualKeyCode::Right => {
                true
            }
            VirtualKeyCode::Space => {
                true
            }
            VirtualKeyCode::LShift => {
                true
            }
            _ => false,
        }
    }

    pub fn process_mouse(&mut self, mouse_dx: f64, mouse_dy: f64) {

    }

    pub fn process_scroll(&mut self, delta: &MouseScrollDelta) {
        match delta {
            MouseScrollDelta::LineDelta(x, y) => {
                self.radius += y + x;
            }
            MouseScrollDelta::PixelDelta(_) => {()}
        };
    }

    pub fn update_camera(&mut self, camera: &mut Camera, dt: Duration) {
        let dt = dt.as_secs_f32();


        camera.position = Point3::from_vec(Vector3::lerp(camera.position.to_vec(), self.camera_target, self.speed * dt));

        self.camera_target.x = f32::sin(self.total_time);
        self.camera_target.z = f32::cos(self.total_time);
        self.camera_target.y = f32::sin(self.total_time / 2.0);
        self.camera_target = self.camera_target.normalize() * self.radius;


        // Only update the time when the sphere is supposed to rotate
        if self.camera_rotation {
            self.total_time = (self.total_time + dt) % (f32::PI() * 4.0);
        } else {
            self.total_time = 3.0;
        }
    }


}
