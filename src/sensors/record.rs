use procfs::CpuTime;
use std::fmt;
use std::time::Duration;
use crate::sensors::units;

// !!!!!!!!!!!!!!!!! Record !!!!!!!!!!!!!!!!!!!!!!!
/// Record struct represents an electricity consumption measurement
/// tied to a domain.
#[derive(Debug, Clone)]
pub struct Record {
    pub timestamp: Duration,
    pub value: String,
    pub unit: units::Unit,
}

impl Record {
    /// Instances Record and returns the instance
    pub fn new(timestamp: Duration, value: String, unit: units::Unit) -> Record {
        Record {
            timestamp,
            value,
            unit,
        }
    }
}

impl fmt::Display for Record {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "recorded {} {} at {:?}",
            self.value.trim(),
            self.unit,
            self.timestamp
        )
    }
}

#[derive(Debug)]
pub struct CPUStat {
    pub user: u64,
    pub nice: u64,
    pub system: u64,
    pub idle: u64,
    pub irq: Option<u64>,
    pub iowait: Option<u64>,
    pub softirq: Option<u64>,
    pub steal: Option<u64>,
    pub guest: Option<u64>,
    pub guest_nice: Option<u64>,
}

impl CPUStat {
    pub fn from_procfs_cputime(cpu_time: CpuTime) -> CPUStat {
        CPUStat {
            user: cpu_time.user,
            nice: cpu_time.nice,
            system: cpu_time.system,
            idle: cpu_time.idle,
            irq: cpu_time.irq,
            iowait: cpu_time.iowait,
            softirq: cpu_time.softirq,
            steal: cpu_time.steal,
            guest: cpu_time.guest,
            guest_nice: cpu_time.guest_nice,
        }
    }

    /// Returns the total of active CPU time spent, for this stat measurement
    /// (not iowait, idle, irq or softirq)
    pub fn total_time_jiffies(&self) -> u64 {
        let user = self.user;
        let nice = self.nice;
        let system = self.system;
        let idle = self.idle;
        let irq = self.irq.unwrap_or_default();
        let iowait = self.iowait.unwrap_or_default();
        let softirq = self.softirq.unwrap_or_default();
        let steal = self.steal.unwrap_or_default();
        let guest_nice = self.guest_nice.unwrap_or_default();
        let guest = self.guest.unwrap_or_default();

        trace!(
            "CPUStat contains user {} nice {} system {} idle: {} irq {} softirq {} iowait {} steal {} guest_nice {} guest {}",
            user, nice, system, idle, irq, softirq, iowait, steal, guest_nice, guest
        );
        user + nice + system + guest_nice + guest
    }
}

impl Clone for CPUStat {
    /// Returns a copy of CPUStat instance
    fn clone(&self) -> CPUStat {
        CPUStat {
            user: self.user,
            guest: self.guest,
            guest_nice: self.guest_nice,
            idle: self.idle,
            iowait: self.iowait,
            irq: self.irq,
            nice: self.nice,
            softirq: self.softirq,
            steal: self.steal,
            system: self.system,
        }
    }
}
