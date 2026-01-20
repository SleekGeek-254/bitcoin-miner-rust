# Bitcoin Miner Rust

A high-performance Bitcoin mining application written in Rust, utilizing GPU acceleration via CUDA for efficient cryptocurrency mining.

## Features

- **GPU-Accelerated Mining**: Leverages NVIDIA GPUs with CUDA for high-speed hash calculations
- **Real-Time Statistics**: Monitors mining performance, hashrate, power consumption, and energy usage
- **Block Verification**: Validates found blocks with hash verification
- **Flexible Configuration**: Command-line interface with customizable mining parameters
- **Cross-Platform**: Built with Rust for performance and reliability

## Prerequisites

- Rust (latest stable version recommended)
- CUDA Toolkit (compatible with your NVIDIA GPU)
- NVIDIA GPU with CUDA support (compute capability 3.0 or higher)
- Windows/Linux/macOS operating system

## Installation

1. Clone the repository:
   ```bash
   git clone https://github.com/yourusername/bitcoin-miner-rust.git
   cd bitcoin-miner-rust
   ```

2. Build the project in release mode:
   ```bash
   cargo build --release
   ```

## Usage

Run the miner with the following command structure:

```bash
cargo run --release -- mine \
  --version 0x251e0000 \
  --prev-hash c334194761cbc35fa2165c33efe0c774d806b03faf6101000000000000000000 \
  --merkle-root 70d303e3d12a22ca651b9cafc9b9e9417e1591784f3641ecb8d0c1bd31a436f1 \
  --timestamp 0x696f62d3 \
  --bits 0x1701ebf2 \
  --start-nonce 0x00000000 \
  --end-nonce 0xffffffff \
  --device 0
```

### Parameters

- `--version`: Block version (hex)
- `--prev-hash`: Previous block hash (hex)
- `--merkle-root`: Merkle root (hex)
- `--timestamp`: Block timestamp (hex)
- `--bits`: Target bits (hex)
- `--start-nonce`: Starting nonce value (hex)
- `--end-nonce`: Ending nonce value (hex)
- `--device`: GPU device index (default: 0)

## Sample Output

```
======================================================================
GPU BITCOIN MINER - RUST EDITION (OPTIMIZED)
======================================================================

🔥 NVIDIA GeForce RTX 2060 (CC 7.5, 6143 MB, 30 SMs)

Block data (parsed):
   version:     0x251e0000
   prev_hash:   c334194761cbc35fa2165c33efe0c774d806b03faf6101000000000000000000
   merkle_root: 70d303e3d12a22ca651b9cafc9b9e9417e1591784f3641ecb8d0c1bd31a436f1
   timestamp:   0x696f62d3  (1768907475)
   bits:        0x1701ebf2
   nonce range: 0x00000000 – 0xffffffff

Monitoring GPU: NVIDIA GeForce RTX 2060
⠴ [###################################################################################>----------------------------------------------------------]  59% | 515021906 H/s | 5.1s
Mining statistics saved to: mining_stats_NVIDIA_GeForce_RTX_2060_20260120_155531.json

Mining Statistics Summary:
  GPU          : NVIDIA GeForce RTX 2060
  Duration     : 5.1 s
  Total hashes : 2618778600
  Avg hashrate : 509.95 MH/s
  Peak hashrate: 515.21 MH/s
  Avg power    : 39.8 W
  Peak power   : 40 W
  Energy used  : 0.000 kWh  (161 J)

🎉 BLOCK FOUND!
   Nonce: 2598977637 (0x9ae94065)
   Hash: 00000000000000000000e50a110e848e494dcc62b58c6db07d886ff01075a0f1
   Time: 5.09s
   Rate: 514656834 H/s
   Total hashes: 2618778600
   Hash verification: ✅
======================================================================
```

## Project Structure

- `src/`: Main source code
  - `main.rs`: Entry point and CLI handling
  - `miner/`: Mining logic and CUDA integration
  - `stats.rs`: Statistics collection and reporting
- `kernels/`: CUDA kernel code
- `utils/`: Utility scripts for building and running

## Dependencies

- `rustacuda`: CUDA bindings for Rust
- `nvml-wrapper`: NVIDIA management library for GPU monitoring
- `clap`: Command-line argument parsing
- `serde`: Serialization for statistics
- `sha2`: SHA-256 hashing
- `indicatif`: Progress bars

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## License

This project is licensed under the MIT License - see the LICENSE file for details.

## Disclaimer

This software is for educational and research purposes. Bitcoin mining requires significant computational resources and may not be profitable. Always ensure compliance with local laws and regulations regarding cryptocurrency mining.
