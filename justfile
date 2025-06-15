# Compare build artifacts between macOS (arm) and Docker (x86)

compare:
    @echo "🔍 SHA256 hashes:"
    sha256sum artifacts/arm/lib.wasm artifacts/x86/lib.wasm || true
    sha256sum artifacts/arm/lib.rwasm artifacts/x86/lib.rwasm || true
    sha256sum artifacts/arm/lib.cwasm artifacts/x86/lib.cwasm || true

    @echo "\n🔍 Converting to .wat:"
    wasm2wat artifacts/arm/lib.wasm -o artifacts/arm/lib.wat
    wasm2wat artifacts/x86/lib.wasm -o artifacts/x86/lib.wat

    @echo "\n🔍 Diff .wat:"
    diff -u artifacts/arm/lib.wat artifacts/x86/lib.wat || echo "✅ wat diff done"

    @echo "\n🔍 Compare BUILD-INFO.md:"
    diff -u artifacts/arm/BUILD-INFO.md artifacts/x86/BUILD-INFO.md || echo "✅ build-info diff done"
