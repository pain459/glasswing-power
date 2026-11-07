use procfs::{CurrentSI, KernelStats};
use std::collections::HashMap;
use crate::sensors::record::CPUStat;

// !!!!!!!!!!!!!!!!! CPUCore !!!!!!!!!!!!!!!!!!!!!!!
/// CPUCore reprensents each CPU core on the host,
/// owned by a CPUSocket. CPUCores are instanciated regardless if
/// HyperThreading is activated on the host.
/// Reprensents the processor field in /proc/cpuinfo.
#[derive(Debug, Clone)]
pub struct CPUCore {
    pub id: u16,
    pub attributes: HashMap<String, String>,
}

impl CPUCore {
    /// Instantiates CPUCore and returns the instance.
    pub fn new(id: u16, attributes: HashMap<String, String>) -> CPUCore {
        CPUCore { id, attributes }
    }

    /// Reads content from /proc/stat and extracts the stats of the CPU core
    pub fn read_stats(&self) -> Option<CPUStat> {
        if let Ok(mut kernelstats) = KernelStats::current() {
            return Some(CPUStat::from_procfs_cputime(
                kernelstats.cpu_time.remove(self.id as usize),
            ));
        }
        None
    }
}
