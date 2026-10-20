//! # State
//!
//! Stores window state
//!
//! Handles rendering, resizing window details

use anyhow::Ok;
use winit::{
    dpi::{PhysicalPosition},
    event_loop::{ActiveEventLoop},
    keyboard::{KeyCode},
    window::{Window}
};
use std::sync::Arc;
use wgpu::util::DeviceExt;
use crate::{
    camera::{Camera, CameraController, CameraUniform, Projection},
    instance::{Instance, InstanceRaw},
    light::{LightUniform},
    model::{self, DrawModel, Model},
    resources, texture::Texture,
    vertex::{Vertex}
};


/// # State
///
/// Stores the current state of the device, window, surface, etc.
pub struct State {
    /// Surface the app writes to (where textures are written to)
    ///
    /// Lasts for the entire duration of the program
    surface: wgpu::Surface<'static>,
    /// The GPU device reference
    device: wgpu::Device,
    /// Handles buffers, textures, and other queue-related things per device
    queue: wgpu::Queue,
    /// Config for the app window surface
    config: wgpu::SurfaceConfiguration,
    /// Quick check for config in case of failures
    is_surface_configured: bool,
    /// Window the app state is storing
    window: Arc<Window>,
    /// Renders the vertices on the screen
    render_pipeline: wgpu::RenderPipeline,
    // /// Holds the vertecies to be sent to the GPU
    // vertex_buffer: wgpu::Buffer,
    // /// Holds the indecies for the buffer
    // index_buffer: wgpu::Buffer,
    // /// How many indecies there are
    // num_indices: u32,
    // /// Texture bind group
    // diffuse_bind_group: wgpu::BindGroup,
    // /// The actual texture
    // diffuse_texture: Texture,
    /// The virtual camera for the window
    camera: Camera,
    projection: Projection,
    /// Camera control for movement
    camera_controller: CameraController,
    /// Camera data
    camera_uniform: CameraUniform,
    /// Data buffered from camera
    camera_buffer: wgpu::Buffer,
    /// Bind Group for camera
    camera_bind_group: wgpu::BindGroup,
    /// Instances
    instances: Vec<Instance>,
    /// Buffer to send to GPU of instances
    instance_buffer: wgpu::Buffer,
    /// Depth buffer for culling and renering better with depth
    depth_texture: Texture,
    /// 3D model to render
    obj_model: Model,
    /// Light
    light_uniform: LightUniform,
    light_buffer: wgpu::Buffer,
    light_bind_group: wgpu::BindGroup,
    light_render_pipeline: wgpu::RenderPipeline,
    mouse_pressed: bool,
}

impl State {
    /// Creates a new window
    pub async fn new(window: Arc<Window>) -> anyhow::Result<State> {

        let size: winit::dpi::PhysicalSize<u32> = window.inner_size();

        // The instance is a handle to our GPU
        // BackendBit::PRIMARY => Vulkan + Metal + DX12 + Browser WebGPU
        let instance: wgpu::Instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            #[cfg(not(target_arch = "wasm32"))]
            backends: wgpu::Backends::PRIMARY,
            #[cfg(target_arch = "wasm32")]
            backends: wgpu::Backends::GL,
            flags: Default::default(),
            memory_budget_thresholds: Default::default(),
            backend_options: Default::default(),
            display: None,
        });

        // Part of the window to draw to
        let surface: wgpu::Surface<'_> = instance.create_surface(window.clone()).unwrap();

        // Handle for physical GPU
        let adapter: wgpu::Adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                // 3 Possible Values:
                // None: not considered
                // Low Power: iGPU, forces lower resources
                // High Performance: Dedicated GPU, less capped resources
                power_preference: wgpu::PowerPreference::default(),

                // Forces an adapter for all possible systems
                force_fallback_adapter: false,
                compatible_surface: Some(&surface),
                apply_limit_buckets: true,
            })
        .await?;

        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: None,
                // Defines extra features for the devices
                required_features: wgpu::Features::empty(),
                // Defines extra UNSTABLE features for the devices
                experimental_features: wgpu::ExperimentalFeatures::disabled(),

                // WebGL doesn't support all of wgpu's features, so if we're building for the web we'll have to disable some.
                // Describes the limit of certain types of resources that it can create
                required_limits: if cfg!(target_arch = "wasm32") {
                    wgpu::Limits::downlevel_webgl2_defaults()
                } else {
                    wgpu::Limits::default()
                },

                // Define Memory Allocation Strategy
                memory_hints: Default::default(),
                trace: wgpu::Trace::Off,
            })
        .await?;

        let surface_caps: wgpu::SurfaceCapabilities = surface.get_capabilities(&adapter);

        // Shader code in this tutorial assumes an sRGB surface texture. Using a different one will result in all the colors coming out darker.
        // If you want to support non-sRGB surfaces, you'll need to account for that when drawing to the frame.
        let surface_format: wgpu::TextureFormat = surface_caps.formats.iter()
            .find(|f: &&wgpu::TextureFormat| f.is_srgb())
            .copied()
            .unwrap_or(surface_caps.formats[0]);

        let config: wgpu::wgt::SurfaceConfiguration<Vec<wgpu::TextureFormat>> = wgpu::SurfaceConfiguration {
            // Describes how SurfaceTextures will be used
            // Specifies that the textures will be used to write to the screen
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,

            // Defines how SurfaceTextures will be stored on the GPU
            format: surface_format,
            width: size.width,
            height: size.height,

            // Determines how to sync the surface with the display
            // PresentMode::Fifo will do VSync
            present_mode: surface_caps.present_modes[0],
            // Controls window alpha channel
            alpha_mode: surface_caps.alpha_modes[0],
            // A list of TextureFormats that are used when creating TextureViews
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
            // Color space for window (sRGB, Display P3, etc.)
            color_space: wgpu::SurfaceColorSpace::Auto,
        };

        // Get bytes from image
        // let diffuse_bytes: &[u8; _] = include_bytes!("happy-tree.png");
        // Turn bytes into image texture
        // let diffuse_texture: Texture = Texture::from_bytes(&device, &queue, diffuse_bytes, "happy-tree.png").unwrap();

        let texture_bind_group_layout: wgpu::BindGroupLayout = device.create_bind_group_layout(
            &wgpu::BindGroupLayoutDescriptor {
                // Gives textures to fragment shader
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            multisampled: false,
                            view_dimension: wgpu::TextureViewDimension::D2,
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        // This should match the filterable field of the
                        // corresponding Texture entry above.
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                    // Normal maps for objs
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            multisampled: false,
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            view_dimension: wgpu::TextureViewDimension::D2,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 3,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
                label: Some("texture_bind_group_layout"),
            }
        );

        // let diffuse_bind_group: wgpu::BindGroup = device.create_bind_group(
        //     &wgpu::BindGroupDescriptor {
        //         layout: &texture_bind_group_layout,
        //         entries: &[
        //             wgpu::BindGroupEntry {
        //                 binding: 0,
        //                 resource: wgpu::BindingResource::TextureView(&diffuse_texture.view),
        //             },
        //             wgpu::BindGroupEntry {
        //                 binding: 1,
        //                 resource: wgpu::BindingResource::Sampler(&diffuse_texture.sampler),
        //             }
        //         ],
        //         label: Some("diffuse_bind_group"),
        //     }
        // );

        let camera: Camera = Camera::new(
            // +z is out of the screen
            (0.0, 5.0, 10.0),
            // have it look at the origin
            (-90 as f32).to_radians(),
            // which way is "up"
            (-20 as f32).to_radians()
        );
        let projection: Projection = Projection::new(config.width, config.height, (45.0 as f32).to_radians(), 0.1, 100.0);
        let camera_controller: CameraController = CameraController::new(4.0, 0.4);

        let mut camera_uniform: CameraUniform = CameraUniform::new();
        camera_uniform.update_view_proj(&camera, &projection);

        let camera_buffer: wgpu::Buffer = device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("Camera Buffer"),
                contents: bytemuck::cast_slice(&[camera_uniform]),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            }
        );

        let camera_bind_group_layout: wgpu::BindGroupLayout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    // Only need camera information in vertex shader to manipulate vertecies
                    // Camera doesnt't affect colors so no need for fragment
                    visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        // Whether the location of the data in the buffer may change
                        has_dynamic_offset: false,
                        // The smallest size the buffer can be
                        min_binding_size: None,
                    },
                    count: None,
                }
            ],
            label: Some("camera_bind_group_layout"),
        });

        let camera_bind_group: wgpu::BindGroup = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &camera_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: camera_buffer.as_entire_binding(),
                }
            ],
            label: Some("camera_bind_group"),
        });

        let light_uniform: LightUniform = LightUniform::new(
            [2.0, 2.0, 2.0],
            [1.0, 1.0, 1.0]
        );

        // We'll want to update our lights position, so we use COPY_DST
        let light_buffer: wgpu::Buffer = device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("Light VB"),
                contents: bytemuck::cast_slice(&[light_uniform]),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            }
        );

        let light_bind_group_layout: wgpu::BindGroupLayout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
                label: None,
            });

        let light_bind_group: wgpu::BindGroup = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &light_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: light_buffer.as_entire_binding(),
            }],
            label: None,
        });

        let render_pipeline_layout: wgpu::PipelineLayout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout"),
                // List of BindGroupLayouts that the pipeline can use
                bind_group_layouts: &[
                    Some(&texture_bind_group_layout),
                    Some(&camera_bind_group_layout),
                    Some(&light_bind_group_layout),
                ],
                immediate_size: 0,
            });

        let render_pipeline: wgpu::RenderPipeline = {
            let shader: wgpu::ShaderModuleDescriptor<'_> = wgpu::ShaderModuleDescriptor {
                label: Some("Normal Shader"),
                source: wgpu::ShaderSource::Wgsl(include_str!("shader.wgsl").into()),
            };
            resources::create_render_pipeline(
                &device,
                &render_pipeline_layout,
                config.format,
                Some(Texture::DEPTH_FORMAT),
                &[Some(model::ModelVertex::desc()), Some(InstanceRaw::desc())],
                shader,
            )
        };

        let light_render_pipeline: wgpu::RenderPipeline = {
            let layout: wgpu::PipelineLayout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Light Pipeline Layout"),
                bind_group_layouts: &[Some(&camera_bind_group_layout), Some(&light_bind_group_layout)],
                immediate_size: 0,
            });
            let shader: wgpu::ShaderModuleDescriptor<'_> = wgpu::ShaderModuleDescriptor {
                label: Some("Light Shader"),
                source: wgpu::ShaderSource::Wgsl(include_str!("light.wgsl").into()),
            };
            resources::create_render_pipeline(
                &device,
                &layout,
                config.format,
                Some(Texture::DEPTH_FORMAT),
                &[Some(model::ModelVertex::desc())],
                shader,
            )
        };

        // Creates the vertex buffer
        // let vertex_buffer: wgpu::Buffer = device.create_buffer_init(
        //     &wgpu::util::BufferInitDescriptor {
        //         // Name
        //         label: Some("Vertex Buffer"),
        //         // The verticies and where they are
        //         contents: bytemuck::cast_slice(VERTICES),
        //         // What the contents are
        //         usage: wgpu::BufferUsages::VERTEX,
        //     }
        // );

        // Creates the index buffer
        // let index_buffer: wgpu::Buffer = device.create_buffer_init(
        //     &wgpu::util::BufferInitDescriptor {
        //         label: Some("Index Buffer"),
        //         contents: bytemuck::cast_slice(INDICES),
        //         usage: wgpu::BufferUsages::INDEX,
        //     }
        // );

        // Counts the number of indecies in the buffer
        // let num_indices: u32 = INDICES.len() as u32;

        let obj_model: Model =
            resources::load_model("cube.obj", &device, &queue, &texture_bind_group_layout)
                .await
                .unwrap();

        const NUM_INSTANCES_PER_ROW: u32 = 10;
        // const INSTANCE_DISPLACEMENT: glam::Vec3 = glam::Vec3::new(NUM_INSTANCES_PER_ROW as f32 * 0.5, 0.0, NUM_INSTANCES_PER_ROW as f32 * 0.5);
        const SPACE_BETWEEN: f32 = 3.0;

        let instances: Vec<Instance> = (0..NUM_INSTANCES_PER_ROW).flat_map(|z: u32| {
            (0..NUM_INSTANCES_PER_ROW).map(move |x: u32| {
                let x: f32 = SPACE_BETWEEN * (x as f32 - NUM_INSTANCES_PER_ROW as f32 / 2.0);
                let z: f32 = SPACE_BETWEEN * (z as f32 - NUM_INSTANCES_PER_ROW as f32 / 2.0);

                let position: glam::Vec3 = glam::Vec3 { x, y: 0.0, z };
                // let position: glam::Vec3 = glam::Vec3 { x: x as f32, y: 0.0, z: z as f32 } - INSTANCE_DISPLACEMENT;

                let rotation: glam::Quat = if position == glam::Vec3::ZERO {
                    // This is needed so an object at (0, 0, 0) won't get scaled to zero
                    // Quaternions can affect scale if they're not created correctly
                    // That might just be cgmath though
                    glam::Quat::from_axis_angle(glam::Vec3::Z, 0.0)
                } else {
                    glam::Quat::from_axis_angle(position.normalize(), 45.0)
                };

                Instance::new(position, rotation)
            })
        }).collect::<Vec<Instance>>();

        let instance_data: Vec<InstanceRaw> = instances.iter().map(Instance::to_raw).collect::<Vec<InstanceRaw>>();
        let instance_buffer: wgpu::Buffer = device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("Instance Buffer"),
                contents: bytemuck::cast_slice(&instance_data),
                usage: wgpu::BufferUsages::VERTEX,
            }
        );

        let depth_texture: Texture = Texture::create_depth_texture(&device, &config, "depth_texture");

        // Return the fancy new wgpu variables :)
       Ok(Self {
            surface,
            device,
            queue,
            config,
            is_surface_configured: false,
            window,
            render_pipeline,
            // vertex_buffer,
            // index_buffer,
            // num_indices,
            // diffuse_bind_group,
            // diffuse_texture,
            camera,
            projection,
            camera_controller,
            camera_uniform,
            camera_buffer,
            camera_bind_group,
            instances,
            instance_buffer,
            depth_texture,
            obj_model,
            light_uniform,
            light_buffer,
            light_bind_group,
            light_render_pipeline,
            mouse_pressed: false,
        })
    }

    /// # Resize
    ///
    /// Handles window resize call
    ///
    /// Parameters must be greater than 0 (prevents crashes)
    ///
    /// Reconfigures the [Surface][`wgpu::Surface`] everytime the size changes
    ///
    /// Also configures the [Surface][`wgpu::Surface`] on startup
    pub fn resize(&mut self, width: u32, height: u32) {
        if width > 0 && height > 0 {
            self.projection.resize(width, height);
            // Max size of WebGL Support
            let max: u32 = 2048;
            self.config.width = width.min(max);
            self.config.height = height.min(max);
            self.depth_texture = Texture::create_depth_texture(&self.device, &self.config, "depth_texture");
            self.surface.configure(&self.device, &self.config);
            self.is_surface_configured = true;
        }
    }

    /// # Render
    ///
    /// Forces winit to render as soon as possible
    pub fn render(&mut self) -> anyhow::Result<()> {
        self.window.request_redraw();

        // We can't render unless the surface is configured
        if !self.is_surface_configured {
            return Ok(());
        }

        // Wait for the Surface to provide a new SurfaceTexture that we will render to
        let output: wgpu::SurfaceTexture = match self.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(surface_texture) => surface_texture,
            wgpu::CurrentSurfaceTexture::Suboptimal(surface_texture) => {
                surface_texture
            }
            wgpu::CurrentSurfaceTexture::Timeout
            | wgpu::CurrentSurfaceTexture::Occluded
            | wgpu::CurrentSurfaceTexture::Validation => {
                // Skip this frame
                return Ok(());
            }
            wgpu::CurrentSurfaceTexture::Outdated => {
                self.surface.configure(&self.device, &self.config);
                return Ok(());
            }
            wgpu::CurrentSurfaceTexture::Lost => {
                // You could recreate the devices and all resources created with it here, but we'll just bail
                anyhow::bail!("Lost device");
            }
        };

        // Creates a TextureView with default settings to control how the render code interacts with the texture
        let view: wgpu::TextureView = output.texture.create_view(&wgpu::TextureViewDescriptor::default());

        // Create the actual commands to send to the GPU
        // Most modern graphics frameworks expect commands to be stored in a command buffer before being sent to the GPU
        // The encoder builds a command buffer that we can then send to the GPU.
        let mut encoder: wgpu::CommandEncoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Render Encoder"),
        });

        // Can't call encoder.finish() until mutable borrow by begin_render_pass on encoder released
        // Tells Rust to drop any variables within it when the code leaves that scope
        {
            // borrows the encoder mutably (&mut self)
            let mut render_pass: wgpu::RenderPass<'_> = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                // Name for debugging
                label: Some("Render Pass"),
                // Where to draw the color to and what color to draw
                // This is what @location(0) in the fragment shader targets
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    // Informs wgpu what texture to save the colors to
                    // Texture View from earlier, so draws to the screen
                    view: &view,
                    // Texture that will receive the resolved output
                    // Same as view unless multisampling is enabled
                    resolve_target: None,
                    depth_slice: None,
                    // Tells wgpu what to do with the colors on the screen (specified by view)
                    ops: wgpu::Operations {
                        // Tells wgpu how to handle colors stored from the previous frame
                        // Clearing the screen and applying a blue color
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.1,
                            g: 0.2,
                            b: 0.3,
                            a: 1.0,
                        }),
                        // Tells wgpu whether to store the rendered results to the Texture behind TextureView
                        // This stores the rendered results
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.depth_texture.view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                occlusion_query_set: None,
                timestamp_writes: None,
                multiview_mask: None,
            });

            use crate::light::DrawLight;
            render_pass.set_pipeline(&self.light_render_pipeline);
            render_pass.draw_light_model(
                &self.obj_model,
                &self.camera_bind_group,
                &self.light_bind_group,
            );

            // Set the pipeline on the render_pass
            render_pass.set_pipeline(&self.render_pipeline);

            // Render the texture image
            // render_pass.set_bind_group(0, &self.diffuse_bind_group, &[]);
            // Render the camera offsets
            // render_pass.set_bind_group(1, &self.camera_bind_group, &[]);

            // First parameter is what buffer slot to use for vertex buffer
            // Second parameter is the slice of the buffer to use
            // .slice() allows specifying which portion of buffer to use
            // .. to specifies the entire buffer
            // render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));

            // Add instance buffering
            render_pass.set_vertex_buffer(1, self.instance_buffer.slice(..));

            // Max of one index buffer set at a time
            // render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);

            // Tell wgpu to draw something based on the index count with multiple instances
            // render_pass.draw_indexed(0..self.num_indices, 0, 0..self.instances.len() as u32);

            render_pass.draw_model_instanced(
                &self.obj_model,
                0..self.instances.len() as u32,
                &self.camera_bind_group,
                &self.light_bind_group,
            );
        }
        // Borrow on encoder released by extra {}

        // Tell wgpu to finish the command buffer and submit it to the GPU's render queue
        // submit() will accept anything that implements IntoIter
        self.queue.submit(std::iter::once(encoder.finish()));
        self.queue.present(output);

        Ok(())
    }

    /// # Handle Key
    ///
    /// Handles user key inputes and perfoms actions when it matches
    ///
    /// Current keys include:
    /// - `Escape`: Close the app by exiting the `event_loop`
    pub fn handle_key(&mut self, event_loop: &ActiveEventLoop, code: KeyCode, is_pressed: bool) {
        if code == KeyCode::Escape && is_pressed {
            event_loop.exit();
        } else {
            self.camera_controller.handle_key(code, is_pressed);
        }
    }

    /// # Update
    ///
    /// Handles app updates
    ///
    /// Current updates include:
    /// - `NONE`
    pub fn update(&mut self) {
        self.camera_controller.update_camera(&mut self.camera);
        self.camera_uniform.update_view_proj(&self.camera);
        self.queue.write_buffer(&self.camera_buffer, 0, bytemuck::cast_slice(&[self.camera_uniform]));
        let old_position: glam::Vec3 = self.light_uniform.position.into();
        self.light_uniform.position =
            (glam::Quat::from_axis_angle((0.0, 1.0, 0.0).into(), 0.025)
                * old_position)
                .into();
        self.queue.write_buffer(&self.light_buffer, 0, bytemuck::cast_slice(&[self.light_uniform]));
    }
}
