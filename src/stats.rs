use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::Duration;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Sample {
    pub timestamp: DateTime<Utc>,
    pub elapsed_secs: f64,
    pub hashrate_mhs: f64,
    pub power_w: u32,              // current power draw in watts
    pub gpu_util: u32,             // percent
    pub mem_util: u32,             // percent (often 0 on mining)
    pub gpu_temp: u32,             // °C
    pub hashes_this_sample: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MiningStats {
    pub device_name: String,
    pub start_time: DateTime<Utc>,
    pub end_time: Option<DateTime<Utc>>,
    pub total_hashes: u64,
    pub best_hashrate_mhs: f64,
    pub worst_hashrate_mhs: f64,
    pub avg_hashrate_mhs: f64,
    pub avg_power_w: f64,
    pub max_power_w: u32,
    pub total_energy_j: f64,           // ≈ watt-seconds
    pub total_energy_kwh: f64,
    pub samples: Vec<Sample>,
}

impl MiningStats {
    pub fn new(device_name: String) -> Self {
        MiningStats {
            device_name,
            start_time: Utc::now(),
            end_time: None,
            total_hashes: 0,
            best_hashrate_mhs: 0.0,
            worst_hashrate_mhs: f64::MAX,
            avg_hashrate_mhs: 0.0,
            avg_power_w: 0.0,
            max_power_w: 0,
            total_energy_j: 0.0,
            total_energy_kwh: 0.0,
            samples: Vec::with_capacity(3600), // ~1 sample/sec for 1 hour
        }
    }

    pub fn add_sample(
        &mut self,
        elapsed: Duration,
        hashrate_mhs: f64,
        power_w: u32,
        gpu_util: u32,
        mem_util: u32,
        gpu_temp: u32,
        hashes_delta: u64,
    ) {
        let now = Utc::now();
        let sample = Sample {
            timestamp: now,
            elapsed_secs: elapsed.as_secs_f64(),
            hashrate_mhs,
            power_w,
            gpu_util,
            mem_util,
            gpu_temp,
            hashes_this_sample: hashes_delta,
        };

        self.samples.push(sample);

        // Update aggregates
        self.total_hashes += hashes_delta;

        if hashrate_mhs > self.best_hashrate_mhs {
            self.best_hashrate_mhs = hashrate_mhs;
        }
        if hashrate_mhs > 0.0 && hashrate_mhs < self.worst_hashrate_mhs {
            self.worst_hashrate_mhs = hashrate_mhs;
        }

        let n = self.samples.len() as f64;
        self.avg_hashrate_mhs = ((self.avg_hashrate_mhs * (n - 1.0)) + hashrate_mhs) / n;
        self.avg_power_w = ((self.avg_power_w * (n - 1.0)) + power_w as f64) / n;

        if power_w > self.max_power_w {
            self.max_power_w = power_w;
        }

        // Energy calculation — use the last two samples (safe after push)
        if self.samples.len() >= 2 {
            let curr = &self.samples[self.samples.len() - 1];
            let prev = &self.samples[self.samples.len() - 2];
            let dt = curr.elapsed_secs - prev.elapsed_secs;
            if dt > 0.0 {
                let avg_p = (curr.power_w as f64 + prev.power_w as f64) / 2.0;
                self.total_energy_j += avg_p * dt;
                self.total_energy_kwh = self.total_energy_j / 3_600_000.0;
            }
        }
    }

    pub fn finish(&mut self) {
        self.end_time = Some(Utc::now());
    }

    pub fn save_to_file(&self, path: impl Into<PathBuf>) -> anyhow::Result<()> {
        let path = path.into();
        let json = serde_json::to_string_pretty(self)?;
        std::fs::write(&path, json)?;
        println!("Mining statistics saved to: {}", path.display());
        Ok(())
    }
}