use anyhow::Result;
use rustacuda::memory::DeviceBuffer;
use rustacuda::prelude::*;
use std::ffi::CString;
use rustacuda::device::DeviceAttribute;
use rustacuda::launch;

pub struct CudaContext {
    context: Option<Context>,
    module: Option<Module>,
    stream: Option<Stream>,
    device_id: usize,
}

impl CudaContext {
    pub fn new(device_id: usize) -> Result<Self> {
        // Initialize CUDA runtime once
        rustacuda::init(CudaFlags::empty())?;

        // Get the device
        let device = Device::get_device(device_id as u32).map_err(|e|
            anyhow::anyhow!("Failed to get device {}: {}", device_id, e)
        )?;

        // Create context
        let context = Context::create_and_push(
            ContextFlags::MAP_HOST | ContextFlags::SCHED_AUTO,
            device
        )?;

        // Load PTX module once
        let ptx_content = include_str!("../../kernels/mining_kernel.ptx");
        let module_data = CString::new(ptx_content).map_err(|e|
            anyhow::anyhow!("Invalid PTX content: {}", e)
        )?;
        let module = Module::load_from_string(&module_data)?;

        // Create stream for kernel execution
        let stream = Stream::new(StreamFlags::NON_BLOCKING, None)?;

        Ok(Self {
            context: Some(context),
            module: Some(module),
            stream: Some(stream),
            device_id,
        })
    }

    pub fn get_device_info(&self) -> Result<String> {
        let device = Device::get_device(self.device_id as u32)?;
        let name = device.name()?;

        let major = device.get_attribute(DeviceAttribute::ComputeCapabilityMajor)? as u32;
        let minor = device.get_attribute(DeviceAttribute::ComputeCapabilityMinor)? as u32;
        let total_memory = device.total_memory()?;
        let multiprocessor_count = device.get_attribute(
            DeviceAttribute::MultiprocessorCount
        )? as u32;

        Ok(
            format!(
                "{} (CC {}.{}, {} MB, {} SMs)",
                name,
                major,
                minor,
                total_memory / 1024 / 1024,
                multiprocessor_count
            )
        )
    }

    pub fn mine_batch(
        &self,
        header_prefix: &[u8],
        target_words_bytes: &[u8; 32],
        start_nonce: u32,
        batch_size: usize
    ) -> Result<Option<(u32, [u8; 32])>> {
        let module = self.module
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("CUDA module not initialized"))?;
        let stream = self.stream
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("CUDA stream not initialized"))?;

        // Get kernel function
        let kernel_name = CString::new("mine_kernel")?;
        let kernel = module.get_function(&kernel_name)?;

        // Prepare device memory
        let mut header_prefix_gpu = DeviceBuffer::from_slice(header_prefix)?;

        // Convert the target bytes to a slice of u32 for the kernel
        let target_words_ptr = target_words_bytes.as_ptr() as *const u32;
        let target_words_slice = unsafe { std::slice::from_raw_parts(target_words_ptr, 8) };
        let mut target_gpu = DeviceBuffer::from_slice(target_words_slice)?;

        // Result buffers (initialize to "not found")
        let result_nonce_host = [0xffffffffu32];
        let mut result_nonce_gpu = DeviceBuffer::from_slice(&result_nonce_host)?;
        let mut result_hash_gpu = DeviceBuffer::from_slice(&[0u8; 32])?;

        // Configure kernel launch parameters
        let threads_per_block = 384;
        let blocks_per_grid = (batch_size + threads_per_block - 1) / threads_per_block;

        // Limit blocks to CUDA maximum
        let max_blocks = 65535;
        let blocks_per_grid = blocks_per_grid.min(max_blocks) as u32;

        let grid_size = (blocks_per_grid, 1, 1);
        let block_size = (threads_per_block as u32, 1, 1);

        // Calculate shared memory size
        let shared_mem_size = (64 + 8) * 4;

        // Launch kernel
        unsafe {
            launch!(kernel<<<grid_size, block_size, shared_mem_size, stream>>>(
                header_prefix_gpu.as_device_ptr(),
                target_gpu.as_device_ptr(),
                start_nonce,
                batch_size as i32,
                result_nonce_gpu.as_device_ptr(),
                result_hash_gpu.as_device_ptr()
            ))?;
        }

        // Synchronize stream
        stream.synchronize()?;

        // Copy results back to host
        let mut found_nonce = [0u32];
        result_nonce_gpu.copy_to(&mut found_nonce)?;

        if found_nonce[0] != 0xffffffff {
            let mut found_hash = [0u8; 32];
            result_hash_gpu.copy_to(&mut found_hash)?;

            // Reverse the hash for Bitcoin display convention
            found_hash.reverse();

            return Ok(Some((found_nonce[0], found_hash)));
        }

        Ok(None)
    }
}

impl Drop for CudaContext {
    fn drop(&mut self) {
        // Manually drop CUDA resources in the correct order
        if let Some(stream) = self.stream.take() {
            // Don't panic if stream destruction fails
            let _ = stream.synchronize();
            // Stream will be dropped automatically
        }

        if let Some(module) = self.module.take() {
            // Module will be dropped automatically
            let _ = module;
        }

        if let Some(context) = self.context.take() {
            // Context will be dropped automatically
            let _ = context;
        }
    }
}

pub fn initialize_cuda() -> Result<()> {
    rustacuda::init(CudaFlags::empty())?;
    Ok(())
}

pub fn get_device_info(device_id: usize) -> Result<String> {
    let device = Device::get_device(device_id as u32)?;
    let name = device.name()?;

    let major = device.get_attribute(DeviceAttribute::ComputeCapabilityMajor)? as u32;
    let minor = device.get_attribute(DeviceAttribute::ComputeCapabilityMinor)? as u32;
    let total_memory = device.total_memory()?;
    let multiprocessor_count = device.get_attribute(DeviceAttribute::MultiprocessorCount)? as u32;

    Ok(
        format!(
            "{} (CC {}.{}, {} MB, {} SMs)",
            name,
            major,
            minor,
            total_memory / 1024 / 1024,
            multiprocessor_count
        )
    )
}
