use devx_cmd::run;
use khonsu_tools::{anyhow, code_coverage::CodeCoverage};
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
enum Args {
    CompileShaders,
    GenerateCodeCoverageReport,
}

fn main() -> anyhow::Result<()> {
    let args = Args::from_args();
    match args {
        Args::CompileShaders => compile_shaders()?,
        Args::GenerateCodeCoverageReport => CodeCoverage::<CodeCoverageConfig>::execute()?,
    };
    Ok(())
}

fn compile_shaders() -> Result<(), devx_cmd::Error> {
    println!("Building sprite fragment shader");
    run!(
        "glslc",
        "core/src/sprite/shaders/sprite.frag",
        "-o",
        "core/src/sprite/shaders/sprite.frag.spv",
    )?;
    println!("Building sprite vertex shader");
    run!(
        "glslc",
        "core/src/sprite/shaders/sprite.vert",
        "-o",
        "core/src/sprite/shaders/sprite.vert.spv",
    )?;
    println!("Building sprite SRGB vertex shader");
    run!(
        "glslc",
        "core/src/sprite/shaders/sprite-srgb.vert",
        "-o",
        "core/src/sprite/shaders/sprite-srgb.vert.spv",
    )?;

    Ok(())
}

struct CodeCoverageConfig;

impl khonsu_tools::code_coverage::Config for CodeCoverageConfig {
    fn ignore_paths() -> Vec<String> {
        vec![String::from("kludgine/examples/*")]
    }
}
