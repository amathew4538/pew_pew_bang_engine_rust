use std::io::Cursor;
use wgpu::util::DeviceExt;
use crate::model;
use crate::texture;

#[cfg(target_arch = "wasm32")]
fn format_url(file_name: &str) -> reqwest::Url {
    let window = web_sys::window().unwrap();
    let location = window.location();
    let mut origin = location.origin().unwrap();
    if !origin.ends_with("learn-wgpu") {
        origin = format!("{}/learn-wgpu", origin);
    }
    let base = reqwest::Url::parse(&format!("{}/", origin,)).unwrap();
    base.join(file_name).unwrap()
}

pub async fn load_string(file_name: &str) -> anyhow::Result<String> {
    #[cfg(target_arch = "wasm32")]
    let txt = {
        let url = format_url(file_name);
        reqwest::get(url).await?.text().await?
    };
    #[cfg(not(target_arch = "wasm32"))]
    let txt: String = {
        let path: std::path::PathBuf = std::path::Path::new(env!("OUT_DIR"))
            .join("res")
            .join(file_name);
        std::fs::read_to_string(path)?
    };

    Ok(txt)
}

pub async fn load_binary(file_name: &str) -> anyhow::Result<Vec<u8>> {
    #[cfg(target_arch = "wasm32")]
    let data = {
        let url = format_url(file_name);
        reqwest::get(url).await?.bytes().await?.to_vec()
    };
    #[cfg(not(target_arch = "wasm32"))]
    let data: Vec<u8> = {
        let path: std::path::PathBuf = std::path::Path::new(env!("OUT_DIR"))
            .join("res")
            .join(file_name);
        std::fs::read(path)?
    };

    Ok(data)
}

/// # Load Texture
///
/// Loads model textures without knowing the file name at compile time
pub async fn load_texture(
    file_name: &str,
    is_normal_map: bool,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
) -> anyhow::Result<texture::Texture> {
    let data: Vec<u8> = load_binary(file_name).await?;
    texture::Texture::from_bytes(device, queue, &data, file_name, is_normal_map)
}

pub async fn load_model(
    file_name: &str,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    layout: &wgpu::BindGroupLayout,
) -> anyhow::Result<model::Model> {
    let obj_text: String = load_string(file_name).await?;
    let obj_cursor: Cursor<String> = Cursor::new(obj_text);
    let mut obj_reader: tokio::io::BufReader<Cursor<String>> = tokio::io::BufReader::new(obj_cursor);

    let (models, obj_materials) = tobj::tokio::load_obj_buf(
        &mut obj_reader,
        &tobj::LoadOptions {
            triangulate: true,
            single_index: true,
            ..Default::default()
        },
        |p| async move {
            let mat_text: String = load_string(&p.to_str().unwrap()).await.unwrap();

            tobj::tokio::load_mtl_buf(tokio::io::BufReader::new(Cursor::new(mat_text))).await
        },
    )
    .await?;

    let mut materials: Vec<model::Material> = Vec::new();
    for m in obj_materials? {
        let diffuse_texture: texture::Texture = load_texture(&m.diffuse_texture.unwrap(), false, device, queue).await?;
        let normal_texture: texture::Texture = load_texture(&m.normal_texture.unwrap(),true, device, queue).await?;

        materials.push(model::Material::new(
            device,
            &m.name,
            diffuse_texture,
            normal_texture,
            layout,
        ));
    }

    let meshes: Vec<model::Mesh> = models
        .into_iter()
        .map(|m| {
            let mut vertices = (0..m.mesh.positions.len() / 3)
                .map(|i| model::ModelVertex {
                    position: [
                        m.mesh.positions[i * 3],
                        m.mesh.positions[i * 3 + 1],
                        m.mesh.positions[i * 3 + 2],
                    ],
                    tex_coords: [m.mesh.texcoords[i * 2], 1.0 - m.mesh.texcoords[i * 2 + 1]],
                    normal: [
                        m.mesh.normals[i * 3],
                        m.mesh.normals[i * 3 + 1],
                        m.mesh.normals[i * 3 + 2],
                    ],
                    // We'll calculate these later
                    tangent: [0.0; 3],
                    bitangent: [0.0; 3],
                })
                .collect::<Vec<model::ModelVertex>>();

            let indices: &Vec<u32> = &m.mesh.indices;
            let mut triangles_included: Vec<i32> = vec![0; vertices.len()];

            // Calculate tangents and bitangets. We're going to
            // use the triangles, so we need to loop through the
            // indices in chunks of 3
            for c in indices.chunks(3) {
                let v0: model::ModelVertex = vertices[c[0] as usize];
                let v1: model::ModelVertex = vertices[c[1] as usize];
                let v2: model::ModelVertex = vertices[c[2] as usize];

                let pos0: glam::Vec3 = v0.position.into();
                let pos1: glam::Vec3 = v1.position.into();
                let pos2: glam::Vec3 = v2.position.into();

                let uv0: glam::Vec2 = v0.tex_coords.into();
                let uv1: glam::Vec2 = v1.tex_coords.into();
                let uv2: glam::Vec2 = v2.tex_coords.into();

                // Calculate the edges of the triangle
                let delta_pos1: glam::Vec3 = pos1 - pos0;
                let delta_pos2: glam::Vec3 = pos2 - pos0;

                // This will give us a direction to calculate the
                // tangent and bitangent
                let delta_uv1: glam::Vec2 = uv1 - uv0;
                let delta_uv2: glam::Vec2 = uv2 - uv0;

                // Solving the following system of equations will
                // give us the tangent and bitangent.
                //     delta_pos1 = delta_uv1.x * T + delta_u.y * B
                //     delta_pos2 = delta_uv2.x * T + delta_uv2.y * B
                // Luckily, the place I found this equation provided
                // the solution!
                let r: f32 = 1.0 / (delta_uv1.x * delta_uv2.y - delta_uv1.y * delta_uv2.x);
                let tangent: glam::Vec3 = (delta_pos1 * delta_uv2.y - delta_pos2 * delta_uv1.y) * r;
                // We flip the bitangent to enable right-handed normal
                // maps with wgpu texture coordinate system
                let bitangent: glam::Vec3 = (delta_pos2 * delta_uv1.x - delta_pos1 * delta_uv2.x) * -r;

                // We'll use the same tangent/bitangent for each vertex in the triangle
                vertices[c[0] as usize].tangent =
                    (tangent + glam::Vec3::from(vertices[c[0] as usize].tangent)).into();
                vertices[c[1] as usize].tangent =
                    (tangent + glam::Vec3::from(vertices[c[1] as usize].tangent)).into();
                vertices[c[2] as usize].tangent =
                    (tangent + glam::Vec3::from(vertices[c[2] as usize].tangent)).into();
                vertices[c[0] as usize].bitangent =
                    (bitangent + glam::Vec3::from(vertices[c[0] as usize].bitangent)).into();
                vertices[c[1] as usize].bitangent =
                    (bitangent + glam::Vec3::from(vertices[c[1] as usize].bitangent)).into();
                vertices[c[2] as usize].bitangent =
                    (bitangent + glam::Vec3::from(vertices[c[2] as usize].bitangent)).into();

                // Used to average the tangents/bitangents
                triangles_included[c[0] as usize] += 1;
                triangles_included[c[1] as usize] += 1;
                triangles_included[c[2] as usize] += 1;
            }

            // Average the tangents/bitangents
            for (i, n) in triangles_included.into_iter().enumerate() {
                let denom: f32 = 1.0 / n as f32;
                let v: &mut model::ModelVertex = &mut vertices[i];
                v.tangent = (glam::Vec3::from(v.tangent) * denom).into();
                v.bitangent = (glam::Vec3::from(v.bitangent) * denom).into();
            }

            let vertex_buffer: wgpu::Buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some(&format!("{:?} Vertex Buffer", file_name)),
                contents: bytemuck::cast_slice(&vertices),
                usage: wgpu::BufferUsages::VERTEX,
            });
            let index_buffer: wgpu::Buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some(&format!("{:?} Index Buffer", file_name)),
                contents: bytemuck::cast_slice(&m.mesh.indices),
                usage: wgpu::BufferUsages::INDEX,
            });

            model::Mesh {
                name: file_name.to_string(),
                vertex_buffer,
                index_buffer,
                num_elements: m.mesh.indices.len() as u32,
                material: m.mesh.material_id.unwrap_or(0),
            }
        })
        .collect::<Vec<_>>();

    Ok(model::Model { meshes, materials })
}

pub fn create_render_pipeline(
    device: &wgpu::Device,
    layout: &wgpu::PipelineLayout,
    color_format: wgpu::TextureFormat,
    depth_format: Option<wgpu::TextureFormat>,
    vertex_layouts: &[Option<wgpu::VertexBufferLayout>],
    shader: wgpu::ShaderModuleDescriptor,
) -> wgpu::RenderPipeline {
    let shader: wgpu::ShaderModule = device.create_shader_module(shader);

    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        // Name
        label: Some("Render Pipeline"),
        layout: Some(layout),
        vertex: wgpu::VertexState {
            module: &shader,
            // Specify which function inside the shader should be the entry_point (@vertex)
            entry_point: Some("vs_main"),
            // Tells wgpu what type of vertices we want to pass to the vertex shader
            // Specified in shader currently
            buffers: vertex_layouts,
            compilation_options: wgpu::PipelineCompilationOptions::default(),
        },
        // Technically optional so have to wrap it in Some()
        // Need it to store color data to the surface
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            // Specify which function inside the shader should be the entry_point (@fragment)
            entry_point: Some("fs_main"),
            // Tells wgpu what color outputs it should set up
            targets: &[Some(wgpu::ColorTargetState {
                // Use the surface's format so that copying to it is easy
                format: color_format,
                // Replace old pixel data with new data
                blend: Some(wgpu::BlendState::REPLACE),
                // Write to all colors (red, blue, green, alpha)
                write_mask: wgpu::ColorWrites::ALL,
            })],
            compilation_options: wgpu::PipelineCompilationOptions::default(),
        }),

        // How to interpret vertices when converting to triangles
        primitive: wgpu::PrimitiveState {
            // Three vertices correspond to one triangle
            topology: wgpu::PrimitiveTopology::TriangleList,
            strip_index_format: None,
            // Triangle is facing forward if vertices are in counter-clockwise direction
            front_face: wgpu::FrontFace::Ccw,
            // Culls triangles
            cull_mode: None, // Some(wgpu::Face::Back),
            // Setting this to anything other than Fill requires Features::NON_FILL_POLYGON_MODE
            polygon_mode: wgpu::PolygonMode::Fill,
            // Requires Features::DEPTH_CLIP_CONTROL
            unclipped_depth: false,
            // Requires Features::CONSERVATIVE_RASTERIZATION
            conservative: false,
        },
        depth_stencil: depth_format.map(|format| wgpu::DepthStencilState {
            format,
            depth_write_enabled: Some(true),
            // Pixels will be drawn front to back
            depth_compare: Some(wgpu::CompareFunction::Less),
            // Different type of buffer combined with depth
            stencil: wgpu::StencilState::default(),
            bias: wgpu::DepthBiasState::default(),
        }),
        multisample: wgpu::MultisampleState {
            // How many samples to use
            count: 1,
            // Which samples to use
            // This is all of them
            mask: !0,
            // Anti-Aliasing
            alpha_to_coverage_enabled: false,
        },
        // How many array layers the render attachments can have
        multiview_mask: None,
        // Cache shader compilation data
        // Only really useful for Android build targets
        cache: None,
    })
}
