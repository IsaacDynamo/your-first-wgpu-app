# Your first wgpu app
An adaptation of [Your first WebGPU app](https://codelabs.developers.google.com/your-first-webgpu-app) in Rust with wgpu.

## Initialize WebGPU
Used a wgpu [example](https://github.com/gfx-rs/wgpu/blob/trunk/wgpu/examples/hello-triangle/main.rs) to get the missing pieces needed in 'Initialize WebGPU'.

## Draw geometry
[bytemuck](https://github.com/Lokathor/bytemuck) 'This lets you cast a slice of color values into a slice of u8 and send it to the GPU, or things like that.'