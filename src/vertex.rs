//! # Vertex
//!
//! Vertex does:
//! - Defines a vertex
//! - Adds a descriptor function for buffers
//! - Handles indecies for more efficient verticies
//!
//! Current shapes:
//! - Rainbow triangle
//! - Magenta Irregular Pentagon
//! - Purple Irregular Pentagon
//! - Inverted Tree Irregular Pentagon
//! - Tree Irregular Pentagon
//!
//! After Camera:
//! - Tree Pentagon

pub trait Vertex {
    fn desc() -> wgpu::VertexBufferLayout<'static>;
}

/// # Image Vertex
///
/// Stores a vertex to make a triangle on the GPU.
/// Used for vertex buffers.
///
/// ## What is a Vertex Buffer?
///
/// A vertex buffer is a set of verticies sent directly to the GPU.
/// It prevents CPU/GPU connection bottlenecks.
/// Its a giant packet of vertecies stored in the VRAM of the GPU.
/// The GPU can just unpack and render all the verticies by itself much quicker.
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct ImageVertex {
    position: [f32; 3],
    // colors: [f32; 3],
    tex_coords: [f32; 2],
}

// Rainbow Triangle
// pub const VERTICES: &[Vertex] = &[
//     Vertex { position: [0.0, 0.5, 0.0], color: [1.0, 0.0, 0.0] },
//     Vertex { position: [-0.5, -0.5, 0.0], color: [0.0, 1.0, 0.0] },
//     Vertex { position: [0.5, -0.5, 0.0], color: [0.0, 0.0, 1.0] },
// ];

// Magenta Irregular Pentagon
// Magenta due to sRGB color conversion
// pub const VERTICES: &[Vertex] = &[
//     Vertex { position: [-0.0868241, 0.49240386, 0.0], color: [0.5, 0.0, 0.5] },
//     Vertex { position: [-0.49513406, 0.06958647, 0.0], color: [0.5, 0.0, 0.5] },
//     Vertex { position: [-0.21918549, -0.44939706, 0.0], color: [0.5, 0.0, 0.5] },
//     Vertex { position: [0.35966998, -0.3473291, 0.0], color: [0.5, 0.0, 0.5] },
//     Vertex { position: [0.44147372, 0.2347359, 0.0], color: [0.5, 0.0, 0.5] },
// ];

// Orders vertecies to lower byte count by handling dupes
// Saves 76 bytes over manually listing all vertecies including dupes
// pub const INDICES: &[u16] = &[
//     0, 1, 4,
//     1, 2, 4,
//     2, 3, 4,
// ];

// Inverted Tree Irregular Pentagon
// pub const VERTICES: &[Vertex] = &[
//     Vertex { position: [-0.0868241, 0.49240386, 0.0], tex_coords: [0.4131759, 0.99240386], },
//     Vertex { position: [-0.49513406, 0.06958647, 0.0], tex_coords: [0.0048659444, 0.56958647], },
//     Vertex { position: [-0.21918549, -0.44939706, 0.0], tex_coords: [0.28081453, 0.05060294], },
//     Vertex { position: [0.35966998, -0.3473291, 0.0], tex_coords: [0.85967, 0.1526709], },
//     Vertex { position: [0.44147372, 0.2347359, 0.0], tex_coords: [0.9414737, 0.7347359], },
// ];

// pub const INDICES: &[u16] = &[
//     0, 1, 4,
//     1, 2, 4,
//     2, 3, 4,
// ];

// Tree Irregular Pentagon
// No camera causes irregularity
// pub const VERTICES: &[Vertex] = &[
//     Vertex { position: [-0.0868241, 0.49240386, 0.0], tex_coords: [0.4131759, 0.00759614], },
//     Vertex { position: [-0.49513406, 0.06958647, 0.0], tex_coords: [0.0048659444, 0.43041354], },
//     Vertex { position: [-0.21918549, -0.44939706, 0.0], tex_coords: [0.28081453, 0.949397], },
//     Vertex { position: [0.35966998, -0.3473291, 0.0], tex_coords: [0.85967, 0.84732914], },
//     Vertex { position: [0.44147372, 0.2347359, 0.0], tex_coords: [0.9414737, 0.2652641], },
// ];

// pub const INDICES: &[u16] = &[
//     0, 1, 4,
//     1, 2, 4,
//     2, 3, 4,
// ];

// CAMERA CREATED

// Tree Pentagon
// Regular Pentagon due to camera distortion
// pub const VERTICES: &[ImageVertex] = &[
//     ImageVertex { position: [-0.0868241, 0.49240386, 0.0], tex_coords: [0.4131759, 0.00759614], },
//     ImageVertex { position: [-0.49513406, 0.06958647, 0.0], tex_coords: [0.0048659444, 0.43041354], },
//     ImageVertex { position: [-0.21918549, -0.44939706, 0.0], tex_coords: [0.28081453, 0.949397], },
//     ImageVertex { position: [0.35966998, -0.3473291, 0.0], tex_coords: [0.85967, 0.84732914], },
//     ImageVertex { position: [0.44147372, 0.2347359, 0.0], tex_coords: [0.9414737, 0.2652641], },
// ];

// pub const INDICES: &[u16] = &[
//     0, 1, 4,
//     1, 2, 4,
//     2, 3, 4,
// ];


impl Vertex for ImageVertex {
    /// # Desc
    ///
    /// Defines how a buffer is represented in memory
    ///
    /// Inefficient version for obvious layout
    fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            // How wide a vertex is
            // Will probably be 24 bytes
            array_stride: std::mem::size_of::<ImageVertex>() as wgpu::BufferAddress,
            // Whether each array element in this buffer represents per-vertex data or per-instance data
            step_mode: wgpu::VertexStepMode::Vertex,
            // Describes the individual parts of the vertex
            // 1:1 mapping with the struct's fields
            attributes: &[
                // Tells the shader what location to store this attribute at
                wgpu::VertexAttribute {
                    // Offset in bytes until the attribute starts
                    offset: 0,
                    // What location to store this attribute at
                    shader_location: 0,
                    // Tells the shader the shape of the attribute
                    // Float32x3 corresponds to vec3<f32>
                    format: wgpu::VertexFormat::Float32x3,
                },
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
                    shader_location: 1,
                    // format: wgpu::VertexFormat::Float32x3,
                    format: wgpu::VertexFormat::Float32x2,
                }
            ]
        }
    }
}