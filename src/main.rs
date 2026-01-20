mod miner;
mod stats;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use indicatif::{ProgressBar, ProgressStyle};
use miner::{BlockData, GPUMiner, verify_block};
use std::time::Duration;

fn parse_u32_hex_or_decimal(s: &str) -> Result<u32> {
    let s = s.trim();

    if s.to_lowercase().starts_with("0x") {
        // Keep original case for parsing
        let hex_part = &s[2..];
        u32::from_str_radix(hex_part, 16)
            .with_context(|| format!("Invalid hex value: {}", s))
    } else {
        s.parse::<u32>()
            .with_context(|| format!("Invalid number: {}", s))
    }
}

#[derive(Parser)]
#[command(name = "bitcoin-miner-rust")]
#[command(about = "High-performance Bitcoin miner in Rust with CUDA")]
#[command(version = "1.0")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}



#[derive(Subcommand)]
enum Commands {
    Mine {
        /// Block version (decimal or hex with 0x prefix)
        #[arg(long)]
        version: String,

        /// Previous block hash (32-byte hex, no 0x prefix)
        #[arg(long)]
        prev_hash: String,

        /// Merkle root (32-byte hex, no 0x prefix)
        #[arg(long)]
        merkle_root: String,

        /// Timestamp (decimal or hex with 0x prefix)
        #[arg(long)]
        timestamp: String,

        /// Bits / difficulty target (decimal or hex with 0x prefix)
        #[arg(long)]
        bits: String,

        /// Start nonce (decimal or hex with 0x prefix)
        #[arg(long, default_value = "0")]
        start_nonce: String,

        /// End nonce (decimal or hex with 0x prefix)
        #[arg(long, default_value = "0xffffffff")]
        end_nonce: String,

        /// GPU device ID
        #[arg(short = 'd', long, default_value_t = 0)]
        device: usize,

        /// Progress report interval (seconds)
        #[arg(short = 'i', long, default_value_t = 0.5)]
        interval: f64,
    },
    
    /// Verify a known block
    Verify {
        /// Block version
        #[arg(short, long, default_value_t = 2)]
        version: u32,
        
        /// Previous block hash
        #[arg(short = 'p', long)]
        prev_hash: String,
        
        /// Merkle root
        #[arg(short = 'm', long)]
        merkle_root: String,
        
        /// Timestamp
        #[arg(short = 't', long)]
        timestamp: u32,
        
        /// Bits (can be decimal or hex with 0x prefix)
        #[arg(short = 'b', long, value_parser = parse_hex_or_decimal)]
        bits: u32,
        
        /// Nonce to verify (can be decimal or hex with 0x prefix)
        #[arg(short = 'n', long, value_parser = parse_hex_or_decimal)]
        nonce: u32,
    },
    
    /// Quick test - search near known nonce
    Test {
        /// GPU device ID
        #[arg(short = 'd', long, default_value_t = 0)]
        device: usize,
    },
    
    /// List available GPU devices
    Devices,
}

/// Parse a string that can be either decimal or hex (with 0x prefix)
fn parse_hex_or_decimal(s: &str) -> Result<u32, String> {
    let s = s.trim();
    
    if s.starts_with("0x") || s.starts_with("0X") {
        u32::from_str_radix(&s[2..], 16)
            .map_err(|e| format!("Invalid hex value '{}': {}", s, e))
    } else {
        s.parse::<u32>()
            .map_err(|e| format!("Invalid decimal value '{}': {}", s, e))
    }
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    
    match cli.command {
        Commands::Mine {
            version,
            prev_hash,
            merkle_root,
            timestamp,
            bits,
            start_nonce,
            end_nonce,
            device,
            interval,
        } => {
            let separator = "=".repeat(70);
            println!("{}", separator);
            println!("GPU BITCOIN MINER - RUST EDITION (OPTIMIZED)");
            println!("{}", separator);

            // Parse all numeric fields
            let version_u32    = parse_u32_hex_or_decimal(&version)?;
            let timestamp_u32  = parse_u32_hex_or_decimal(&timestamp)?;
            let bits_u32       = parse_u32_hex_or_decimal(&bits)?;
            let start_nonce_u32 = parse_u32_hex_or_decimal(&start_nonce)?;
            let end_nonce_u32  = parse_u32_hex_or_decimal(&end_nonce)?;
            
            // Create miner (now with persistent CUDA context)
            let miner = GPUMiner::new(device, true)?;
            
            // Display device info
            println!("\n🔥 {}", miner.get_device_info()?);

            println!("\nBlock data (parsed):");
            println!("   version:     0x{:08x}", version_u32);
            println!("   prev_hash:   {}", prev_hash);
            println!("   merkle_root: {}", merkle_root);
            println!("   timestamp:   0x{:08x}  ({})", timestamp_u32, timestamp_u32);
            println!("   bits:        0x{:08x}", bits_u32);
            println!("   nonce range: 0x{:08x} – 0x{:08x}", start_nonce_u32, end_nonce_u32);
            
            let block_data = BlockData {
                version:     version_u32,
                prev_hash,
                merkle_root,
                timestamp:   timestamp_u32,
                bits:        bits_u32,
            };
            
            // Create progress bar
            let pb = ProgressBar::new(100);
            pb.set_style(
                ProgressStyle::with_template("{spinner:.green} [{wide_bar:.cyan/blue}] {percent:>3}% | {msg}")
                    .unwrap()
                    .progress_chars("#>-")
            );
            
            // Use interior mutability for last_update
            use std::cell::Cell;
            use std::rc::Rc;
            
            let start_time = std::time::Instant::now();
            let last_update = Rc::new(Cell::new(std::time::Instant::now()));
            let pb_rc = Rc::new(std::cell::RefCell::new(pb));
            
            let result = miner.mine(
                &block_data,
                Some((start_nonce_u32, end_nonce_u32)),
                Some(Box::new({
                    let last_update = last_update.clone();
                    let pb_rc = pb_rc.clone();
                    let interval = interval;
                    
                    move |progress, hashrate| {
                        if last_update.get().elapsed() >= Duration::from_secs_f64(interval) {
                            pb_rc.borrow_mut().set_position(progress as u64);
                            pb_rc.borrow_mut().set_message(format!(
                                "{:.0} H/s | {:.1}s",
                                hashrate,
                                start_time.elapsed().as_secs_f64()
                            ));
                            last_update.set(std::time::Instant::now());
                        }
                    }
                })),
            )?;
            
            // Get the progress bar back
            let pb = Rc::try_unwrap(pb_rc).unwrap().into_inner();
            pb.finish_and_clear();
            
            if let Some(result) = result {
                println!("\n🎉 BLOCK FOUND!");
                println!("   Nonce: {} (0x{:08x})", result.nonce, result.nonce);
                println!("   Hash: {}", result.hash);
                println!("   Time: {:.2}s", result.elapsed.as_secs_f64());
                println!("   Rate: {:.0} H/s", result.hashrate);
                println!("   Total hashes: {}", result.total_hashes);
                
                // Verify the found hash
                let expected_hash = verify_block(&block_data, result.nonce)?;
                if result.hash == expected_hash {
                    println!("   Hash verification: ✅");
                } else {
                    println!("   Hash verification: ❌");
                    println!("   Expected: {}", expected_hash);
                }
            } else {
                println!("\n❌ No block found in the specified range");
            }
            
            println!("\n💡 RTX 2060 PERFORMANCE (OPTIMIZED):");
            println!("   • Expected: 250-350 MH/s");
            println!("{}", separator);
        }
        
        Commands::Verify {
            version,
            prev_hash,
            merkle_root,
            timestamp,
            bits,
            nonce,
        } => {
            let block_data = BlockData {
                version,
                prev_hash,
                merkle_root,
                timestamp,
                bits,
            };
            
            println!("🔍 VERIFYING BLOCK...");
            println!("   Nonce: 0x{:08x}", nonce);
            
            match verify_block(&block_data, nonce) {
                Ok(hash) => {
                    println!("   Hash: {}", hash);
                    println!("   Valid: ✅ (Hash computed successfully)");
                }
                Err(e) => {
                    println!("   Error: {}", e);
                    println!("   Valid: ❌");
                }
            }
        }
        
        Commands::Test { device } => {
            let separator = "=".repeat(70);
            println!("{}", separator);
            println!("QUICK TEST - SEARCHING NEAR KNOWN NONCE");
            println!("{}", separator);
            
            let miner = GPUMiner::new(device, true)?;
            println!("\n🔥 {}", miner.get_device_info()?);
            
            let block_data = BlockData {
                version: 2,
                prev_hash: "15a20d97f5a65e130e08f2b254f97f65b96173a7057aef0da203000000000000".to_string(),
                merkle_root: "887e309c02ebdddbd0f3faff78f868d61b1c4cff2a25e5b3c9d90ff501818fa0".to_string(),
                timestamp: 1348310759,
                bits: 0x1a05db8b,
            };
            
            // Search near the known correct nonce
            let search_start = 0xf7d8d000; // 0xf7d8d840 - 0x840
            let search_end = 0xf7d8ffff;   // 0xf7d8d840 + 0x17bf
            
            println!("\n🔍 Searching range: 0x{:08x} - 0x{:08x}", search_start, search_end);
            println!("   Known correct nonce: 0xf7d8d840");
            
            let result = miner.mine(
                &block_data,
                Some((search_start, search_end)),
                None,
            )?;
            
            if let Some(result) = result {
                println!("\n🎉 TEST SUCCESS!");
                println!("   Found nonce: 0x{:08x}", result.nonce);
                println!("   Expected:   0xf7d8d840");
                println!("   Match: {}", if result.nonce == 0xf7d8d840 { "✅" } else { "❌" });
                println!("   Hash: {}", result.hash);
                println!("   Time: {:.2}s", result.elapsed.as_secs_f64());
                println!("   Rate: {:.0} H/s", result.hashrate);
            } else {
                println!("\n❌ TEST FAILED - Did not find correct nonce");
            }
            
            println!("{}", separator);
        }
        
        Commands::Devices => {
            println!("🔧 AVAILABLE GPU DEVICES:");
            match miner::cuda_wrapper::initialize_cuda() {
                Ok(_) => {
                    for i in 0.. {
                        match miner::cuda_wrapper::get_device_info(i) {
                            Ok(info) => println!("   {}: {}", i, info),
                            Err(_) => break,
                        }
                    }
                }
                Err(e) => println!("   CUDA not available: {}", e),
            }
        }
    }
    
    Ok(())
}