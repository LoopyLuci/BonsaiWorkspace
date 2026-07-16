//! CLI: create a matrix, write and read some pixels, and report on the
//! detected hardware capability context.

use titan_opencv::{CapabilityContext, CpuCapability, GpuCapability, Mat};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut mat = Mat::create(4, 4, 3, 0)?;
    mat.set(1, 1, 0, 200)?;
    mat.set(1, 1, 1, 100)?;
    mat.set(1, 1, 2, 50)?;

    println!("{}", mat);
    println!(
        "pixel (1,1) = ({}, {}, {})",
        mat.at(1, 1, 0)?,
        mat.at(1, 1, 1)?,
        mat.at(1, 1, 2)?
    );

    let ctx = CapabilityContext::new()
        .with_cpu(CpuCapability::with_avx2())
        .with_gpu(GpuCapability::cuda_12_0(8192));

    println!(
        "capabilities: cpu={} gpu={}",
        ctx.has_cpu(),
        ctx.has_gpu()
    );

    if let Some(gpu) = &ctx.gpu {
        println!(
            "gpu: {} (compute {}), sufficient for 4096MB workload: {}",
            gpu.vendor,
            gpu.compute_capability,
            gpu.has_sufficient_memory(4096)
        );
    }

    Ok(())
}
