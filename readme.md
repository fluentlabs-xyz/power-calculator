# 🐳 Building inside Docker (x86\_64)

To compare artifacts between macOS (ARM) and Linux (x86), you can build inside Docker:

## 1. **Build Docker image**

```bash
docker build --platform=linux/amd64 -t rust-wasm-x86 -f Dockerfile .
```

## 2. **Run build inside container**

```bash
docker run --rm -it \
  --platform=linux/amd64 \
  -v "$PWD":/workspace \
  -w /workspace \
  rust-wasm-x86
```

> This will produce output artifacts in `artifacts/x86/`.

## 3. **Build locally on macOS (ARM)**

```bash
CARGO_TARGET_DIR=target-arm \
cargo build --release --target wasm32-unknown-unknown \
--no-default-features --locked
```

> Output will be stored in `artifacts/arm/`.

## 4. **Compare artifacts**

```bash
just compare

wasm-objdump -j code -s arm.wasm > arm.code
wasm-objdump -j code -s x86.wasm > x86.code
cmp arm.code x86.code

wasm-tools strip -a arm.wasm -o arm.clean.wasm
wasm-tools strip -a x86.wasm -o x86.clean.wasm
```

  Finished `release` profile [optimized] target(s) in 2m 52s
