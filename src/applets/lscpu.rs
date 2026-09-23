//! `lscpu` -- print basic CPU information from `/proc/cpuinfo`
//! (Linux). A summary, not a full replica of the real `lscpu` --
//! that also reads cache topology, NUMA nodes, and CPU flags out of
//! `/sys`, none of which is covered here yet.

use crate::common::errors::{AppError, AppResult};

pub const USAGE: &str = "lscpu -- print basic CPU information";

#[cfg(target_os = "linux")]
pub fn run(_args: Vec<String>) -> AppResult<()> {
    let contents = std::fs::read_to_string("/proc/cpuinfo")
        .map_err(|e| AppError::new(format!("cannot read /proc/cpuinfo: {e}")))?;

    let mut model_name = None;
    let mut vendor_id = None;
    let mut cpu_mhz = None;
    let mut cache_size = None;
    let mut processor_count = 0usize;

    for line in contents.lines() {
        let Some((key, value)) = line.split_once(':') else {
            continue;
        };
        let key = key.trim();
        let value = value.trim();
        match key {
            "processor" => processor_count += 1,
            "model name" if model_name.is_none() => model_name = Some(value.to_string()),
            "vendor_id" if vendor_id.is_none() => vendor_id = Some(value.to_string()),
            "cpu MHz" if cpu_mhz.is_none() => cpu_mhz = Some(value.to_string()),
            "cache size" if cache_size.is_none() => cache_size = Some(value.to_string()),
            _ => {}
        }
    }

    println!("CPU(s):              {processor_count}");
    if let Some(v) = vendor_id {
        println!("Vendor ID:           {v}");
    }
    if let Some(m) = model_name {
        println!("Model name:          {m}");
    }
    if let Some(mhz) = cpu_mhz {
        println!("CPU MHz:             {mhz}");
    }
    if let Some(cache) = cache_size {
        println!("Cache size:          {cache}");
    }
    Ok(())
}

#[cfg(not(target_os = "linux"))]
pub fn run(_args: Vec<String>) -> AppResult<()> {
    Err(AppError::new(
        "CPU info not available on this target (needs /proc/cpuinfo)",
    ))
}
