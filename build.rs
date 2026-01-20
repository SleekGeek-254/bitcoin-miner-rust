fn main() {
    // Tell cargo to link with CUDA
    println!("cargo:rustc-link-search=D:/AppData/NVIDIA GPU Computing Toolkit/CUDA/v12.8/lib/x64");
    println!("cargo:rustc-link-lib=cudart");
    
    // Compile CUDA kernels
    cc::Build::new()
        .cuda(true)
        .flag("-cudart=shared")
        .flag("-gencode")
        .flag("arch=compute_75,code=sm_75")  // RTX 2060 uses Turing (sm_75)
        .file("kernels/mining_kernel.cu")
        .compile("mining_kernel");
}