use wgpu::BindGroupLayout;

use wgpu::util::DeviceExt;
use winit::{
    event::*,
    event_loop::{ControlFlow, EventLoop},
    window::{Window, WindowBuilder},
};
use winit::dpi::{LogicalSize};
use winit::platform::windows::{WindowBuilderExtWindows};
use winit::window::{Icon};

mod avatar;
mod model;
mod camera;
mod renderer;
mod texture;
mod avatar_generator;

#[cfg(target_arch="wasm32")]
use wasm_bindgen::prelude::*;
use crate::graphics::camera::{Camera, CameraController, CameraUniform};
use crate::graphics::renderer::Renderer;
use crate::graphics::texture::Texture;
use crate::{AUDIO_IN, graphics, Settings};
use crate::graphics::avatar::{Avatar, AvatarModule};


const BACKGROUND_COLOR: [f64; 4] = [0.0,0.0,0.0,0.0];

struct DefaultBindGroups {
    camera_buffer: wgpu::Buffer,
    time_buffer: wgpu::Buffer,
    audio_buffer: wgpu::Buffer,
    keyboard_speed_buffer: wgpu::Buffer,

    default_bind_group_layout: BindGroupLayout,
    default_bindings: wgpu::BindGroup,
}

#[rustfmt::skip]
pub struct State {
    surface: wgpu::Surface,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    size: winit::dpi::PhysicalSize<u32>,

    // Camera stuff
    camera: Camera,
    camera_controller: CameraController,
    camera_uniform: CameraUniform,

    // time
    time: f32,

    default_bind_group: DefaultBindGroups,
    depth_texture: graphics::texture::Texture,
}

impl State {
    async fn new(window: &Window) -> Self {

        let size = window.inner_size();


        // The instance is a handle to our GPU
        // BackendBit::PRIMARY => Vulkan + Metal + DX12 + Browser WebGPU
        let instance = wgpu::Instance::new(wgpu::Backends::all());
        let surface = unsafe { instance.create_surface(window) };
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .unwrap();

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: None,
                    features: wgpu::Features::empty(),
                    // WebGL doesn't support all of wgpu's features, so if
                    // we're building for the web we'll have to disable some.
                    limits: if cfg!(target_arch = "wasm32") {
                        wgpu::Limits::downlevel_webgl2_defaults()
                    } else {
                        wgpu::Limits::default()
                    },
                },
                None, // Trace path
            )
            .await
            .unwrap();

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface.get_supported_formats(&adapter)[0],
            width: size.width,
            height: size.height,
            present_mode: wgpu::PresentMode::Fifo,
            alpha_mode: wgpu::CompositeAlphaMode::Auto,
        };
        surface.configure(&device, &config);

        let depth_texture = texture::Texture::create_depth_texture(&device, &config, "depth_texture");

        let camera = Camera {
            eye: (0.0, 1.0, 2.0).into(),
            target: (0.0, 0.0, 0.0).into(),
            up: cgmath::Vector3::unit_y(),
            aspect: config.width as f32 / config.height as f32,
            fovy: 45.0,
            znear: 0.1,
            zfar: 100.0,
        };
        let camera_controller = CameraController::new(0.2);

        let mut camera_uniform = CameraUniform::new();
        camera_uniform.update_view_proj(&camera);


        // #########################################################
        // ################ Default Uniforms #######################

        // Camera uniform
        let camera_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Camera Buffer"),
            contents: bytemuck::cast_slice(&[camera_uniform]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        // Time uniform
        let time_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Time Buffer"),
            contents: &[0,0,0,0],
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let audio_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Audio Buffer"),
            contents: &[0,0,0,0],
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let keyboard_speed_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Keyboard Speed Buffer"),
            contents: &[0,0,0,0],
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        // Creating the bind group layout
        let default_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries:
                &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },],
                label: Some("camera_bind_group_layout"),
            });

        let default_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &default_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: camera_buffer.as_entire_binding(),
            }, wgpu::BindGroupEntry {
                binding: 1,
                resource: time_buffer.as_entire_binding(),
            }, wgpu::BindGroupEntry {
                binding: 2,
                resource: audio_buffer.as_entire_binding(),
            }, wgpu::BindGroupEntry {
                binding: 3,
                resource: keyboard_speed_buffer.as_entire_binding(),
            },],
            label: Some("default_bind_group"),
        });

        let default_bind_group_struct = DefaultBindGroups {
            camera_buffer,
            default_bindings: default_bind_group,
            time_buffer,
            audio_buffer,
            keyboard_speed_buffer,
            default_bind_group_layout,
        };

        Self {
            surface,
            device,
            queue,
            config,
            size,


            camera,
            camera_controller,
            camera_uniform,

            time: 0.0,
            default_bind_group: default_bind_group_struct,

            depth_texture,
        }
    }

    fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {


        if new_size.width > 0 && new_size.height > 0 {
            self.size = new_size;
            self.config.width = new_size.width;
            self.config.height = new_size.height;
            self.surface.configure(&self.device, &self.config);

            self.camera.aspect = self.config.width as f32 / self.config.height as f32;

            self.depth_texture = texture::Texture::create_depth_texture(&self.device, &self.config, "depth_texture");
        }
    }

    fn input(&mut self, event: &WindowEvent) -> bool {
        self.camera_controller.process_events(event)
    }

    unsafe fn update(&mut self) {
        // Update time
        self.time += 0.05;

        self.camera_controller.update_camera(&mut self.camera);
        self.camera_uniform.update_view_proj(&self.camera);
        self.queue.write_buffer(
            &self.default_bind_group.camera_buffer,
            0,
            bytemuck::cast_slice(&[self.camera_uniform]),
        );
        self.queue.write_buffer(
            &self.default_bind_group.time_buffer,
            0,
            &self.time.to_ne_bytes(),
        );
        self.queue.write_buffer(
            &self.default_bind_group.audio_buffer,
            0,
            &AUDIO_IN.to_ne_bytes(),
        );
    }
}

#[cfg_attr(target_arch="wasm32", wasm_bindgen(start))]
pub async fn run(settings: &Settings) {
    cfg_if::cfg_if! {
        if #[cfg(target_arch = "wasm32")] {
            std::panic::set_hook(Box::new(console_error_panic_hook::hook));
            console_log::init_with_level(log::Level::Warn).expect("Could't initialize logger");
        } else {
            env_logger::init();
        }
    }

    let event_loop = EventLoop::new();

    let mut window = WindowBuilder::new()
        .with_decorations(false)
        .with_transparent(settings.transparent_background)
        .with_always_on_top(settings.always_on_top)
        .with_inner_size(LogicalSize::new(settings.default_width as f32, settings.default_height as f32))
        .with_title(&settings.title)
        .with_taskbar_icon(Some(load_icon()))
        .with_window_icon(Some(load_icon()))
        .build(&event_loop)
        .unwrap();
    
    window.set_cursor_hittest(false).expect("TODO: panic message");

    #[cfg(target_arch = "wasm32")]
    {
        // Winit prevents sizing with CSS, so we have to set
        // the size manually when on web.
        use winit::dpi::PhysicalSize;
        window.set_inner_size(PhysicalSize::new(450, 400));

        use winit::platform::web::WindowExtWebSys;
        web_sys::window()
            .and_then(|win| win.document())
            .and_then(|doc| {
                let dst = doc.get_element_by_id("wasm-example")?;
                let canvas = web_sys::Element::from(window.canvas());
                dst.append_child(&canvas).ok()?;
                Some(())
            })
            .expect("Couldn't append canvas to document body.");
    }

    // State::new uses async code, so we're going to wait for it to finish
    let mut state = State::new(&window).await;
    let mut renderer = Renderer::new();

    let avatar: crate::graphics::avatar::Avatar = avatar_generator::build_avatar(avatar_generator::load_avatar_data().unwrap(), &state);
    for avatar_module in avatar.avatar_modules.into_iter() {
        renderer.add_render_batch(Box::new(avatar_module));
    }

    event_loop.run(move |event, _, control_flow| {
        match event {
            Event::WindowEvent {
                ref event,
                window_id,
            } if window_id == window.id() => {
                window_events(&mut window, event);
                if !state.input(event) {

                    match event {
                        WindowEvent::CloseRequested => *control_flow = ControlFlow::Exit,
                        WindowEvent::Resized(physical_size) => {
                            state.resize(*physical_size);
                        }
                        WindowEvent::ScaleFactorChanged { new_inner_size, .. } => {
                            // new_inner_size is &mut so w have to dereference it twice
                            state.resize(**new_inner_size);
                        }
                        _ => {}
                    }
                }
            }
            Event::RedrawRequested(window_id) if window_id == window.id() => unsafe {
                state.update();
                match renderer.render(&state) {
                    Ok(_) => {}
                    // Reconfigure the surface if it's lost or outdated
                    Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => state.resize(state.size),
                    // The system is out of memory, we should probably quit
                    Err(wgpu::SurfaceError::OutOfMemory) => *control_flow = ControlFlow::Exit,
                    // We're ignoring timeouts
                    Err(wgpu::SurfaceError::Timeout) => log::warn!("Surface timeout"),
                }
            }
            Event::MainEventsCleared => {
                // RedrawRequested will only trigger once, unless we manually
                // request it.
                window.request_redraw();
            }
            Event::DeviceEvent {
                event,
                ..
            } => {
                device_events(&mut window, &event);
            }
            _ => {}
        }
    });
}

static mut TAKE_FOCUS: bool = true;

fn device_events(window: &mut Window, event: &DeviceEvent) {
    match event {
        DeviceEvent::Added => {}
        DeviceEvent::Removed => {}
        DeviceEvent::MouseMotion { .. } => {}
        DeviceEvent::MouseWheel { .. } => {}
        DeviceEvent::Motion { .. } => {}
        DeviceEvent::Button { .. } => {}
        DeviceEvent::Key(input) => {
            let is_pressed = input.state == ElementState::Pressed;
            match input.virtual_keycode.unwrap() {
                VirtualKeyCode::RShift => unsafe {
                    if is_pressed {
                        window.set_cursor_hittest(TAKE_FOCUS).expect("TODO: panic message");
                        window.set_decorations(TAKE_FOCUS);
                        TAKE_FOCUS = !TAKE_FOCUS;
                    }
                }
                _ => {}
            }
        }
        DeviceEvent::Text { .. } => {}
    }
}

fn window_events(window: &mut Window, event: &WindowEvent) {
    match event {
        WindowEvent::KeyboardInput {
            input:
            KeyboardInput {
                state,
                virtual_keycode: Some(keycode),
                ..
            },
            ..
        } => {
            // let is_pressed = *state == ElementState::Pressed;
            match keycode {
                VirtualKeyCode::LShift => {

                }

                _ => {}
            }
        },
        WindowEvent::MouseInput {
            ..
        } => {
            window.drag_window().expect("TODO: panic message");
        }
        _ => {}
    };
}

fn load_icon() -> Icon {
    let (icon_rgba, icon_width, icon_height) = {
        let image = image::open("C:\\Users\\wing_\\IdeaProjects\\sound_guy\\src\\wall.png")
            .expect("Failed to open icon path")
            .into_rgba8();
        let (width, height) = image.dimensions();
        let rgba = image.into_raw();
        (rgba, width, height)
    };

    Icon::from_rgba(icon_rgba, icon_width, icon_height).unwrap()
}