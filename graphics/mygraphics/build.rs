use cargo_gpu::Install;
use cargo_gpu::spirv_builder::{MetadataPrintout, ShaderPanicStrategy, SpirvMetadata};
use std::path::PathBuf;

pub fn main() -> anyhow::Result<()> {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let crate_path = [manifest_dir, "..", "mygraphics-shaders"]
        .iter()
        .copied()
        .collect::<PathBuf>();

    let install = Install::from_shader_crate(crate_path.clone()).run()?;
    let mut builder = install.to_spirv_builder(crate_path, "spirv-unknown-vulkan1.3");
    builder.print_metadata = MetadataPrintout::DependencyOnly;
    builder.shader_panic_strategy = ShaderPanicStrategy::SilentExit;
    builder.spirv_metadata = SpirvMetadata::Full;

    let compile_result = builder.build()?;
    let spv_path = compile_result.module.unwrap_single();
    println!("cargo::rustc-env=SHADER_SPV_PATH={}", spv_path.display());
    Ok(())
}
