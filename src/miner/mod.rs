// src/miner/mod.rs (updated)

pub mod cuda_wrapper;

use anyhow::{Context, Result};
use hex;
use std::time::{Duration, Instant};

use crate::stats::MiningStats;
use nvml_wrapper::Nvml;
use nvml_wrapper::struct_wrappers::device::Utilization;
use nvml_wrapper::enum_wrappers::device::TemperatureSensor;
use chrono::Utc;

#[derive(Debug, Clone)]
pub struct BlockData {
    pub version: u32,
    pub prev_hash: String,
    pub merkle_root: String,
    pub timestamp: u32,
    pub bits: u32,
}

#[derive(Debug, Clone)]
pub struct MiningResult {
    pub nonce: u32,
    pub hash: String,
    pub elapsed: Duration,
    pub hashrate: f64,
    pub total_hashes: u64,
}

#[allow(dead_code)]
pub struct GPUMiner {
    device_id: usize,
    verbose: bool,
    cuda_context: Option<cuda_wrapper::CudaContext>,
}

impl GPUMiner {
    pub fn new(device_id: usize, verbose: bool) -> Result<Self> {
        // Create CUDA context once
        let cuda_context = match cuda_wrapper::CudaContext::new(device_id) {
            Ok(ctx) => Some(ctx),
            Err(e) => {
                if verbose {
                    eprintln!("Failed to create CUDA context: {}", e);
                }
                return Err(e);
            }
        };

        Ok(Self {
            device_id,
            verbose,
            cuda_context,
        })
    }

    pub fn get_device_info(&self) -> Result<String> {
        match &self.cuda_context {
            Some(ctx) => ctx.get_device_info(),
            None => Ok("CUDA not available".to_string()),
        }
    }

    pub fn mine(
        &self,
        block_data: &BlockData,
        search_range: Option<(u32, u32)>,
        progress_callback: Option<Box<dyn Fn(f64, f64)>>,
    ) -> Result<Option<MiningResult>> {
        let cuda_context = self.cuda_context
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("CUDA context not initialized"))?;

        // ────────────────────────────────────────────────
        //   Initialize NVML
        // ────────────────────────────────────────────────
        let nvml = Nvml::init().context("Failed to initialize NVML (is driver/NVML installed?)")?;
        let device = nvml.device_by_index(self.device_id as u32)
            .context("Cannot access NVML device by index")?;

        let device_name = device.name().unwrap_or_else(|_| "Unknown GPU".to_string());

        let mut stats = MiningStats::new(device_name.clone());

        println!("Monitoring GPU: {}", device_name);

        let (start_nonce, end_nonce) = search_range.unwrap_or((0, u32::MAX));

        // Convert hex strings to bytes
        let prev_hash_bytes = hex::decode(&block_data.prev_hash).context("Invalid previous hash")?;
        let merkle_root_bytes = hex
            ::decode(&block_data.merkle_root)
            .context("Invalid merkle root")?;

        if prev_hash_bytes.len() != 32 {
            anyhow::bail!("Previous hash must be 32 bytes");
        }
        if merkle_root_bytes.len() != 32 {
            anyhow::bail!("Merkle root must be 32 bytes");
        }

        // Build header prefix (76 bytes: version + prev_hash + merkle_root + timestamp + bits)
        let mut header_prefix = Vec::with_capacity(76);
        header_prefix.extend_from_slice(&block_data.version.to_le_bytes());
        header_prefix.extend_from_slice(&prev_hash_bytes);
        header_prefix.extend_from_slice(&merkle_root_bytes);
        header_prefix.extend_from_slice(&block_data.timestamp.to_le_bytes());
        header_prefix.extend_from_slice(&block_data.bits.to_le_bytes());

        // Calculate target (Bitcoin difficulty target)
        let exponent = (block_data.bits >> 24) as usize;
        let coefficient = block_data.bits & 0xffffff;

        // Ensure exponent >= 3
        if exponent < 3 {
            anyhow::bail!("Invalid bits exponent: {}", exponent);
        }

        // Build target as 32-byte little-endian array
        let mut target_bytes = [0u8; 32];

        // The coefficient occupies 3 bytes, starting at position (exponent - 3)
        let start_pos = exponent - 3;

        if start_pos + 3 <= 32 {
            // Place the 3-byte coefficient at the correct position
            target_bytes[start_pos] = (coefficient & 0xff) as u8;
            target_bytes[start_pos + 1] = ((coefficient >> 8) & 0xff) as u8;
            target_bytes[start_pos + 2] = ((coefficient >> 16) & 0xff) as u8;
        } else {
            anyhow::bail!("Target overflow: exponent {} too large", exponent);
        }

        let start_time = Instant::now();
        let mut total_hashes: u64 = 0;
        let mut last_hashes: u64 = 0;
        let mut last_sample_time = Instant::now();

        let mut current_nonce = start_nonce;
        let mut found: Option<(u32, [u8; 32])> = None;

        // Configure kernel parameters
        let threads_per_block = 666;
        let max_blocks = 65535;
        let max_batch_size = threads_per_block * max_blocks;

        while current_nonce <= end_nonce && found.is_none() {
            // Calculate batch size
            let remaining = (end_nonce as u64) - (current_nonce as u64) + 1;
            let batch_size = if remaining > (max_batch_size as u64) {
                max_batch_size
            } else {
                remaining as usize
            };

            let batch_start = Instant::now();

            // Call CUDA kernel
            let result = cuda_context.mine_batch(
                &header_prefix,
                &target_bytes,
                current_nonce,
                batch_size,
            )?;

            let _batch_duration = batch_start.elapsed();  // Prefix with _ to suppress unused warning

            let _batch_hashes = batch_size as u64;  // Prefix with _ if not used

            total_hashes += batch_size as u64;

            // ────────────────────────────────────────────────
            //   Collect GPU stats ~ every 1–2 seconds
            // ────────────────────────────────────────────────
            if last_sample_time.elapsed() >= Duration::from_secs(1) || result.is_some() {
                let elapsed = start_time.elapsed();

                let power_mw = device.power_usage().unwrap_or(0);           // milliwatts
                let power_w   = (power_mw / 1000) as u32;                   // watts
                let util       = device.utilization_rates().unwrap_or(Utilization { gpu: 0, memory: 0 });
                let temp       = device.temperature(TemperatureSensor::Gpu).unwrap_or(0);

                let current_hashrate_mhs = if elapsed.as_secs_f64() > 0.3 {
                    (total_hashes as f64 / elapsed.as_secs_f64()) / 1_000_000.0
                } else {
                    0.0
                };

                let hashes_delta = total_hashes - last_hashes;

                stats.add_sample(
                    elapsed,
                    current_hashrate_mhs,
                    power_w,
                    util.gpu,
                    util.memory,
                    temp,
                    hashes_delta,
                );

                last_hashes = total_hashes;
                last_sample_time = Instant::now();
            }

            if let Some(res) = result {
                found = Some(res);
            }

            // Update progress
            if let Some(callback) = &progress_callback {
                let progress = {
                    let processed = (current_nonce as u64).wrapping_sub(start_nonce as u64);
                    let total = (end_nonce as u64).wrapping_sub(start_nonce as u64).wrapping_add(1);
                    (processed as f64 / total as f64) * 100.0
                };
                let elapsed = start_time.elapsed().as_secs_f64();
                let hashrate = (total_hashes as f64) / elapsed;
                callback(progress, hashrate);
            }

            current_nonce = current_nonce.saturating_add(batch_size as u32);
        }

        stats.finish();

        // Save stats
        let filename = format!(
            "mining_stats_{}_{}.json",
            device_name.replace(" ", "_"),
            Utc::now().format("%Y%m%d_%H%M%S")
        );
        let _ = stats.save_to_file(&filename);

        // Print summary
        println!("\nMining Statistics Summary:");
        println!("  GPU          : {}", stats.device_name);
        println!("  Duration     : {:.1} s", start_time.elapsed().as_secs_f64());
        println!("  Total hashes : {}", stats.total_hashes);
        println!("  Avg hashrate : {:.2} MH/s", stats.avg_hashrate_mhs);
        println!("  Peak hashrate: {:.2} MH/s", stats.best_hashrate_mhs);
        println!("  Avg power    : {:.1} W", stats.avg_power_w);
        println!("  Peak power   : {} W", stats.max_power_w);
        println!("  Energy used  : {:.3} kWh  ({:.0} J)", stats.total_energy_kwh, stats.total_energy_j);

        if let Some((found_nonce, found_hash)) = found {
            let elapsed = start_time.elapsed();
            let hashrate = (total_hashes as f64) / elapsed.as_secs_f64();

            // Verify the found hash
            let _expected_hash = verify_block(block_data, found_nonce)?;
            let found_hash_hex = hex::encode(found_hash);

            return Ok(
                Some(MiningResult {
                    nonce: found_nonce,
                    hash: found_hash_hex,
                    elapsed,
                    hashrate,
                    total_hashes,
                })
            );
        } else {
            if self.verbose {
                println!("\n❌ No block found in range");
            }
            Ok(None)
        }
    }
}

pub fn verify_block(block_data: &BlockData, nonce: u32) -> Result<String> {
    use sha2::{ Digest, Sha256 };

    let prev_hash_bytes = hex::decode(&block_data.prev_hash).context("Invalid previous hash")?;
    let merkle_root_bytes = hex::decode(&block_data.merkle_root).context("Invalid merkle root")?;

    // Build full header (80 bytes)
    let mut header = Vec::with_capacity(80);
    header.extend_from_slice(&block_data.version.to_le_bytes());
    header.extend_from_slice(&prev_hash_bytes);
    header.extend_from_slice(&merkle_root_bytes);
    header.extend_from_slice(&block_data.timestamp.to_le_bytes());
    header.extend_from_slice(&block_data.bits.to_le_bytes());
    header.extend_from_slice(&nonce.to_le_bytes());

    // Double SHA-256
    let hash1 = Sha256::digest(&header);
    let hash2 = Sha256::digest(&hash1);

    // Reverse bytes for Bitcoin's hash display
    let mut reversed = hash2.to_vec();
    reversed.reverse();

    Ok(hex::encode(reversed))
}