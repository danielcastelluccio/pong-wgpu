#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct Vertex {
    position: [f32; 3],
    color: [f32; 4]
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct PaddleRaw {
    position: [f32; 3]
}

const VERTICES: &[Vertex] = &[
    Vertex { position: [0.05, -0.1, 0.0], color: [0.9, 0.9, 0.9, 1.0] },
    Vertex { position: [0.05, 0.1, 0.0], color: [0.9, 0.9, 0.9, 1.0] },
    Vertex { position: [-0.05, -0.1, 0.0], color: [0.9, 0.9, 0.9, 1.0] },
    Vertex { position: [-0.05, 0.1, 0.0], color: [0.9, 0.9, 0.9, 1.0] }
];

const INDICES: &[u16] = &[
    0, 1, 2, 3, 2, 1
];

struct PongState {
    surface: wgpu::Surface,
    device: wgpu::Device,
    queue: wgpu::Queue,
    swap_chain_descriptor: wgpu::SwapChainDescriptor,
    swap_chain: wgpu::SwapChain,
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    render_pipeline: wgpu::RenderPipeline,
    instance_data: [PaddleRaw; 2],
    instance_buffer: wgpu::Buffer,
    pressed_keycodes: Vec<winit::event::VirtualKeyCode>
}

impl PongState {
    async fn new(window: &winit::window::Window) -> Self {
        let size = window.inner_size();
        let instance = wgpu::Instance::new(wgpu::BackendBit::PRIMARY);
        let surface = unsafe { instance.create_surface(window) };
        let adapter = instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::default(),
            compatible_surface: Some(&surface)
        }).await.unwrap();
        let (device, queue) = adapter.request_device(&wgpu::DeviceDescriptor {
            label: Some("Device"),
            features: wgpu::Features::empty(),
            limits: wgpu::Limits::default()
        },
        None).await.unwrap();

        let swap_chain_descriptor = wgpu::SwapChainDescriptor {
            usage: wgpu::TextureUsage::RENDER_ATTACHMENT,
            format: adapter.get_swap_chain_preferred_format(&surface).unwrap(),
            width: size.width,
            height: size.height,
            present_mode: wgpu::PresentMode::Fifo
        };
        let swap_chain = device.create_swap_chain(&surface, &swap_chain_descriptor);

        let shader_module = device.create_shader_module(&wgpu::ShaderModuleDescriptor {
            label: Some("Shader Module"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shader.wgsl").into()),
            flags: wgpu::ShaderFlags::all()
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Pipeline Layout"),
            bind_group_layouts: &[],
            push_constant_ranges: &[]
        });

        let vertex_descriptor = wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex>() as u64,
            step_mode: wgpu::InputStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32x3,
                    offset: 0,
                    shader_location: 0
                },
                wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32x4,
                    offset: std::mem::size_of::<[f32; 3]>() as u64,
                    shader_location: 1
                }
            ]
        };

        let vertex_buffer_raw = bytemuck::cast_slice(VERTICES);
        let vertex_buffer = wgpu::util::DeviceExt::create_buffer_init(&device, &wgpu::util::BufferInitDescriptor {
            label: Some("Vertex Buffer"),
            contents: vertex_buffer_raw,
            usage: wgpu::BufferUsage::VERTEX
        });

        let index_buffer_raw = bytemuck::cast_slice(INDICES);
        let index_buffer = wgpu::util::DeviceExt::create_buffer_init(&device, &wgpu::util::BufferInitDescriptor {
            label: Some("Index Buffer"),
            contents: index_buffer_raw,
            usage: wgpu::BufferUsage::INDEX
        });

        let instance_positions_descriptor = wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<PaddleRaw>() as u64,
            step_mode: wgpu::InputStepMode::Instance,
            attributes: &[
                wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32x3,
                    offset: 0,
                    shader_location: 2
                }
            ]
        };

        let instance_data = [
            PaddleRaw { position: [-0.9, 0.0, 0.0] },
            PaddleRaw { position: [0.9, 0.0, 0.0] }
        ];

        let instance_buffer_raw = bytemuck::cast_slice(&instance_data);
        let instance_buffer = wgpu::util::DeviceExt::create_buffer_init(&device, &wgpu::util::BufferInitDescriptor {
            label: Some("Instance Buffer"),
            contents: instance_buffer_raw,
            usage: wgpu::BufferUsage::VERTEX
        });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader_module,
                entry_point: "main",
                buffers: &[vertex_descriptor, instance_positions_descriptor]
            },
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                clamp_depth: false,
                polygon_mode: wgpu::PolygonMode::Fill,
                conservative: false
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader_module,
                entry_point: "main",
                targets: &[wgpu::ColorTargetState {
                    format: swap_chain_descriptor.format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrite::ALL
                }]
            })
        });

        PongState {
            surface,
            device,
            queue,
            swap_chain_descriptor,
            swap_chain,
            vertex_buffer,
            index_buffer,
            render_pipeline,
            instance_data,
            instance_buffer,
            pressed_keycodes: Vec::new(),
        }
    }

    fn resize(&mut self, size: winit::dpi::PhysicalSize<u32>) {
        self.swap_chain_descriptor.width = size.width;
        self.swap_chain_descriptor.height = size.height;
        self.swap_chain = self.device.create_swap_chain(&self.surface, &self.swap_chain_descriptor);
    }

    fn update(&mut self, delta_time: std::time::Duration) {
        let mut changed = false;

        let movement_amount = delta_time.as_millis() as f32 * 0.002;

        if self.pressed_keycodes.contains(&winit::event::VirtualKeyCode::Up)  {
            self.instance_data[1].position[1] += movement_amount;
            changed = true;
        }
        if self.pressed_keycodes.contains(&winit::event::VirtualKeyCode::Down) {
            self.instance_data[1].position[1] -= movement_amount;
            changed = true;
        }
        if self.pressed_keycodes.contains(&winit::event::VirtualKeyCode::W) {
            self.instance_data[0].position[1] += movement_amount;
            changed = true;
        }
        if self.pressed_keycodes.contains(&winit::event::VirtualKeyCode::S) {
            self.instance_data[0].position[1] -= movement_amount;
            changed = true;
        }
        //println!("{} {:?}", changed, self.instance_data[0]);
        
        if changed {
            let instance_buffer_raw = bytemuck::cast_slice(&self.instance_data);
            self.instance_buffer = wgpu::util::DeviceExt::create_buffer_init(&self.device, &wgpu::util::BufferInitDescriptor {
                label: Some("Instance Buffer"),
                contents: instance_buffer_raw,
                usage: wgpu::BufferUsage::VERTEX
            });
        }
    }
    
    fn input(&mut self, input: winit::event::KeyboardInput) {
        if let Some(keycode) = input.virtual_keycode {
            if input.state == winit::event::ElementState::Pressed {
                if !self.pressed_keycodes.contains(&keycode) {
                    self.pressed_keycodes.push(keycode);
                }
                
            }
            if input.state == winit::event::ElementState::Released {
                let keycode_index = self.pressed_keycodes.iter().position(|&r| r == keycode);
                if let Some(keycode_index) = keycode_index {
                    self.pressed_keycodes.remove(keycode_index);
                }
            }
        }
    }
    
    fn render(&self) {
        let frame = self.swap_chain.get_current_frame().unwrap();
        
        let mut command_encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Command Encoder")
        });
        
        {
            let mut render_pass = command_encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[
                    wgpu::RenderPassColorAttachment {
                        view: &frame.output.view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color {
                                r: 0.0,
                                g: 0.0,
                                b: 0.0,
                                a: 1.0
                            }),
                            store: true
                        }
                    }
                ],
                depth_stencil_attachment: None
            });

            render_pass.set_pipeline(&self.render_pipeline);
            render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
            render_pass.set_vertex_buffer(1, self.instance_buffer.slice(..));
            render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
            render_pass.draw_indexed(0..INDICES.len() as u32, 0, 0..2);
        }
        
        self.queue.submit(std::iter::once(command_encoder.finish()));
    }
}

fn main() {
    env_logger::init();
    
    let event_loop = winit::event_loop::EventLoop::new();
    let window = winit::window::WindowBuilder::new().build(&event_loop).unwrap();
    
    let mut state = pollster::block_on(PongState::new(&window));

    let mut previous_frame_time = std::time::Instant::now();
    
    event_loop.run( move |event, _, control_flow| {
        match event {
            winit::event::Event::WindowEvent {
                event, .. } => {
                match event {
                    winit::event::WindowEvent::CloseRequested => *control_flow = winit::event_loop::ControlFlow::Exit,
                    winit::event::WindowEvent::Resized(size) => state.resize(size),
                    winit::event::WindowEvent::KeyboardInput {
                        input,
                        ..
                    } => state.input(input),
                    _ => {}
                }
            },
            winit::event::Event::MainEventsCleared => window.request_redraw(),
            winit::event::Event::RedrawRequested(_) => {
                let frame_time = std::time::Instant::now();
                let delta_time = frame_time - previous_frame_time;
                state.update(delta_time);
                state.render();
                previous_frame_time = frame_time;
            }
            _ => {}
        }
    });
}
