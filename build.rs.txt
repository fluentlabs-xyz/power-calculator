//! build.rs - Deterministic WASM build and artifact packaging

use std::{
   env,
   fs::{self, File},
   io::Write,
   path::PathBuf,
   process::Command,
};

use cargo_metadata::{CrateType, Metadata, MetadataCommand, TargetKind};
use chrono::Utc;
use fluentbase_build::{
   copy_wasm_and_wat, generate_build_output_file, rust_to_wasm as original_rust_to_wasm,
   wasm_to_wasmtime, RustToWasmConfig,
};
use fluentbase_types::compile_wasm_to_rwasm;

/// Compile Rust project to WASM with deterministic settings
pub fn rust_to_wasm(config: RustToWasmConfig) -> PathBuf {
   let manifest_dir = get_manifest_dir();
   let metadata = load_project_metadata(&manifest_dir);
   let target_dir = get_deterministic_target_dir(&metadata);

   // Build cargo arguments
   let mut args = build_cargo_args(&manifest_dir, &target_dir, &config);

   // Set deterministic RUSTFLAGS
   let rustflags = build_deterministic_rustflags(&manifest_dir);

   // Execute cargo build
   execute_cargo_build(args, rustflags);

   // Find and return WASM artifact path
   resolve_wasm_artifact_path(&metadata, &target_dir)
}

/// Get the cargo manifest directory
fn get_manifest_dir() -> PathBuf {
   PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR not set"))
}

/// Load project metadata using cargo_metadata
fn load_project_metadata(manifest_dir: &PathBuf) -> Metadata {
   let manifest_path = manifest_dir.join("Cargo.toml");
   MetadataCommand::new()
       .manifest_path(manifest_path)
       .exec()
       .expect("Failed to load cargo metadata")
}

/// Get deterministic target directory (separate from default)
fn get_deterministic_target_dir(metadata: &Metadata) -> PathBuf {
   let base_target_dir: PathBuf = metadata.target_directory.clone().into();
   base_target_dir.join("target2")
}

/// Build cargo command arguments
fn build_cargo_args(manifest_dir: &PathBuf, target_dir: &PathBuf, config: &RustToWasmConfig) -> Vec<String> {
   let mut args = vec![
       "build".to_string(),
       "--target".to_string(),
       "wasm32-unknown-unknown".to_string(),
       "--release".to_string(),
       "--manifest-path".to_string(),
       manifest_dir.join("Cargo.toml").display().to_string(),
       "--target-dir".to_string(),
       target_dir.display().to_string(),
       "--color=always".to_string(),
       "--locked".to_string(),
   ];

   if config.no_default_features {
       args.push("--no-default-features".to_string());
   }

   if !config.features.is_empty() {
       args.push("--features".to_string());
       args.push(config.features.join(","));
   }

   args
}

/// Build deterministic RUSTFLAGS for reproducible builds
fn build_deterministic_rustflags(manifest_dir: &PathBuf) -> String {
   let cargo_home = env::var("CARGO_HOME").unwrap_or_else(|_| "/cargo".to_string());
   let rustup_home = env::var("RUSTUP_HOME").unwrap_or_else(|_| "/rustup".to_string());

   let flags = vec![
       "-C".to_string(),
       format!("link-arg=-zstack-size={}", RustToWasmConfig::default().stack_size),
       "-C".to_string(),
       "panic=abort".to_string(),
       "-C".to_string(),
       "target-feature=+bulk-memory".to_string(),
       "-C".to_string(),
       "codegen-units=1".to_string(),
       "-C".to_string(),
       "incremental=false".to_string(),
       // Remap paths for reproducibility
       format!("--remap-path-prefix={}=/project", manifest_dir.display()),
       format!("--remap-path-prefix={}=/cargo", cargo_home),
       format!("--remap-path-prefix={}=/rustup", rustup_home),
   ];

   flags.join("\x1f")
}

/// Execute cargo build with specified arguments and flags
fn execute_cargo_build(args: Vec<String>, rustflags: String) {
   eprintln!("Building WASM with cargo...");
   
   let status = Command::new("cargo")
       .env("CARGO_ENCODED_RUSTFLAGS", rustflags)
       .args(args)
       .status()
       .expect("Failed to execute cargo build");

   if !status.success() {
       panic!(
           "WASM compilation failed with exit code: {}",
           status.code().unwrap_or(1)
       );
   }
}

/// Resolve the path to the generated WASM artifact
fn resolve_wasm_artifact_path(metadata: &Metadata, target_dir: &PathBuf) -> PathBuf {
   let artifact_name = determine_wasm_artifact_name(metadata);
   
   target_dir
       .join("wasm32-unknown-unknown")
       .join("release")
       .join(artifact_name)
}

/// Determine the name of the WASM artifact from metadata
fn determine_wasm_artifact_name(metadata: &Metadata) -> String {
   let mut artifacts = Vec::new();

   for workspace_member in &metadata.workspace_default_members {
       let package = metadata
           .packages
           .iter()
           .find(|p| p.id == *workspace_member)
           .unwrap_or_else(|| panic!("Cannot find package for {}", workspace_member));

       for target in &package.targets {
           if is_wasm_target(target) {
               artifacts.push(format!("{}.wasm", target.name));
           }
       }
   }

   match artifacts.len() {
       0 => panic!(
           "No WASM artifact found. Ensure the package defines a `bin` or `cdylib` crate."
       ),
       1 => artifacts.into_iter().next().unwrap(),
       _ => panic!(
           "Multiple WASM artifacts found. Ensure the package defines exactly one `bin` or `cdylib` crate."
       ),
   }
}

/// Check if a target produces WASM output
fn is_wasm_target(target: &cargo_metadata::Target) -> bool {
   let is_bin = target.kind.contains(&TargetKind::Bin)
       && target.crate_types.contains(&CrateType::Bin);
   let is_cdylib = target.kind.contains(&TargetKind::CDyLib)
       && target.crate_types.contains(&CrateType::CDyLib);
   
   is_bin || is_cdylib
}

/// Convert WASM to RWASM format
pub fn wasm_to_rwasm(wasm_path: &PathBuf) -> PathBuf {
   let wasm_bytes = fs::read(wasm_path).expect("Failed to read WASM file");
   let rwasm = compile_wasm_to_rwasm(&wasm_bytes)
       .expect("Failed to compile WASM to RWASM")
       .rwasm_bytecode
       .to_vec();
   
   let output_path = PathBuf::from(env::var("OUT_DIR").unwrap()).join("lib.rwasm");
   fs::write(&output_path, &rwasm).expect("Failed to write RWASM file");
   
   output_path
}

/// Create build artifacts and metadata
fn create_build_artifacts(
   manifest_dir: &PathBuf,
   wasm_path: &PathBuf,
   rwasm_path: &PathBuf,
   cwasm_path: &PathBuf,
) {
   let timestamp = Utc::now().format("%Y%m%dT%H%M%S").to_string();
   let arch = if cfg!(target_arch = "x86_64") { "x86" } else { "arm" };
   let artifact_dir = manifest_dir.join("artifacts").join(arch).join(&timestamp);
   
   fs::create_dir_all(&artifact_dir).expect("Failed to create artifact directory");

   // Strip WASM for additional artifact
   strip_wasm(manifest_dir);

   // Copy all artifacts and calculate hashes
   let artifacts = collect_artifacts(manifest_dir, wasm_path, rwasm_path, cwasm_path);
   let mut build_info = copy_artifacts_with_hashes(&artifacts, &artifact_dir);

   // Add build metadata
   build_info.push_str(&collect_build_metadata());

   // Save build info
   save_build_info(&artifact_dir, &build_info);
}

/// Strip WASM file to create minimal version
fn strip_wasm(manifest_dir: &PathBuf) {
   Command::new("wasm-tools")
       .args(["strip", "-a", "lib.wasm", "-o", "lib.stripped.wasm"])
       .current_dir(manifest_dir)
       .status()
       .expect("Failed to strip WASM");

   Command::new("wasm2wat")
       .args(["lib.stripped.wasm", "-o", "lib.stripped.wat"])
       .current_dir(manifest_dir)
       .status()
       .expect("Failed to convert stripped WASM to WAT");
}

/// Collect all artifacts to be packaged
fn collect_artifacts(
   manifest_dir: &PathBuf,
   wasm_path: &PathBuf,
   rwasm_path: &PathBuf,
   cwasm_path: &PathBuf,
) -> Vec<(PathBuf, &'static str)> {
   vec![
       (manifest_dir.join("lib.wasm"), "lib.wasm"),
       (manifest_dir.join("lib.wat"), "lib.wat"),
       (manifest_dir.join("lib.stripped.wasm"), "lib.stripped.wasm"),
       (manifest_dir.join("lib.stripped.wat"), "lib.stripped.wat"),
       (rwasm_path.clone(), "lib.rwasm"),
       (cwasm_path.clone(), "lib.cwasm"),
   ]
}

/// Copy artifacts and calculate their SHA256 hashes
fn copy_artifacts_with_hashes(
   artifacts: &[(PathBuf, &str)],
   artifact_dir: &PathBuf,
) -> String {
   let mut info = String::new();
   
   for (src, name) in artifacts {
       if src.exists() {
           let dst = artifact_dir.join(name);
           fs::copy(src, &dst).expect("Failed to copy artifact");
           
           if let Ok(bytes) = fs::read(&dst) {
               let hash = sha256::digest(&bytes);
               info.push_str(&format!("sha256({}): {}\n", name, hash));
           }
       }
   }
   
   info
}

/// Collect build metadata (git commit, toolchain versions, etc.)
fn collect_build_metadata() -> String {
   let mut metadata = String::new();

   // Git commit hash
   if let Ok(output) = Command::new("git").args(["rev-parse", "HEAD"]).output() {
       if output.status.success() {
           metadata.push_str(&format!(
               "commit: {}\n",
               String::from_utf8_lossy(&output.stdout).trim()
           ));
       }
   }

   // Rust toolchain info
   if let Ok(output) = Command::new("rustc").arg("--version").output() {
       metadata.push_str(&format!(
           "rustc: {}\n",
           String::from_utf8_lossy(&output.stdout).trim()
       ));
   }

   if let Ok(output) = Command::new("cargo").arg("--version").output() {
       metadata.push_str(&format!(
           "cargo: {}\n",
           String::from_utf8_lossy(&output.stdout).trim()
       ));
   }

   // Build environment
   metadata.push_str(&format!("target: {}\n", env::var("TARGET").unwrap()));
   metadata.push_str(&format!("build_time: {}\n", Utc::now().to_rfc3339()));

   metadata
}

/// Save build information to file
fn save_build_info(artifact_dir: &PathBuf, info: &str) {
   let info_path = artifact_dir.join("BUILD-INFO.md");
   File::create(info_path)
       .and_then(|mut f| f.write_all(info.as_bytes()))
       .expect("Failed to write build info");
}

fn main() {
   // Skip when building for WASM target (this build.rs is not needed inside WASM)
   if env::var("TARGET").unwrap() == "wasm32-unknown-unknown" {
       return;
   }

   let manifest_dir = get_manifest_dir();

   // Build deterministic WASM and derivatives
   let wasm_path = rust_to_wasm(RustToWasmConfig::default());
   copy_wasm_and_wat(&wasm_path);
   let rwasm_path = wasm_to_rwasm(&wasm_path);
   let cwasm_path = wasm_to_wasmtime(&wasm_path);
   generate_build_output_file(&wasm_path, &rwasm_path, &cwasm_path);

   // Create build artifacts with metadata
   create_build_artifacts(&manifest_dir, &wasm_path, &rwasm_path, &cwasm_path);
}