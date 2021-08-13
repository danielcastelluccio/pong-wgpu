use cgmath::{Deg, Vector2};
use rand::Rng;

#[cfg(not(target_arch = "wasm32"))]
const BACKEND_BITS: wgpu::BackendBit = wgpu::BackendBit::PRIMARY;
#[cfg(target_arch = "wasm32")]
const BACKEND_BITS: wgpu::BackendBit = wgpu::BackendBit::all();

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct Vertex {
    position: [f32; 3],
    color: [f32; 4],
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct PaddleRaw {
    position: [f32; 3],
}

const PADDLE_VERTICES: &[Vertex] = &[
    Vertex {
        position: [0.025, -0.15, 0.0],
        color: [0.9, 0.9, 0.9, 1.0],
    },
    Vertex {
        position: [0.025, 0.15, 0.0],
        color: [0.9, 0.9, 0.9, 1.0],
    },
    Vertex {
        position: [-0.025, -0.15, 0.0],
        color: [0.9, 0.9, 0.9, 1.0],
    },
    Vertex {
        position: [-0.025, 0.15, 0.0],
        color: [0.9, 0.9, 0.9, 1.0],
    },
];

const BALL_VERTICES: &[Vertex] = &[
    Vertex {
        position: [0.025, -0.025, 0.0],
        color: [0.9, 0.9, 0.9, 1.0],
    },
    Vertex {
        position: [0.025, 0.025, 0.0],
        color: [0.9, 0.9, 0.9, 1.0],
    },
    Vertex {
        position: [-0.025, -0.025, 0.0],
        color: [0.9, 0.9, 0.9, 1.0],
    },
    Vertex {
        position: [-0.025, 0.025, 0.0],
        color: [0.9, 0.9, 0.9, 1.0],
    },
];

const RECTANGLE_INDICES: &[u16] = &[0, 1, 2, 3, 2, 1];

struct WgpuState {
    surface: wgpu::Surface,
    device: wgpu::Device,
    queue: wgpu::Queue,
    swap_chain_descriptor: wgpu::SwapChainDescriptor,
    swap_chain: wgpu::SwapChain,
    render_pipeline: wgpu::RenderPipeline,
}

struct PongState {
    wgpu_state: WgpuState,
    paddle_vertex_buffer: wgpu::Buffer,
    paddle_instance_data: [PaddleRaw; 2],
    paddle_instance_buffer: wgpu::Buffer,
    ball_vertex_buffer: wgpu::Buffer,
    ball_transform_data: cgmath::Vector2<f32>,
    ball_transform_buffer: wgpu::Buffer,
    ball_direction: cgmath::Deg<f32>,
    rectangle_index_buffer: wgpu::Buffer,
    pressed_keycodes: Vec<winit::event::VirtualKeyCode>,
}

impl PongState {
    async fn new(window: &winit::window::Window) -> Self {
        let size = window.inner_size();
        let instance = wgpu::Instance::new(BACKEND_BITS);
        let surface = unsafe { instance.create_surface(window) };
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: Some(&surface),
            })
            .await
            .unwrap();
        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: Some("Device"),
                    features: wgpu::Features::empty(),
                    limits: wgpu::Limits::default(),
                },
                None,
            )
            .await
            .unwrap();

        let swap_chain_descriptor = wgpu::SwapChainDescriptor {
            usage: wgpu::TextureUsage::RENDER_ATTACHMENT,
            format: adapter.get_swap_chain_preferred_format(&surface).unwrap(),
            width: size.width,
            height: size.height,
            present_mode: wgpu::PresentMode::Mailbox,
        };
        let swap_chain = device.create_swap_chain(&surface, &swap_chain_descriptor);

        let shader_module = device.create_shader_module(&wgpu::ShaderModuleDescriptor {
            label: Some("Shader Module"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shader.wgsl").into()),
            flags: wgpu::ShaderFlags::all(),
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Pipeline Layout"),
            bind_group_layouts: &[],
            push_constant_ranges: &[],
        });

        let vertex_descriptor = wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex>() as u64,
            step_mode: wgpu::InputStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32x3,
                    offset: 0,
                    shader_location: 0,
                },
                wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32x4,
                    offset: std::mem::size_of::<[f32; 3]>() as u64,
                    shader_location: 1,
                },
            ],
        };

        let paddle_vertex_buffer_raw = bytemuck::cast_slice(PADDLE_VERTICES);
        let paddle_vertex_buffer = wgpu::util::DeviceExt::create_buffer_init(
            &device,
            &wgpu::util::BufferInitDescriptor {
                label: Some("Vertex Buffer"),
                contents: paddle_vertex_buffer_raw,
                usage: wgpu::BufferUsage::VERTEX,
            },
        );

        let ball_vertex_buffer_raw = bytemuck::cast_slice(BALL_VERTICES);
        let ball_vertex_buffer = wgpu::util::DeviceExt::create_buffer_init(
            &device,
            &wgpu::util::BufferInitDescriptor {
                label: Some("Vertex Buffer"),
                contents: ball_vertex_buffer_raw,
                usage: wgpu::BufferUsage::VERTEX,
            },
        );

        let ball_transform_data = Vector2::new(0.0, 0.0);
        let ball_transform_data_raw = [ball_transform_data.x, ball_transform_data.y, 0.0];
        let ball_transform_buffer_raw = bytemuck::cast_slice(&ball_transform_data_raw);
        let ball_transform_buffer = wgpu::util::DeviceExt::create_buffer_init(
            &device,
            &wgpu::util::BufferInitDescriptor {
                label: Some("Instance Buffer"),
                contents: ball_transform_buffer_raw,
                usage: wgpu::BufferUsage::VERTEX,
            },
        );

        let rectangle_index_buffer_raw = bytemuck::cast_slice(RECTANGLE_INDICES);
        let rectangle_index_buffer = wgpu::util::DeviceExt::create_buffer_init(
            &device,
            &wgpu::util::BufferInitDescriptor {
                label: Some("Index Buffer"),
                contents: rectangle_index_buffer_raw,
                usage: wgpu::BufferUsage::INDEX,
            },
        );

        let instance_positions_descriptor = wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<PaddleRaw>() as u64,
            step_mode: wgpu::InputStepMode::Instance,
            attributes: &[wgpu::VertexAttribute {
                format: wgpu::VertexFormat::Float32x3,
                offset: 0,
                shader_location: 2,
            }],
        };

        let paddle_instance_data = [
            PaddleRaw {
                position: [-0.9, 0.0, 0.0],
            },
            PaddleRaw {
                position: [0.9, 0.0, 0.0],
            },
        ];

        let paddle_instance_buffer_raw = bytemuck::cast_slice(&paddle_instance_data);
        let paddle_instance_buffer = wgpu::util::DeviceExt::create_buffer_init(
            &device,
            &wgpu::util::BufferInitDescriptor {
                label: Some("Instance Buffer"),
                contents: paddle_instance_buffer_raw,
                usage: wgpu::BufferUsage::VERTEX,
            },
        );

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader_module,
                entry_point: "main",
                buffers: &[vertex_descriptor, instance_positions_descriptor],
            },
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                clamp_depth: false,
                polygon_mode: wgpu::PolygonMode::Fill,
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader_module,
                entry_point: "main",
                targets: &[wgpu::ColorTargetState {
                    format: swap_chain_descriptor.format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrite::ALL,
                }],
            }),
        });

        PongState {
            wgpu_state: WgpuState {
                surface,
                device,
                queue,
                swap_chain_descriptor,
                swap_chain,
                render_pipeline,
            },
            paddle_vertex_buffer,
            paddle_instance_data,
            paddle_instance_buffer,
            ball_vertex_buffer,
            ball_transform_data,
            ball_transform_buffer,
            ball_direction: cgmath::Deg(80.0),
            rectangle_index_buffer,
            pressed_keycodes: Vec::new(),
        }
    }

    fn resize(&mut self, size: winit::dpi::PhysicalSize<u32>) {
        let wgpu_state = &mut self.wgpu_state;
        wgpu_state.swap_chain_descriptor.width = size.width;
        wgpu_state.swap_chain_descriptor.height = size.height;
        wgpu_state.swap_chain = wgpu_state
            .device
            .create_swap_chain(&wgpu_state.surface, &wgpu_state.swap_chain_descriptor);
    }

    fn update(&mut self, delta_time: std::time::Duration) {
        let mut changed = false;

        let movement_amount = delta_time.as_millis() as f32 * 0.002;

        if self
            .pressed_keycodes
            .contains(&winit::event::VirtualKeyCode::Up)
        {
            self.paddle_instance_data[1].position[1] += movement_amount;
            changed = true;
        }
        if self
            .pressed_keycodes
            .contains(&winit::event::VirtualKeyCode::Down)
        {
            self.paddle_instance_data[1].position[1] -= movement_amount;
            changed = true;
        }
        if self
            .pressed_keycodes
            .contains(&winit::event::VirtualKeyCode::W)
        {
            self.paddle_instance_data[0].position[1] += movement_amount;
            changed = true;
        }
        if self
            .pressed_keycodes
            .contains(&winit::event::VirtualKeyCode::S)
        {
            self.paddle_instance_data[0].position[1] -= movement_amount;
            changed = true;
        }

        self.paddle_instance_data[1].position[1] =
            self.paddle_instance_data[1].position[1].clamp(-0.85, 0.85);
        self.paddle_instance_data[0].position[1] =
            self.paddle_instance_data[0].position[1].clamp(-0.85, 0.85);

        if changed {
            let paddle_instance_buffer_raw = bytemuck::cast_slice(&self.paddle_instance_data);
            self.paddle_instance_buffer = wgpu::util::DeviceExt::create_buffer_init(
                &self.wgpu_state.device,
                &wgpu::util::BufferInitDescriptor {
                    label: Some("Instance Buffer"),
                    contents: paddle_instance_buffer_raw,
                    usage: wgpu::BufferUsage::VERTEX,
                },
            );
        }

        let x_change = cgmath::Angle::sin(self.ball_direction) * 0.01;
        let y_change = cgmath::Angle::cos(self.ball_direction) * 0.01;
        self.ball_transform_data += cgmath::Vector2::new(x_change, y_change);
        let ball_transform_data_raw = [self.ball_transform_data.x, self.ball_transform_data.y, 0.0];
        let ball_transform_buffer_raw = bytemuck::cast_slice(&ball_transform_data_raw);
        self.ball_transform_buffer = wgpu::util::DeviceExt::create_buffer_init(
            &self.wgpu_state.device,
            &wgpu::util::BufferInitDescriptor {
                label: Some("Instance Buffer"),
                contents: ball_transform_buffer_raw,
                usage: wgpu::BufferUsage::VERTEX,
            },
        );

        if self.ball_transform_data.y > 0.975 || self.ball_transform_data.y < -0.975 {
            self.ball_direction.0 = 180.0 - self.ball_direction.0;
            PongState::randomize_direction(&mut self.ball_direction);
        }

        let ball_left = self.ball_transform_data.x - 0.025;
        let ball_right = self.ball_transform_data.x + 0.025;
        let ball_top = self.ball_transform_data.y + 0.025;
        let ball_bottom = self.ball_transform_data.y - 0.025;

        for paddle_raw in self.paddle_instance_data {
            if ball_left < paddle_raw.position[0] + 0.025
                && ball_right > paddle_raw.position[0] - 0.025
                && ball_bottom < paddle_raw.position[1] + 0.15
                && ball_top > paddle_raw.position[1] - 0.15
            {
                self.ball_direction = -self.ball_direction;
                PongState::randomize_direction(&mut self.ball_direction);
            }
        }
    }

    fn randomize_direction(direction: &mut Deg<f32>) {
        direction.0 += (rand::thread_rng().gen::<f32>() - 0.5) * 20.0;
    }

    fn input(&mut self, input: winit::event::KeyboardInput) {
        if let Some(keycode) = input.virtual_keycode {
            if input.state == winit::event::ElementState::Pressed
                && !self.pressed_keycodes.contains(&keycode)
            {
                self.pressed_keycodes.push(keycode);
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
        let wgpu_state = &self.wgpu_state;
        let frame = wgpu_state.swap_chain.get_current_frame().unwrap();

        let mut command_encoder =
            wgpu_state
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("Command Encoder"),
                });

        {
            let mut render_pass = command_encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[wgpu::RenderPassColorAttachment {
                    view: &frame.output.view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.0,
                            g: 0.0,
                            b: 0.0,
                            a: 1.0,
                        }),
                        store: true,
                    },
                }],
                depth_stencil_attachment: None,
            });

            render_pass.set_pipeline(&wgpu_state.render_pipeline);
            render_pass.set_vertex_buffer(0, self.paddle_vertex_buffer.slice(..));
            render_pass.set_vertex_buffer(1, self.paddle_instance_buffer.slice(..));
            render_pass.set_index_buffer(
                self.rectangle_index_buffer.slice(..),
                wgpu::IndexFormat::Uint16,
            );
            render_pass.draw_indexed(0..RECTANGLE_INDICES.len() as u32, 0, 0..2);

            render_pass.set_vertex_buffer(0, self.ball_vertex_buffer.slice(..));
            render_pass.set_vertex_buffer(1, self.ball_transform_buffer.slice(..));
            render_pass.draw_indexed(0..RECTANGLE_INDICES.len() as u32, 0, 0..1);
        }

        wgpu_state
            .queue
            .submit(std::iter::once(command_encoder.finish()));
    }
}

async fn run(event_loop: winit::event_loop::EventLoop<()>, window: winit::window::Window) {
    let mut state = PongState::new(&window).await;

    let mut previous_frame_time = instant::Instant::now();

    event_loop.run(move |event, _, control_flow| match event {
        winit::event::Event::WindowEvent { event, .. } => match event {
            winit::event::WindowEvent::CloseRequested => {
                *control_flow = winit::event_loop::ControlFlow::Exit
            }
            winit::event::WindowEvent::Resized(size) => state.resize(size),
            winit::event::WindowEvent::KeyboardInput { input, .. } => state.input(input),
            _ => {}
        },
        winit::event::Event::MainEventsCleared => window.request_redraw(),
        winit::event::Event::RedrawRequested(_) => {
            let frame_time = instant::Instant::now();
            let delta_time = frame_time - previous_frame_time;
            state.update(delta_time);
            state.render();
            previous_frame_time = frame_time;
        }
        _ => {}
    });
}

fn main() {
    let event_loop = winit::event_loop::EventLoop::new();
    let window = winit::window::Window::new(&event_loop).unwrap();
    #[cfg(not(target_arch = "wasm32"))]
    {
        env_logger::init();
        pollster::block_on(run(event_loop, window));
    }
    #[cfg(target_arch = "wasm32")]
    {
        std::panic::set_hook(Box::new(console_error_panic_hook::hook));
        console_log::init().expect("could not initialize logger");
        use winit::platform::web::WindowExtWebSys;
        // On wasm, append the canvas to the document body
        web_sys::window()
            .and_then(|win| win.document())
            .and_then(|doc| doc.body())
            .and_then(|body| {
                body.append_child(&web_sys::Element::from(window.canvas()))
                    .ok()
            })
            .expect("couldn't append canvas to document body");
        wasm_bindgen_futures::spawn_local(run(event_loop, window));
    }
}
