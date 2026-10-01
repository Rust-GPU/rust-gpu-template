# rust-gpu templates

Examples on how to setup rust-gpu with various APIs, use-cases and integration paths.

## Generating a project

Install our `cargo gpu` cli, if you haven't already:

```shell
cargo install cargo-gpu
```

Then generate a rust-gpu project with:

```shell
cargo gpu generate
```

(This subcommand is merely a wrapper around [`cargo-generate`](https://crates.io/crates/cargo-generate/))

If you don't want to install `cargo generate`, you can also go to the [`generated/` folder](https://github.com/Rust-GPU/rust-gpu-template/tree/main/generated/) and navigate its subfolders, each level corresponding to the questions below.

## Questions

1. **Which sub-template should be expanded?**

We currently only have a "graphics" template, showcasing how to use rust-gpu for vertex and fragment shaders. More templates to come!

2. **What API?**

You can choose between the high-level [wgpu](https://github.com/gfx-rs/wgpu) API with browser support and [ash](https://github.com/ash-rs/ash), a lightweight wrapper around the low level Vulkan API. If you're new to graphics, we recommend you start at [learn wgpu](https://sotrh.github.io/learn-wgpu/), and once you have a basic triangle or compute shader working, return here.

3. **How to integrate rust-gpu?**

[cargo-gpu](https://github.com/Rust-GPU/cargo-gpu) is a rust-gpu installation manager, which isolates the specific nightly toolchain that rust-gpu requires, thus allowing the rest of your project to remain on a stable toolchain (or any other toolchain). In addition, it is also a command line tool you can use. Whereas the "raw" [spirv-builder](https://github.com/Rust-GPU/rust-gpu/tree/main/crates/spirv-builder) setup requires your entire project to be compiled using the specific nightly rust-gpu toolchain. Note that cargo-gpu merely wraps spirv-builder, making it easy to switch and keep most of your configuration between both platforms.
