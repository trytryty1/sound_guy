use wgpu::{BindGroup, Buffer, Queue, RenderPipeline};
use crate::graphics::avatar::AvatarModule;
use crate::graphics::model::{Vertex};
use crate::graphics::State;

pub(crate) struct Renderer {
    render_batches: Vec<Box<dyn RenderBatch>>,
}

pub(crate) trait RenderBatch {
    fn get_pipeline(&self) -> Option<&RenderPipeline>;
    fn get_vertex_buffer(&self) -> &Buffer;
    fn get_index_buffer(&self) -> &Buffer;
    fn get_vertices(&self) -> &[Vertex];
    fn get_indices(&self) -> &[u16];
    fn get_indices_count(&self) -> u32;
    fn get_instance_buffer(&self) -> Option<&Buffer>;
    fn get_instance_count(&self) -> Option<u16>;
    fn get_visible(&self) -> bool;
}

const BACKGROUND_COLOR: [f64; 4] = [0.0,0.0,0.0,0.0];

// TODO: need to update render batches when the screen gets resized
impl Renderer {
    pub fn new() -> Self {
        let render_batches = Vec::new();
        Self {
            render_batches
        }
    }

    pub fn add_render_batch(&mut self, render_batch: Box<AvatarModule>) {
        self.render_batches.push(render_batch);
    }

    pub fn render(&mut self, state: &State) -> Result<(), wgpu::SurfaceError> {
        let device = &state.device;
        let surface = &state.surface;
        let queue = &state.queue;

        let output = surface.get_current_texture().unwrap();
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
        });
        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: None,
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: BACKGROUND_COLOR[0],
                            g: BACKGROUND_COLOR[1],
                            b: BACKGROUND_COLOR[2],
                            a: BACKGROUND_COLOR[3],
                        }),
                        store: true,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &state.depth_texture.view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: true,
                    }),
                    stencil_ops: None,
                }),
            });

            // Draw all of the render batches in the renderer
            for render_batch in self.render_batches.iter() {
                // Skip the rendering if the current render batch is not visible
                if !render_batch.get_visible() {
                    continue;
                }
                let pipeline = render_batch.get_pipeline().unwrap();
                let vertex_buffer = render_batch.get_vertex_buffer();
                let index_buffer = render_batch.get_index_buffer();

                // Pass in all of the bind groups
                render_pass.set_pipeline(pipeline);
                render_pass.set_bind_group(0, &state.default_bind_group.default_bindings, &[]);
                render_pass.set_vertex_buffer(0, vertex_buffer.slice(..));
                match render_batch.get_instance_buffer() {
                    None => {}
                    Some(buffer) => {
                        render_pass.set_vertex_buffer(1, buffer.slice(..));
                    }
                }
                render_pass.set_index_buffer(index_buffer.slice(..), wgpu::IndexFormat::Uint16);
                render_pass.draw_indexed(0..render_batch.get_indices_count(), 0, 0..match render_batch.get_instance_count() {
                    None => {1}
                    Some(t) => {t as u32}
                });
            }
        }
        // Output to the screen
        queue.submit(std::iter::once(encoder.finish()));
        output.present();


        Ok(())
    }
}