# Your first wgpu app
An adaptation of [Your first WebGPU app](https://codelabs.developers.google.com/your-first-webgpu-app) in Rust with wgpu.

After each chapter the sources have been committed. So the git history contains all the intermediate programs.

- [Initialize WebGPU](https://github.com/IsaacDynamo/your-first-wgpu-app/blob/6ecc5a4/src/main.rs)
- [Draw geometry](https://github.com/IsaacDynamo/your-first-wgpu-app/blob/0a11a69/src/main.rs)
- [Draw a grid](https://github.com/IsaacDynamo/your-first-wgpu-app/blob/57ce4a3/src/main.rs)
- [Extra credit](https://github.com/IsaacDynamo/your-first-wgpu-app/blob/e4831c02/src/main.rs)
- [Manage cell state](https://github.com/IsaacDynamo/your-first-wgpu-app/blob/d0f477d/src/main.rs)
- [Run the simulation](https://github.com/IsaacDynamo/your-first-wgpu-app/blob/e99f24c/src/main.rs)

## Initialize WebGPU
Used a wgpu [example](https://github.com/gfx-rs/wgpu/blob/trunk/wgpu/examples/hello-triangle/main.rs) to get the missing pieces needed in 'Initialize WebGPU'.

## Draw geometry
Referenced the example to see how it did the `Vec<f32>` into `[u8]` transform needed for `write_buffer()`. It used the [bytemuck](https://github.com/Lokathor/bytemuck) crate, that seem to be made for this according to its own description. 
> This lets you cast a slice of color values into a slice of u8 and send it to the GPU, or things like that.

## Manage cell state
Used a winit [example](https://github.com/rust-windowing/winit/blob/master/examples/timer.rs) to create the 200ms render loop.

## Run the simulation
Had to add `wgpu::ShaderStages::FRAGMENT` to visibility of the `grid` uniform in the bind group layout. wgpu was complaining rightfully so that `grid` is used in the fragment shader. This seems to be a bug in the tutorial.
