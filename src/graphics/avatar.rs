use wgpu::{Buffer, RenderPipeline};
use crate::graphics::model::{Mesh, Vertex};
use crate::graphics::renderer::{RenderBatch};

pub struct Avatar {
    pub(crate) avatar_modules: Vec<AvatarModule>,
}

pub struct AvatarModule {
    pub(crate) module_name: String,
    pub(crate) visible: bool,
    pub(crate) mesh: Mesh,
    pub(crate) render_pipeline: RenderPipeline,
    pub(crate) vertex_buffer: Buffer,
    pub(crate) index_buffer: Buffer,
    pub(crate) instance_buffer: Buffer,
    pub(crate) index_count: u16,
    pub(crate) instance_count: u16,
}

impl RenderBatch for AvatarModule {
    fn get_pipeline(&self) -> Option<&RenderPipeline> {
        Some(&self.render_pipeline)
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
        self.index_count as u32
    }

    fn get_instance_buffer(&self) -> Option<&Buffer> {
        Some(&self.instance_buffer)
    }

    fn get_instance_count(&self) -> Option<u16> {
        Some(self.instance_count as u16)
    }

    fn get_visible(&self) -> bool {
        self.visible
    }
}