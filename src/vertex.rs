use crate::{imports::*, renderer::VertexConvert};

#[derive(Debug, Copy, Clone)]
#[repr(C)]
pub struct UIVertex {
    pub pos: [f32; 2],
    pub uv: [f32; 2],
    pub color: [u8; 4],
}

#[derive(Debug, Copy, Clone)]
#[repr(C)]
pub struct PointVertex2D {
    pub pos: [f32; 2],
    pub size: f32,
    pub color: [u8; 4],
}

#[derive(Debug, Copy, Clone)]
#[repr(C)]
pub struct PointVertex3D {
    pub pos: [f32; 3],
    pub size: f32,
    pub color: [u8; 4],
}

#[derive(Debug, Copy, Clone)]
#[repr(C)]
pub struct Vertex2D {
    pub pos: [f32; 2],
    pub uv: [f32; 2],
    pub color: [u8; 4],
}

#[derive(Debug, Copy, Clone)]
#[repr(C)]
pub struct Vertex3D {
    pub pos: [f32; 3],
    pub uv: [f32; 2],
    pub color: [u8; 4],
}

impl VertexConvert for UIVertex {
    fn convert_to_vertex_input_binding_desc() -> Vec<VertexInputBindingDescription> {
        vec![VertexInputBindingDescription::builder()
            .binding(0)
            .stride(size_of::<UIVertex>() as u32)
            .input_rate(VertexInputRate::VERTEX)
            .build()]
    }

    fn convert_to_vertex_input_attrib_desc() -> Vec<VertexInputAttributeDescription> {
        vec![
            VertexInputAttributeDescription::builder()
                .binding(0)
                .location(0)
                .format(Format::R32G32_SFLOAT)
                .offset(0)
                .build(),
            VertexInputAttributeDescription::builder()
                .binding(0)
                .location(1)
                .format(Format::R32G32_SFLOAT)
                .offset(8)
                .build(),
            VertexInputAttributeDescription::builder()
                .binding(0)
                .location(2)
                .format(Format::R8G8B8A8_UNORM)
                .offset(16)
                .build(),
        ]
    }
}

impl VertexConvert for PointVertex2D {
    fn convert_to_vertex_input_binding_desc() -> Vec<VertexInputBindingDescription> {
        vec![VertexInputBindingDescription::builder()
            .binding(0)
            .stride(size_of::<PointVertex2D>() as u32)
            .input_rate(VertexInputRate::VERTEX)
            .build()]
    }

    fn convert_to_vertex_input_attrib_desc() -> Vec<VertexInputAttributeDescription> {
        vec![
            VertexInputAttributeDescription::builder()
                .binding(0)
                .location(0)
                .format(Format::R32G32B32A32_SFLOAT)
                .offset(0)
                .build(),
            VertexInputAttributeDescription::builder()
                .binding(0)
                .location(1)
                .format(Format::R8G8B8A8_UNORM)
                .offset(16)
                .build(),
        ]
    }
}

impl VertexConvert for PointVertex3D {
    fn convert_to_vertex_input_binding_desc() -> Vec<VertexInputBindingDescription> {
        vec![VertexInputBindingDescription::builder()
            .binding(0)
            .stride(size_of::<PointVertex3D>() as u32)
            .input_rate(VertexInputRate::VERTEX)
            .build()]
    }

    fn convert_to_vertex_input_attrib_desc() -> Vec<VertexInputAttributeDescription> {
        vec![
            VertexInputAttributeDescription::builder()
                .binding(0)
                .location(0)
                .format(Format::R32G32B32A32_SFLOAT)
                .offset(0)
                .build(),
            VertexInputAttributeDescription::builder()
                .binding(0)
                .location(1)
                .format(Format::R8G8B8A8_UNORM)
                .offset(16)
                .build(),
        ]
    }
}

impl VertexConvert for Vertex2D {
    fn convert_to_vertex_input_binding_desc() -> Vec<VertexInputBindingDescription> {
        vec![VertexInputBindingDescription::builder()
            .binding(0)
            .stride(size_of::<Vertex2D>() as u32)
            .input_rate(VertexInputRate::VERTEX)
            .build()]
    }

    fn convert_to_vertex_input_attrib_desc() -> Vec<VertexInputAttributeDescription> {
        vec![
            VertexInputAttributeDescription::builder()
                .binding(0)
                .location(0)
                .format(Format::R32G32_SFLOAT)
                .offset(0)
                .build(),
            VertexInputAttributeDescription::builder()
                .binding(0)
                .location(1)
                .format(Format::R32G32_SFLOAT)
                .offset(8)
                .build(),
            VertexInputAttributeDescription::builder()
                .binding(0)
                .location(2)
                .format(Format::R8G8B8A8_UNORM)
                .offset(16)
                .build(),
        ]
    }
}

impl VertexConvert for Vertex3D {
    fn convert_to_vertex_input_binding_desc() -> Vec<VertexInputBindingDescription> {
        vec![VertexInputBindingDescription::builder()
            .binding(0)
            .stride(size_of::<Vertex3D>() as u32)
            .input_rate(VertexInputRate::VERTEX)
            .build()]
    }

    fn convert_to_vertex_input_attrib_desc() -> Vec<VertexInputAttributeDescription> {
        vec![
            VertexInputAttributeDescription::builder()
                .binding(0)
                .location(0)
                .format(Format::R32G32B32_SFLOAT)
                .offset(0)
                .build(),
            VertexInputAttributeDescription::builder()
                .binding(0)
                .location(1)
                .format(Format::R32G32_SFLOAT)
                .offset(12)
                .build(),
            VertexInputAttributeDescription::builder()
                .binding(0)
                .location(2)
                .format(Format::R8G8B8A8_UNORM)
                .offset(20)
                .build(),
        ]
    }
}
