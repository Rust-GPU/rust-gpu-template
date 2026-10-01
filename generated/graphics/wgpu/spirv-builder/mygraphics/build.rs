use spirv_builder::{ShaderPanicStrategy, SpirvBuilder, SpirvMetadata};
use std::path::PathBuf;

pub fn main() -> anyhow::Result<()> {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let crate_path = [manifest_dir, "..", "mygraphics-shaders"]
        .iter()
        .copied()
        .collect::<PathBuf>();

    let target = "spirv-unknown-naga-wgsl";
    let mut builder = SpirvBuilder::new(crate_path, target);
    builder.build_script.defaults = true;
    builder.shader_panic_strategy = ShaderPanicStrategy::SilentExit;
    builder.spirv_metadata = SpirvMetadata::Full;

    let compile_result = builder.build()?;
    let shader_path = compile_result.module.unwrap_single();

    println!("cargo::rustc-env=SHADER_WGSL_PATH={}", shader_path.display());
    Ok(())
}
