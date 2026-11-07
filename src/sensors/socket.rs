use std::collections::HashMap;
use std::fs;
use std::mem::size_of_val;
use crate::sensors::core::CPUCore;
use crate::sensors::domain::Domain;
use crate::sensors::record::{CPUStat, Record};
use crate::sensors::traits::{RecordGenerator, RecordReader};
use crate::sensors::units;
use crate::sensors::utils::current_system_time_since_epoch;

// !!!!!!!!!!!!!!!!! CPUSocket !!!!!!!!!!!!!!!!!!!!!!!
/// CPUSocket struct represents a CPU socket (matches physical_id attribute in /proc/cpuinfo),
/// owning CPU cores (processor in /proc/cpuinfo).
#[derive(Debug, Clone)]
pub struct CPUSocket {
    /// Numerical ID of the CPU socket (physical_id in /proc/cpuinfo)
    pub id: u16,
    /// RAPL domains attached to the socket
    pub domains: Vec<Domain>,
    /// Text attributes linked to that socket, found in /proc/cpuinfo
    pub attributes: Vec<Vec<HashMap<String, String>>>,
    /// Path to the file that provides the counter for energy consumed by the socket, in microjoules.
    pub counter_uj_path: String,
    /// Comsumption records measured and stored by glasswing-power for this socket.
    pub record_buffer: Vec<Record>,
    /// Maximum size of the record_buffer in kilobytes.
    pub buffer_max_kbytes: u16,
    /// CPU cores (core_id in /proc/cpuinfo) attached to the socket.
    pub cpu_cores: Vec<CPUCore>,
    /// Usage statistics records stored for this socket.
    pub stat_buffer: Vec<CPUStat>,
    /// Sensor-specific data that has been stored at the topology generation step.
    #[allow(dead_code)]
    pub sensor_data: HashMap<String, String>,
}

impl RecordGenerator for CPUSocket {
    /// Generates a new record of the socket energy consumption and stores it in the record_buffer.
    /// Returns a clone of this Record instance.
    fn refresh_record(&mut self) {
        match self.read_record() {
            Ok(record) => {
                self.record_buffer.push(record);
            }
            Err(e) => {
                warn!(
                    "Couldn't read record from {}, error was: {:?}",
                    self.sensor_data
                        .get("source_file")
                        .unwrap_or(&String::from("SRCFILENOTKNOWN")),
                    e
                );
            }
        }

        if !self.record_buffer.is_empty() {
            self.clean_old_records();
        }
    }

    /// Checks the size in memory of record_buffer and deletes as many Record
    /// instances from the buffer to make it smaller in memory than buffer_max_kbytes.
    fn clean_old_records(&mut self) {
        let record_ptr = &self.record_buffer[0];
        let curr_size = size_of_val(record_ptr) * self.record_buffer.len();
        trace!(
            "socket rebord buffer current size: {} max_bytes: {}",
            curr_size,
            self.buffer_max_kbytes * 1000
        );
        if curr_size > (self.buffer_max_kbytes * 1000) as usize {
            let size_diff = curr_size - (self.buffer_max_kbytes * 1000) as usize;
            trace!(
                "socket record size_diff: {} sizeof: {}",
                size_diff,
                size_of_val(record_ptr)
            );
            if size_diff > size_of_val(record_ptr) {
                let nb_records_to_delete = size_diff as f32 / size_of_val(record_ptr) as f32;
                for _ in 1..nb_records_to_delete as u32 {
                    if !self.record_buffer.is_empty() {
                        let res = self.record_buffer.remove(0);
                        debug!(
                            "Cleaning socket id {} records buffer, removing: {}",
                            self.id, res
                        );
                    }
                }
            }
        }
    }

    /// Returns a new owned Vector being a clone of the current record_buffer.
    /// This does not affect the current buffer but is costly.
    fn get_records_passive(&self) -> Vec<Record> {
        let mut result = vec![];
        for r in &self.record_buffer {
            result.push(Record::new(
                r.timestamp,
                r.value.clone(),
                units::Unit::MicroJoule,
            ));
        }
        result
    }
}

impl CPUSocket {
    /// Creates and returns a CPUSocket instance with an empty buffer and no CPUCore owned yet.
    pub fn new(
        id: u16,
        domains: Vec<Domain>,
        attributes: Vec<Vec<HashMap<String, String>>>,
        counter_uj_path: String,
        buffer_max_kbytes: u16,
        sensor_data: HashMap<String, String>,
    ) -> CPUSocket {
        CPUSocket {
            id,
            domains,
            attributes,
            counter_uj_path,
            record_buffer: vec![], // buffer has to be empty first
            buffer_max_kbytes,
            cpu_cores: vec![], // cores are instantiated on a later step
            stat_buffer: vec![],
            sensor_data,
        }
    }

    pub fn set_id(&mut self, id: u16) {
        self.id = id
    }

    /// Adds a new Domain instance to the domains vector if and only if it doesn't exist in the vector already.
    pub fn safe_add_domain(&mut self, domain: Domain) {
        if !self.domains.iter().any(|d| d.id == domain.id) {
            self.domains.push(domain);
        }
    }

    /// Returns a mutable reference to the domains vector.
    pub fn get_domains(&mut self) -> &mut Vec<Domain> {
        &mut self.domains
    }

    /// Returns a immutable reference to the domains vector.
    pub fn get_domains_passive(&self) -> &Vec<Domain> {
        &self.domains
    }

    /// Returns a mutable reference to the CPU cores vector.
    pub fn get_cores(&mut self) -> &mut Vec<CPUCore> {
        &mut self.cpu_cores
    }

    /// Returns a immutable reference to the CPU cores vector.
    pub fn get_cores_passive(&self) -> &Vec<CPUCore> {
        &self.cpu_cores
    }

    /// Adds a CPU core instance to the cores vector.
    pub fn add_cpu_core(&mut self, core: CPUCore) {
        self.cpu_cores.push(core);
    }

    /// Generates a new CPUStat object storing current usage statistics of the socket
    /// and stores it in the stat_buffer.
    pub fn refresh_stats(&mut self) {
        if !self.stat_buffer.is_empty() {
            self.clean_old_stats();
        }
        self.stat_buffer.insert(0, self.read_stats().unwrap());
    }

    /// Checks the size in memory of stats_buffer and deletes as many CPUStat
    /// instances from the buffer to make it smaller in memory than buffer_max_kbytes.
    fn clean_old_stats(&mut self) {
        let stat_ptr = &self.stat_buffer[0];
        let size_of_stat = size_of_val(stat_ptr);
        let curr_size = size_of_stat * self.stat_buffer.len();
        trace!("current_size of stats in socket {}: {}", self.id, curr_size);
        trace!(
            "estimated max nb of socket stats: {}",
            self.buffer_max_kbytes as f32 * 1000.0 / size_of_stat as f32
        );
        if curr_size > (self.buffer_max_kbytes * 1000) as usize {
            let size_diff = curr_size - (self.buffer_max_kbytes * 1000) as usize;
            trace!(
                "socket {} size_diff: {} size of: {}",
                self.id, size_diff, size_of_stat
            );
            if size_diff > size_of_stat {
                let nb_stats_to_delete = size_diff as f32 / size_of_stat as f32;
                trace!(
                    "socket {} nb_stats_to_delete: {} size_diff: {} size of: {}",
                    self.id, nb_stats_to_delete, size_diff, size_of_stat
                );
                trace!("nb stats to delete: {}", nb_stats_to_delete as u32);
                for _ in 1..nb_stats_to_delete as u32 {
                    if !self.stat_buffer.is_empty() {
                        let res = self.stat_buffer.pop();
                        debug!(
                            "Cleaning stat buffer of socket {}, removing: {:?}",
                            self.id, res
                        );
                    }
                }
            }
        }
    }

    /// Combines stats from all CPU cores owned byu the socket and returns
    /// a CpuStat struct containing stats for the whole socket.
    pub fn read_stats(&self) -> Option<CPUStat> {
        let mut stats = CPUStat {
            user: 0,
            nice: 0,
            system: 0,
            idle: 0,
            iowait: Some(0),
            irq: Some(0),
            softirq: Some(0),
            guest: Some(0),
            guest_nice: Some(0),
            steal: Some(0),
        };
        for c in &self.cpu_cores {
            if let Some(c_stats) = c.read_stats() {
                stats.user += c_stats.user;
                stats.nice += c_stats.nice;
                stats.system += c_stats.system;
                stats.idle += c_stats.idle;
                stats.iowait =
                    Some(stats.iowait.unwrap_or_default() + c_stats.iowait.unwrap_or_default());
                stats.irq = Some(stats.irq.unwrap_or_default() + c_stats.irq.unwrap_or_default());
                stats.softirq =
                    Some(stats.softirq.unwrap_or_default() + c_stats.softirq.unwrap_or_default());
            }
        }
        Some(stats)
    }

    /// Computes the difference between previous usage statistics record for the socket
    /// and the current one. Returns a CPUStat object containing this difference, field
    /// by field.
    pub fn get_stats_diff(&mut self) -> Option<CPUStat> {
        if self.stat_buffer.len() > 1 {
            let last = &self.stat_buffer[0];
            let previous = &self.stat_buffer[1];
            let mut iowait = None;
            let mut irq = None;
            let mut softirq = None;
            let mut steal = None;
            let mut guest = None;
            let mut guest_nice = None;
            if let Some(last_iowait) = last.iowait
                && let Some(previous_iowait) = previous.iowait
            {
                iowait = Some(last_iowait - previous_iowait);
            }
            if let Some(last_irq) = last.irq
                && let Some(previous_irq) = previous.irq
            {
                irq = Some(last_irq - previous_irq);
            }
            if let Some(last_softirq) = last.softirq
                && let Some(previous_softirq) = previous.softirq
            {
                softirq = Some(last_softirq - previous_softirq);
            }
            if let Some(last_steal) = last.steal
                && let Some(previous_steal) = previous.steal
            {
                steal = Some(last_steal - previous_steal);
            }
            if let Some(last_guest) = last.guest
                && let Some(previous_guest) = previous.guest
            {
                guest = Some(last_guest - previous_guest);
            }
            if let Some(last_guest_nice) = last.guest_nice
                && let Some(previous_guest_nice) = previous.guest_nice
            {
                guest_nice = Some(last_guest_nice - previous_guest_nice);
            }
            return Some(CPUStat {
                user: last.user - previous.user,
                nice: last.nice - previous.nice,
                system: last.system - previous.system,
                idle: last.idle - previous.idle,
                iowait,
                irq,
                softirq,
                steal,
                guest,
                guest_nice,
            });
        }
        None
    }

    /// Returns a Record instance containing the power consumed between last
    /// and previous measurement, for this CPU socket
    pub fn get_records_diff_power_microwatts(&self) -> Option<Record> {
        if self.record_buffer.len() > 1 {
            let last_record = self.record_buffer.last().unwrap();
            let previous_record = self
                .record_buffer
                .get(self.record_buffer.len() - 2)
                .unwrap();
            debug!(
                "socket : last_record value: {} previous_record value: {}",
                &last_record.value, &previous_record.value
            );
            let last_rec_val = last_record.value.trim();
            debug!("socket : l1187 : trying to parse {} as u64", last_rec_val);
            let prev_rec_val = previous_record.value.trim();
            debug!("socket : l1189 : trying to parse {} as u64", prev_rec_val);
            if let (Ok(last_microjoules), Ok(previous_microjoules)) =
                (last_rec_val.parse::<u64>(), prev_rec_val.parse::<u64>())
            {
                let mut microjoules = 0;
                if last_microjoules >= previous_microjoules {
                    microjoules = last_microjoules - previous_microjoules;
                } else {
                    debug!(
                        "socket: previous_microjoules ({}) > last_microjoules ({})",
                        previous_microjoules, last_microjoules
                    );
                }
                let time_diff =
                    last_record.timestamp.as_secs_f64() - previous_record.timestamp.as_secs_f64();
                let microwatts = microjoules as f64 / time_diff;
                debug!("socket : l1067: microwatts: {}", microwatts);
                return Some(Record::new(
                    last_record.timestamp,
                    (microwatts as u64).to_string(),
                    units::Unit::MicroWatt,
                ));
            }
        } else {
            info!("Not enough records for socket");
        }
        None
    }

    pub fn get_rapl_mmio_energy_microjoules(&self) -> Option<Record> {
        if let Some(mmio) = self.sensor_data.get("mmio") {
            match &fs::read_to_string(mmio) {
                Ok(val) => {
                    return Some(Record::new(
                        current_system_time_since_epoch(),
                        val.to_string(),
                        units::Unit::MicroJoule,
                    ));
                }
                Err(e) => {
                    debug!("MMIO Error: {:?}", e)
                }
            }
        }
        None
    }
}
