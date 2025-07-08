use fluentbase_build::{build_with_args, Artifact, BuildArgs};
use std::path::PathBuf;

fn main() {
    println!("cargo:warning=Build script started");

    // std::env::set_var("FLUENT_DOCKER_IMAGE", "fluentbase:local");

    build_with_args(
        ".",
        BuildArgs {
            contract_name: Some("PowerCalculator.wasm".to_string()),
            docker: true,
            tag: "v0.3.4-dev".to_string(),
            mount_dir: Some(PathBuf::from("./")),
            output: Some(PathBuf::from("out")),
            generate: vec![
                Artifact::Metadata,
                Artifact::Rwasm,
                Artifact::Wat,
                Artifact::Solidity,
                Artifact::Abi,
            ],
            wasm_opt: true,
            locked: true,
            ..Default::default()
        },
    );

    println!("cargo:warning=Build script completed");
}
