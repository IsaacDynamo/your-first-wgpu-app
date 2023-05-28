use std::time::{Duration, Instant};

use rand::prelude::Distribution;
use winit::{
    event::{Event, StartCause, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::Window,
};

const GRID_SIZE: usize = 32;
const WORKGROUP_SIZE: usize = 8;

fn byte_length<T>(vec: &Vec<T>) -> u64 {
    (vec.len() * std::mem::size_of::<T>()) as u64
}

async fn run(event_loop: EventLoop<()>, window: Window) {
    let instance = wgpu::Instance::default();

    // Surface is unique to the Rust API of wgpu. In the WebGPU specification, GPUCanvasContext serves a similar role.
    // Source: https://docs.rs/wgpu/latest/wgpu/struct.Surface.html
    //
    // ```js
    // const context = canvas.getContext("webgpu");
    // ```
    let surface = unsafe { instance.create_surface(&window) }.expect("");

    // ```js
    // const adapter = await navigator.gpu.requestAdapter();
    // if (!adapter) {
    //     throw new Error("No appropriate GPUAdapter found.");
    // }
    // ```
    let options = wgpu::RequestAdapterOptions {
        compatible_surface: Some(&surface),
        ..Default::default()
    };
    let adapter = instance
        .request_adapter(&options)
        .await
        .expect("No appropriate adapter found");

    // ```js
    // const device = await adapter.requestDevice();
    // ```
    let desc = wgpu::DeviceDescriptor::default();
    let (device, queue) = adapter
        .request_device(&desc, None)
        .await
        .expect("Device request failed");

    // ```js
    // const canvasFormat = navigator.gpu.getPreferredCanvasFormat();
    // context.configure({
    //     device: device,
    //     format: canvasFormat,
    // });
    // ```
    let size = window.inner_size();
    let config = surface
        .get_default_config(&adapter, size.width, size.height)
        .expect("No default surface config");
    surface.configure(&device, &config);

    let uniform_array = vec![GRID_SIZE as f32, GRID_SIZE as f32];
    let uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("Grid Uniforms"),
        size: byte_length(&uniform_array),
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false, // WebGPU defaults to false `boolean mappedAtCreation = false;`
    });

    queue.write_buffer(&uniform_buffer, 0, bytemuck::cast_slice(&uniform_array));

    #[rustfmt::skip]
    let vertices: Vec<f32> = vec![
        // X,   Y
        -0.8, -0.8, // Triangle 1
         0.8, -0.8,
         0.8,  0.8,
        -0.8, -0.8, // Triangle 2
         0.8,  0.8,
        -0.8,  0.8,
    ];

    let vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("Cell vertices"),
        size: byte_length(&vertices),
        usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false, // WebGPU defaults to false `boolean mappedAtCreation = false;`
    });

    queue.write_buffer(&vertex_buffer, 0, bytemuck::cast_slice(&vertices));

    let vertex_buffer_layout = wgpu::VertexBufferLayout {
        array_stride: 8,
        step_mode: wgpu::VertexStepMode::Vertex, // WebGPU defaults to `GPUVertexStepMode stepMode = "vertex";`
        attributes: &[wgpu::VertexAttribute {
            format: wgpu::VertexFormat::Float32x2,
            offset: 0,
            shader_location: 0,
        }],
    };

    // Create an array representing the active state of each cell.
    let mut cell_state_array = vec![0u32; GRID_SIZE * GRID_SIZE];

    // Create two storage buffers to hold the cell state.
    let cell_state_storage = [
        device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Cell State A"),
            size: byte_length(&cell_state_array),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false, // WebGPU defaults to false `boolean mappedAtCreation = false;`
        }),
        device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Cell State B"),
            size: byte_length(&cell_state_array),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false, // WebGPU defaults to false `boolean mappedAtCreation = false;`
        }),
    ];

    // Set each cell to a random state, then copy the array into the storage buffer.
    let mut rng = rand::thread_rng();
    let dist = rand::distributions::Bernoulli::new(0.6).unwrap();
    for cell in cell_state_array.iter_mut() {
        *cell = dist.sample(&mut rng) as u32;
    }
    queue.write_buffer(
        &cell_state_storage[0],
        0,
        bytemuck::cast_slice(&cell_state_array),
    );

    let cell_shader_module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("Cell shader"),
        source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed(
            "
            struct VertexInput {
                @location(0) pos: vec2f,
                @builtin(instance_index) instance: u32,
            };

            struct VertexOutput {
                @builtin(position) pos: vec4f,
                @location(0) cell: vec2f,
            };

            @group(0) @binding(0) var<uniform> grid: vec2f;
            @group(0) @binding(1) var<storage> cell_state: array<u32>; 

            @vertex
            fn vertexMain(input: VertexInput) -> VertexOutput {

                let i = f32(input.instance);
                let cell = vec2f(i % grid.x, floor(i / grid.x));
                let state = f32(cell_state[input.instance]);

                let cell_offset = cell / grid * 2.0;
                let grid_pos = (input.pos * state + 1.0) / grid - 1.0 + cell_offset;

                var output: VertexOutput;
                output.pos = vec4f(grid_pos, 0.0, 1.0);
                output.cell = cell;
                return output;
            }

            @fragment
            fn fragmentMain(input: VertexOutput) -> @location(0) vec4f {
                let c = input.cell / grid;
                return vec4f(c, 1.0-c.x, 1.0);
            }
        ",
        )),
    });

    // Create the compute shader that will process the simulation.
    let simulation_shader_module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("Game of Life simulation shader"),
        source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Owned(
            "
            @group(0) @binding(0) var<uniform> grid: vec2f;
            @group(0) @binding(1) var<storage> cell_state_in: array<u32>;
            @group(0) @binding(2) var<storage, read_write> cell_state_out: array<u32>;

            fn cell_index(cell: vec2<i32>) -> u32 {
                return u32(
                    ((cell.y + i32(grid.y)) % i32(grid.y)) * i32(grid.x) +
                    ((cell.x + i32(grid.x)) % i32(grid.x))
                );
            }

            fn cell_active(x: i32, y: i32) -> u32 {
                return cell_state_in[cell_index(vec2(x, y))];
            }

            @compute
            @workgroup_size(${WORKGROUP_SIZE},${WORKGROUP_SIZE})
            fn computeMain(@builtin(global_invocation_id) cell: vec3u) {

                let cell = vec2i(cell.xy); 

                // Determine how many active neighbors this cell has.
                let active_neighbors = cell_active(cell.x + 1, cell.y + 1) +
                                       cell_active(cell.x + 1, cell.y) +
                                       cell_active(cell.x + 1, cell.y - 1) +
                                       cell_active(cell.x,     cell.y - 1) +
                                       cell_active(cell.x - 1, cell.y - 1) +
                                       cell_active(cell.x - 1, cell.y) +
                                       cell_active(cell.x - 1, cell.y + 1) +
                                       cell_active(cell.x,     cell.y + 1);

                let i = cell_index(cell);

                // Conway's game of life rules:
                switch active_neighbors {
                    case 2u: { // Active cells with 2 neighbors stay active.
                        cell_state_out[i] = cell_state_in[i];
                    }
                    case 3u: { // Cells with 3 neighbors become or stay active.
                        cell_state_out[i] = 1u;
                    }
                    default: { // Cells with < 2 or > 3 neighbors become inactive.
                        cell_state_out[i] = 0u;
                    }
                }
            }
        "
            .to_string()
            .replace("${WORKGROUP_SIZE}", &format!("{WORKGROUP_SIZE}")),
        )),
    });

    // Create the bind group layout and pipeline layout.
    let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("Cell Bind Group Layout"),
        entries: &[
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX
                    | wgpu::ShaderStages::COMPUTE
                    | wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 2,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: false },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
        ],
    });

    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("Cell Pipeline Layout"),
        bind_group_layouts: &[&bind_group_layout],
        push_constant_ranges: &[],
    });

    let swapchain_capabilities = surface.get_capabilities(&adapter);
    let swapchain_format = swapchain_capabilities.formats[0];

    let cell_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("Cell pipeline"),
        vertex: wgpu::VertexState {
            module: &cell_shader_module,
            entry_point: "vertexMain",
            buffers: &[vertex_buffer_layout],
        },
        fragment: Some(wgpu::FragmentState {
            module: &cell_shader_module,
            entry_point: "fragmentMain",
            targets: &[Some(swapchain_format.into())],
        }),
        layout: Some(&pipeline_layout),
        primitive: wgpu::PrimitiveState::default(),
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
        multiview: None,
    });

    let bind_group = [
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Cell renderer bind group A"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::Buffer(
                        uniform_buffer.as_entire_buffer_binding(),
                    ),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Buffer(
                        cell_state_storage[0].as_entire_buffer_binding(),
                    ),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Buffer(
                        cell_state_storage[1].as_entire_buffer_binding(),
                    ),
                },
            ],
        }),
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Cell renderer bind group B"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::Buffer(
                        uniform_buffer.as_entire_buffer_binding(),
                    ),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Buffer(
                        cell_state_storage[1].as_entire_buffer_binding(),
                    ),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Buffer(
                        cell_state_storage[0].as_entire_buffer_binding(),
                    ),
                },
            ],
        }),
    ];

    // Create a compute pipeline that updates the game state.
    let simulation_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: Some("Simulation pipeline"),
        layout: Some(&pipeline_layout),
        module: &simulation_shader_module,
        entry_point: "computeMain",
    });

    const UPDATE_INTERVAL: Duration = Duration::new(0, 200_000_000);
    let mut step = 0;

    event_loop.run(move |event, _, control_flow| {
        // Have the closure take ownership of the resources.
        // `event_loop.run` never returns, therefore we must do this to ensure
        // the resources are properly cleaned up.
        let _ = (&instance, &adapter, &cell_shader_module);

        match event {
            Event::NewEvents(StartCause::Init) => {
                control_flow.set_wait_until(Instant::now() + UPDATE_INTERVAL);
            }
            Event::NewEvents(StartCause::ResumeTimeReached { .. }) => {
                control_flow.set_wait_until(Instant::now() + UPDATE_INTERVAL);

                // Slow render loop

                // ```js
                // const encoder = device.createCommandEncoder();
                // ```
                let mut encoder =
                    device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());

                let mut compute_pass =
                    encoder.begin_compute_pass(&wgpu::ComputePassDescriptor::default());

                compute_pass.set_pipeline(&simulation_pipeline);
                compute_pass.set_bind_group(0, &bind_group[step], &[]);

                let workgroup_count = (GRID_SIZE / WORKGROUP_SIZE) as u32;
                compute_pass.dispatch_workgroups(workgroup_count, workgroup_count, 1);

                drop(compute_pass);

                // increment step
                step = (step + 1) % 2;

                // ```js
                // const pass = encoder.beginRenderPass({
                //     colorAttachments: [{
                //         view: context.getCurrentTexture().createView(),
                //         loadOp: "clear",
                //         clearValue: { r: 0, g: 0, b: 0.4, a: 1 }, // New line
                //         storeOp: "store",
                //     }],
                // });
                // ```
                let frame = surface
                    .get_current_texture()
                    .expect("Current texture not found");
                let view = frame
                    .texture
                    .create_view(&wgpu::TextureViewDescriptor::default());
                let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: None,
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color {
                                r: 0.0,
                                g: 0.0,
                                b: 0.4,
                                a: 1.0,
                            }),
                            store: true,
                        },
                    })],
                    depth_stencil_attachment: None,
                });

                pass.set_pipeline(&cell_pipeline);
                pass.set_vertex_buffer(0, vertex_buffer.slice(..));
                pass.set_bind_group(0, &bind_group[step], &[]);
                let vs = (vertices.len() / 2) as u32;
                let is: u32 = (GRID_SIZE * GRID_SIZE) as u32;
                pass.draw(0..vs, 0..is);

                // ```js
                // pass.end()
                // ```
                drop(pass);

                // ```js
                // device.queue.submit([encoder.finish()]);
                // ```
                queue.submit(Some(encoder.finish()));

                // Present the the work that has been submitted into the queue
                frame.present();
            }
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => *control_flow = ControlFlow::Exit,
            _ => {}
        }
    });
}

fn main() {
    env_logger::init();

    const WINDOW_SIZE: u32 = 512;
    let event_loop = EventLoop::new();
    let window = winit::window::WindowBuilder::new()
        .with_title("Your first wgpu app")
        .with_inner_size(winit::dpi::PhysicalSize::new(WINDOW_SIZE, WINDOW_SIZE))
        .build(&event_loop)
        .unwrap();

    pollster::block_on(run(event_loop, window));
}
