use winit::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::Window,
};

const GRID_SIZE: usize = 32;

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

            @vertex
            fn vertexMain(input: VertexInput) -> VertexOutput {

                let i = f32(input.instance);
                let cell = vec2f(i % grid.x, floor(i / grid.x));
                let cell_offset = cell / grid * 2.0;
                let grid_pos = (input.pos + 1.0) / grid - 1.0 + cell_offset;

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
        layout: None,
        primitive: wgpu::PrimitiveState::default(),
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
        multiview: None,
    });

    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("Cell renderer bind group"),
        layout: &cell_pipeline.get_bind_group_layout(0),
        entries: &[wgpu::BindGroupEntry {
            binding: 0,
            resource: wgpu::BindingResource::Buffer(uniform_buffer.as_entire_buffer_binding()),
        }],
    });

    // ```js
    // const encoder = device.createCommandEncoder();
    // ```
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());

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
    pass.set_bind_group(0, &bind_group, &[]);
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

    event_loop.run(move |event, _, control_flow| {
        // Have the closure take ownership of the resources.
        // `event_loop.run` never returns, therefore we must do this to ensure
        // the resources are properly cleaned up.
        let _ = (&instance, &adapter);

        *control_flow = ControlFlow::Wait;
        match event {
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
