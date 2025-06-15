//! build.rs — детерминированная сборка WASM + упаковка артефактов

use std::{
    env,
    fs::{self, File},
    io::Write,
    path::PathBuf,
    process::Command,
};

use cargo_metadata::MetadataCommand;
use chrono::Utc;
use fluentbase_build::{
    copy_wasm_and_wat, generate_build_output_file, rust_to_wasm as original_rust_to_wasm, wasm_to_wasmtime, RustToWasmConfig
};
use fluentbase_types::compile_wasm_to_rwasm;
use cargo_metadata::{TargetKind,CrateType, Metadata};

pub fn rust_to_wasm(config: RustToWasmConfig) -> PathBuf {
    let cargo_manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let cargo_manifest_path = PathBuf::from(cargo_manifest_dir.clone()).join("Cargo.toml");
    let mut metadata_cmd = MetadataCommand::new();
    let metadata = metadata_cmd
        .manifest_path(cargo_manifest_path)
        .exec()
        .unwrap();
    let target_dir: PathBuf = metadata.target_directory.clone().into();
    let target2_dir = target_dir.join("target2");

    let mut args = vec![
        "build".to_string(),
        "--target".to_string(),
        "wasm32-unknown-unknown".to_string(),
        "--release".to_string(),
        "--manifest-path".to_string(),
        format!("{}/Cargo.toml", cargo_manifest_dir.to_str().unwrap()),
        "--target-dir".to_string(),
        target2_dir.to_str().unwrap().to_string(),
        "--color=always".to_string(),
        "--locked".to_string()
    ];
    if config.no_default_features {
        args.push("--no-default-features".to_string());
    }
    if !config.features.is_empty() {
        args.push("--features".to_string());
        args.extend_from_slice(&config.features);
    }
    eprintln!("<<<DEBUG>>>: cargo manifest dir: {:?}", cargo_manifest_dir.display());
    let flags = [
        "-C".to_string(),
        format!("link-arg=-zstack-size={}", config.stack_size),
        "-C".to_string(),
        "panic=abort".to_string(),
        "-C".to_string(),
        "target-feature=+bulk-memory".to_string(),
        "-C".to_string(),
        "codegen-units=1".to_string(),
        "-C".to_string(),
        "incremental=false".to_string(),
        // remap common paths
        format!("--remap-path-prefix={}=/project", cargo_manifest_dir.display()),
        format!("--remap-path-prefix={}=/cargo", std::env::var("CARGO_HOME").unwrap_or_default()),
        format!("--remap-path-prefix={}=/rustup", std::env::var("RUSTUP_HOME").unwrap_or_default()),
    ];
    let flags = flags.join("\x1f");

    let status = Command::new("cargo")
        .env("CARGO_ENCODED_RUSTFLAGS", flags)
        .args(args)
        .status()
        .expect("WASM compilation failure: failed to run cargo build");

    if !status.success() {
        panic!(
            "WASM compilation failure: failed to run cargo build with code: {}",
            status.code().unwrap_or(1)
        );
    }

    let wasm_artifact_name = calc_wasm_artifact_name(&metadata);
    let wasm_artifact_path = target2_dir
        .join("wasm32-unknown-unknown")
        .join("release")
        .join(wasm_artifact_name);

    wasm_artifact_path
}


fn calc_wasm_artifact_name(metadata: &Metadata) -> String {
    let mut result = vec![];
    for program_crate in metadata.workspace_default_members.to_vec() {
        let program = metadata
            .packages
            .iter()
            .find(|p| p.id == program_crate)
            .unwrap_or_else(|| panic!("cannot find package for {}", program_crate));
        for bin_target in program.targets.iter() {
            let is_bin = bin_target.kind.contains(&TargetKind::Bin)
                && bin_target.crate_types.contains(&CrateType::Bin);
            let is_cdylib = bin_target.kind.contains(&TargetKind::CDyLib)
                && bin_target.crate_types.contains(&CrateType::CDyLib);
            // Both `bin` and `cdylib` crates produce a `.wasm` file
            if is_cdylib || is_bin {
                let bin_name = bin_target.name.clone() + ".wasm";
                result.push(bin_name);
            }
        }
    }
    if result.is_empty() {
        panic!(
            "No WASM artifact found to build in package `{}`. Ensure the package defines exactly one `bin` or `cdylib` crate.",
            metadata.workspace_members.first().unwrap()
        );
    } else if result.len() > 1 {
        panic!(
            "Multiple WASM artifacts found in package `{}`. Ensure the package defines exactly one `bin` or `cdylib` crate.",
            metadata.workspace_members.first().unwrap()
        );
    }
    result.first().unwrap().clone()
}



/// wasm → rwasm
pub fn wasm_to_rwasm(wasm: &PathBuf) -> PathBuf {
    let bytes = fs::read(wasm).unwrap();
    let rwasm = compile_wasm_to_rwasm(&bytes).unwrap().rwasm_bytecode.to_vec();
    let out = PathBuf::from(env::var("OUT_DIR").unwrap()).join("lib.rwasm");
    fs::write(&out, &rwasm).unwrap();
    out
}
fn main() {
    // внутриво-wasm сборка этого build.rs не нужна
    if env::var("TARGET").unwrap() == "wasm32-unknown-unknown" {
        return;
    }

    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let arch = if cfg!(target_arch = "x86_64") { "x86" } else { "arm" };
    let ts   = Utc::now().format("%Y%m%dT%H%M%S").to_string();
    let artifact_dir = manifest_dir.join("artifacts").join(arch).join(&ts);
    fs::create_dir_all(&artifact_dir).unwrap();

    /* ───── сборка deterministic-wasm и производных ───── */
    let wasm_path  = rust_to_wasm(RustToWasmConfig::default());
    copy_wasm_and_wat(&wasm_path);
    let rwasm_path = wasm_to_rwasm(&wasm_path);
    let cwasm_path = wasm_to_wasmtime(&wasm_path);
    generate_build_output_file(&wasm_path, &rwasm_path, &cwasm_path);

    // ───── strip → lib.stripped.wasm и lib.stripped.wat ─────
    let _ = Command::new("wasm-tools")
        .args(["strip", "-a", "lib.wasm", "-o", "lib.stripped.wasm"])
        .current_dir(&manifest_dir)
        .status()
        .expect("failed to strip wasm");

    let _ = Command::new("wasm2wat")
        .args(["lib.stripped.wasm", "-o", "lib.stripped.wat"])
        .current_dir(&manifest_dir)
        .status()
        .expect("failed to convert stripped wasm to wat");

    /* ───── перенос + расчёт sha256 на лету ───── */
    let mut info = String::new();
    for (src, name) in [
        (manifest_dir.join("lib.wasm"),         "lib.wasm"),
        (manifest_dir.join("lib.wat"),          "lib.wat"),
        (manifest_dir.join("lib.stripped.wasm"),"lib.stripped.wasm"),
        (manifest_dir.join("lib.stripped.wat"), "lib.stripped.wat"),
        (rwasm_path.clone(),                    "lib.rwasm"),
        (cwasm_path.clone(),                    "lib.cwasm"),
    ] {
        let dst = artifact_dir.join(name);
        if src.exists() {
            fs::copy(&src, &dst).unwrap();
            if let Ok(bytes) = fs::read(&dst) {
                let hash = sha256::digest(bytes);
                info.push_str(&format!("sha256({}): {}\n", name, hash));
            }
        }
    }

    /* ───── мета-информация о сборке ───── */
    if let Ok(out) = Command::new("git").args(["rev-parse", "HEAD"]).output() {
        if out.status.success() {
            info.push_str(&format!(
                "commit: {}\n",
                String::from_utf8_lossy(&out.stdout).trim()
            ));
        }
    }
    if let Ok(out) = Command::new("rustc").arg("--version").output() {
        info.push_str(&format!("rustc: {}\n", String::from_utf8_lossy(&out.stdout).trim()));
    }
    if let Ok(out) = Command::new("cargo").arg("--version").output() {
        info.push_str(&format!("cargo: {}\n", String::from_utf8_lossy(&out.stdout).trim()));
    }
    info.push_str(&format!("target: {}\n", env::var("TARGET").unwrap()));
    info.push_str(&format!("build_time: {}\n", Utc::now().to_rfc3339()));

    /* ───── сохраняем BUILD-INFO.md ───── */
    File::create(artifact_dir.join("BUILD-INFO.md"))
        .and_then(|mut f| f.write_all(info.as_bytes()))
        .unwrap();
}
