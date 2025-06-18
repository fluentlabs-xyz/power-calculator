use fluentbase_build::{build_with_args, Artifact, BuildArgs};
use std::{path::PathBuf};

fn main() {
    println!("cargo:warning=Build script started");

    std::env::set_var("FLUENT_DOCKER_IMAGE", "fluentbase-build:v2.0.4-dev");

    build_with_args(
        ".",
        BuildArgs {
            contract_name: Some("PowerCalculator.wasm".to_string()),
            docker: true,
            mount_dir: Some(PathBuf::from("./")),
            output: Some(PathBuf::from("out")),
            generate: vec![
                Artifact::Abi,
                Artifact::Rwasm,
                Artifact::Wat,
                Artifact::Solidity,
                Artifact::Metadata,
            ],
            wasm_opt: false,
            locked: true,
            ..Default::default()
        },
    );

    println!("cargo:warning=Build script completed");
}
