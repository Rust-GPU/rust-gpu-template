use cargo_gpu_install::install::Install;
use cargo_gpu_install::spirv_builder::{ShaderPanicStrategy, SpirvMetadata};
use std::path::PathBuf;

pub fn main() -> anyhow::Result<()> {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let crate_path = [manifest_dir, "..", "mygraphics-shaders"]
        .iter()
        .copied()
        .collect::<PathBuf>();

    let install = Install::from_shader_crate(crate_path.clone())
        .within_build_script()
        .run()?;
    let mut builder = install.to_spirv_builder(&crate_path, "");
    builder.build_script.defaults = true;
    builder.shader_panic_strategy = ShaderPanicStrategy::SilentExit;
    builder.spirv_metadata = SpirvMetadata::Full;

    // SPIR-V
    {
        builder.target = Some("spirv-unknown-vulkan1.3".to_owned());
        let compile_result = builder.build()?;
        println!(
            "cargo::rustc-env=SHADER_SPV_PATH={}",
            compile_result.module.unwrap_single().display()
        );
    }

    // wgsl
    {
        builder.target = Some("spirv-unknown-naga-wgsl".to_owned());
        let compile_result = builder.build()?;
        println!(
            "cargo::rustc-env=SHADER_WGSL_PATH={}",
            compile_result.module.unwrap_single().display()
        );
    }
    Ok(())
}
