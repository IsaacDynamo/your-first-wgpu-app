use winit::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::Window,
};

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
    let pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
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
