# Your first wgpu app
An adaptation of [Your first WebGPU app](https://codelabs.developers.google.com/your-first-webgpu-app) in Rust with wgpu.

After each chapter the sources have been commited. So the git history contains all the intermeadiate programs.

- [Initialize WebGPU](https://github.com/IsaacDynamo/your-first-wgpu-app/blob/6ecc5a4/src/main.rs)
- [Draw geometry](https://github.com/IsaacDynamo/your-first-wgpu-app/blob/0a11a69/src/main.rs)
- [Draw a grid](https://github.com/IsaacDynamo/your-first-wgpu-app/blob/57ce4a3/src/main.rs)
- [Extra credit](https://github.com/IsaacDynamo/your-first-wgpu-app/blob/e4831c02/src/main.rs)
- [Manage cell state](https://github.com/IsaacDynamo/your-first-wgpu-app/blob/d0f477d/src/main.rs)

## Initialize WebGPU
Used a wgpu [example](https://github.com/gfx-rs/wgpu/blob/trunk/wgpu/examples/hello-triangle/main.rs) to get the missing pieces needed in 'Initialize WebGPU'.

## Draw geometry
[bytemuck](https://github.com/Lokathor/bytemuck) 'This lets you cast a slice of color values into a slice of u8 and send it to the GPU, or things like that.'

## Manage cell state
Used a winit [example](https://github.com/rust-windowing/winit/blob/master/examples/timer.rs) to create the 200ms render loop.
