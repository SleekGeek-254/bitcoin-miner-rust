Write-Host "🚀 QUICK COMPILE & RUN" -ForegroundColor Cyan
Write-Host "=" * 60 -ForegroundColor Cyan
 
# Build Rust
Write-Host "Building Rust project..." -ForegroundColor Yellow
cargo build --release
if ($LASTEXITCODE -ne 0) {
    Write-Host "Rust build failed!" -ForegroundColor Red
    exit 1
}
Write-Host "✅ Rust project built" -ForegroundColor Green

# Run test
Write-Host "`nRunning miner test..." -ForegroundColor Yellow
Write-Host "=" * 60 -ForegroundColor Cyan

PS D:\Company\DEXTERS_LAB\DEV\EUTOPIA\BITCOIN_REVERSE_ENG\RUST\bitcoin-miner-rust> cargo run --release -- mine `
>>   --version 0x251e0000 `
>>   --prev-hash c334194761cbc35fa2165c33efe0c774d806b03faf6101000000000000000000 `
>>   --merkle-root 70d303e3d12a22ca651b9cafc9b9e9417e1591784f3641ecb8d0c1bd31a436f1 `
>>   --timestamp 0x696f62d3 `
>>   --bits 0x1701ebf2 `
>>   --start-nonce 0x00000000 `
>>   --end-nonce 0xffffffff `
>>   --device 0

Write-Host "`n" + ("=" * 60) -ForegroundColor Cyan
Write-Host "Test complete!" -ForegroundColor Green