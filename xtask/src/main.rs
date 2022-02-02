use devx_cmd::run;
use khonsu_tools::universal::{
    anyhow,
    clap::{self, Parser},
    DefaultConfig,
};

#[derive(Debug, Parser)]
enum Args {
    CompileShaders,
    #[clap(flatten)]
    Tools(khonsu_tools::Commands),
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    match args {
        Args::CompileShaders => compile_shaders()?,
        Args::Tools(command) => command.execute::<Config>()?,
    }
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

struct Config;

impl khonsu_tools::Config for Config {
    type Publish = Self;
    type Universal = Self;
}

impl khonsu_tools::publish::Config for Config {
    fn paths() -> Vec<String> {
        vec![
            String::from("core"),
            String::from("app"),
            String::from("kludgine"),
        ]
    }
}

impl khonsu_tools::universal::Config for Config {
    type Audit = DefaultConfig;
    type CodeCoverage = Self;
}

impl khonsu_tools::universal::code_coverage::Config for Config {
    fn ignore_paths() -> Vec<String> {
        vec![String::from("kludgine/examples/*")]
    }
}
