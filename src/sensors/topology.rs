use procfs::{CpuInfo, Current, CurrentSI, KernelStats};
use std::collections::HashMap;
use std::fs;
use std::mem::size_of_val;
use sysinfo::{Disks, Pid, System};
use crate::sensors::core::CPUCore;
use crate::sensors::domain::Domain;
use crate::sensors::record::{CPUStat, Record};
use crate::sensors::socket::CPUSocket;
use crate::sensors::traits::{RecordGenerator, RecordReader};
use crate::sensors::units;
use crate::sensors::utils::{IProcess, ProcessTracker, current_system_time_since_epoch};

// !!!!!!!!!!!!!!!!! Topology !!!!!!!!!!!!!!!!!!!!!!!
/// Topology struct represents the whole CPUSocket architecture,
/// from the electricity consumption point of view,
/// including the potentially multiple CPUSocket sockets.
/// Owns a vector of CPUSocket structs representing each socket.
#[derive(Debug, Clone)]
pub struct Topology {
    /// The CPU sockets found on the host, represented as CPUSocket instances attached to this topology
    pub sockets: Vec<CPUSocket>,
    /// ProcessTrack instance that keeps track of processes running on the host and CPU stats associated
    pub proc_tracker: ProcessTracker,
    /// CPU usage stats buffer
    pub stat_buffer: Vec<CPUStat>,
    /// Measurements of energy usage, stored as Record instances
    pub record_buffer: Vec<Record>,
    /// Maximum size in memory for the recor_buffer
    pub buffer_max_kbytes: u16,
    /// Sorted list of all domains names
    pub domains_names: Option<Vec<String>>,
    /// Sensor-specific data needed in the topology
    pub _sensor_data: HashMap<String, String>,
}

impl RecordGenerator for Topology {
    /// Computes a new Record, stores it in the record_buffer
    /// and returns a clone of this record.
    ///
    fn refresh_record(&mut self) {
        match self.read_record() {
            Ok(record) => {
                self.record_buffer.push(record);
            }
            Err(e) => {
                warn!(
                    "Couldn't read record from {}, error was : {:?}",
                    self._sensor_data
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

    /// Removes (and thus drops) as many Record instances from the record_buffer
    /// as needed for record_buffer to not exceed 'buffer_max_kbytes'
    fn clean_old_records(&mut self) {
        let record_ptr = &self.record_buffer[0];
        let record_size = size_of_val(record_ptr);
        let curr_size = record_size * self.record_buffer.len();
        trace!(
            "topology: current size of record buffer: {} max size: {}",
            curr_size,
            self.buffer_max_kbytes * 1000
        );
        if curr_size as u16 > self.buffer_max_kbytes * 1000 {
            let size_diff = curr_size - (self.buffer_max_kbytes * 1000) as usize;
            trace!(
                "topology: size_diff: {} record size: {}",
                size_diff, record_size
            );
            if size_diff > record_size {
                let nb_records_to_delete = size_diff as f32 / record_size as f32;
                for _ in 1..nb_records_to_delete as u32 {
                    if !self.record_buffer.is_empty() {
                        let res = self.record_buffer.remove(0);
                        debug!("Cleaning record buffer on Topology, removing: {:?}", res);
                    }
                }
            }
        }
    }

    /// Returns a copy of the record_buffer
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

impl Default for Topology {
    fn default() -> Self {
        Self::new(HashMap::new())
    }
}

impl Topology {
    /// Instanciates Topology and returns the instance
    pub fn new(sensor_data: HashMap<String, String>) -> Topology {
        Topology {
            sockets: vec![],
            proc_tracker: ProcessTracker::new(5),
            stat_buffer: vec![],
            record_buffer: vec![],
            buffer_max_kbytes: 1,
            domains_names: None,
            _sensor_data: sensor_data,
        }
    }

    /// Parses /proc/cpuinfo and creates instances of CPUCore.
    ///
    ///# Examples
    ///
    /// ```
    /// use glasswing_power::sensors::Topology;
    ///
    /// if let Some(cores) = Topology::generate_cpu_cores() {
    ///     println!("There are {} cores on this host.", cores.len());
    ///     for c in &cores {
    ///         println!("CPU info {:?}", c.attributes);
    ///     }
    /// }
    /// ```
    pub fn generate_cpu_cores() -> Option<Vec<CPUCore>> {
        let mut cores = vec![];

        let sysinfo_system = System::new_all().expect("sysinfo not supported on this OS");
        let sysinfo_cores = sysinfo_system.cpus();
        debug!("Sysinfo sees {}", sysinfo_cores.len());
        let cpuinfo = CpuInfo::current().unwrap();
        for (id, c) in (0_u16..).zip(sysinfo_cores.iter()) {
            let mut info = HashMap::<String, String>::new();
            {
                for (k, v) in cpuinfo.get_info(id as usize).unwrap().iter() {
                    info.insert(String::from(*k), String::from(*v));
                }
            }
            info.insert(String::from("frequency"), c.frequency().to_string());
            info.insert(String::from("name"), c.name().to_string());
            info.insert(String::from("vendor_id"), c.vendor_id().to_string());
            info.insert(String::from("brand"), c.brand().to_string());
            cores.push(CPUCore::new(id, info));
        }
        Some(cores)
    }

    /// Adds a Socket instance to self.sockets if and only if the
    /// socket id doesn't exist already.
    pub fn safe_add_socket(
        &mut self,
        socket_id: u16,
        domains: Vec<Domain>,
        attributes: Vec<Vec<HashMap<String, String>>>,
        counter_uj_path: String,
        buffer_max_kbytes: u16,
        sensor_data: HashMap<String, String>,
    ) -> Option<CPUSocket> {
        if !self.sockets.iter().any(|s| s.id == socket_id) {
            let socket = CPUSocket::new(
                socket_id,
                domains,
                attributes,
                counter_uj_path,
                buffer_max_kbytes,
                sensor_data,
            );
            let res = socket.clone();
            self.sockets.push(socket);
            Some(res)
        } else {
            None
        }
    }

    pub fn safe_insert_socket(&mut self, socket: CPUSocket) {
        if !self.sockets.iter().any(|s| s.id == socket.id) {
            self.sockets.push(socket);
        }
    }

    /// Returns a immutable reference to self.proc_tracker
    pub fn get_proc_tracker(&self) -> &ProcessTracker {
        &self.proc_tracker
    }

    /// Returns a mutable reference to self.sockets
    pub fn get_sockets(&mut self) -> &mut Vec<CPUSocket> {
        &mut self.sockets
    }

    /// Returns an immutable reference to self.sockets
    pub fn get_sockets_passive(&self) -> &Vec<CPUSocket> {
        &self.sockets
    }

    // Build a sorted list of all domains names from all sockets.
    fn build_domains_names(&mut self) {
        let mut names: HashMap<String, ()> = HashMap::new();
        for s in self.sockets.iter() {
            for d in s.get_domains_passive() {
                names.insert(d.name.clone(), ());
            }
        }
        let mut domain_names = names.keys().cloned().collect::<Vec<String>>();
        domain_names.sort();
        self.domains_names = Some(domain_names);
    }

    pub fn set_domains_names(&mut self, names: Vec<String>) {
        self.domains_names = Some(names);
    }

    /// Adds a Domain instance to a given socket, if and only if the domain
    /// id doesn't exist already for the socket.
    pub fn safe_add_domain_to_socket(
        &mut self,
        socket_id: u16,
        domain_id: u16,
        name: &str,
        uj_counter: &str,
        buffer_max_kbytes: u16,
        sensor_data: HashMap<String, String>,
    ) {
        let iterator = self.sockets.iter_mut();
        for socket in iterator {
            if socket.id == socket_id {
                socket.safe_add_domain(Domain::new(
                    domain_id,
                    String::from(name),
                    String::from(uj_counter),
                    buffer_max_kbytes,
                    sensor_data.clone(),
                ));
            }
        }
        self.build_domains_names();
    }

    /// Generates CPUCore instances for the host and adds them
    /// to appropriate CPUSocket instance from self.sockets
    pub fn add_cpu_cores(&mut self) {
        if let Some(mut cores) = Topology::generate_cpu_cores() {
            while let Some(c) = cores.pop() {
                let socket_id = &c
                    .attributes
                    .get("physical id")
                    .unwrap()
                    .parse::<u16>()
                    .unwrap();
                let socket_match = self.sockets.iter_mut().find(|x| &x.id == socket_id);

                //In VMs there might be a missmatch betwen Sockets and Cores - see Issue#133 as a first fix we just map all cores that can't be mapped to the first
                let socket = match socket_match {
                    Some(x) => x,
                    None =>self.sockets.first_mut().expect("Trick: if you are running on a vm, do not forget to use --vm parameter invoking glasswing-power at the command line")
                };

                if socket_id == &socket.id {
                    socket.add_cpu_core(c);
                } else {
                    socket.add_cpu_core(c);
                    warn!(
                        "coud't not match core to socket - mapping to first socket instead - if you are not using --vm there is something wrong"
                    )
                }
            }

        } else {
            panic!("Couldn't retrieve any CPU Core from the topology. (generate_cpu_cores)");
        }
    }

    /// Triggers ProcessTracker refresh on process stats
    /// and power consumption, CPU stats and cores power comsumption,
    /// CPU sockets stats and power consumption.
    pub fn refresh(&mut self) {
        let sockets = &mut self.sockets;
        for s in sockets {
            // refresh each socket with new record
            s.refresh_record();
            s.refresh_stats();
            let domains = s.get_domains();
            for d in domains {
                d.refresh_record();
            }
            //let cores = s.get_cores();
            //for c in cores {
            //
            //}
        }
        self.proc_tracker.refresh();
        self.refresh_procs();
        self.refresh_record();
        self.refresh_stats();
    }

    /// Gets currently running processes (as procfs::Process instances) and stores
    /// them in self.proc_tracker
    fn refresh_procs(&mut self) {
        {
            let pt = &mut self.proc_tracker;
            pt.sysinfo.refresh_processes(sysinfo::ProcessesToUpdate::All, true);
            let current_procs = pt
                .sysinfo
                .processes()
                .values()
                .map(IProcess::new)
                .collect::<Vec<_>>();
            for p in current_procs {
                if let Err(msg) = pt.add_process_record(p) {
                    error!("Failed to track process: {}", msg);
                }
            }
        }
    }

    /// Gets currents stats and stores them as a CPUStat instance in self.stat_buffer
    pub fn refresh_stats(&mut self) {
        if let Some(stats) = self.read_stats() {
            self.stat_buffer.insert(0, stats);
            if !self.stat_buffer.is_empty() {
                self.clean_old_stats();
            }
        } else {
            debug!("read_stats() is None");
        }
    }

    /// Checks the size in memory of stats_buffer and deletes as many CPUStat
    /// instances from the buffer to make it smaller in memory than buffer_max_kbytes.
    fn clean_old_stats(&mut self) {
        let stat_ptr = &self.stat_buffer[0];
        let size_of_stat = size_of_val(stat_ptr);
        let curr_size = size_of_stat * self.stat_buffer.len();
        trace!("current_size of stats in topo: {}", curr_size);
        if curr_size > (self.buffer_max_kbytes * 1000) as usize {
            let size_diff = curr_size - (self.buffer_max_kbytes * 1000) as usize;
            if size_diff > size_of_stat {
                let nb_stats_to_delete = size_diff as f32 / size_of_stat as f32;
                trace!(
                    "nb_stats_to_delete: {} size_diff: {} size of: {}",
                    nb_stats_to_delete, size_diff, size_of_stat
                );
                for _ in 1..nb_stats_to_delete as u32 {
                    if !self.stat_buffer.is_empty() {
                        let res = self.stat_buffer.pop();
                        debug!("Cleaning topology stat buffer, removing: {:?}", res);
                    }
                }
            }
        }
    }

    /// Returns a Record instance containing the difference (attribute by attribute, except timestamp which will be the timestamp from the last record)
    /// between the last (in time) record from self.record_buffer and the previous one
    pub fn get_records_diff(&self) -> Option<Record> {
        let len = self.record_buffer.len();
        if len > 2 {
            let last = self.record_buffer.last().unwrap();
            let previous = self.record_buffer.get(len - 2).unwrap();
            let last_value = last.value.parse::<u64>().unwrap();
            let previous_value = previous.value.parse::<u64>().unwrap();
            if previous_value <= last_value {
                let diff = last_value - previous_value;
                return Some(Record::new(last.timestamp, diff.to_string(), last.unit));
            }
        }
        None
    }

    /// Returns a Record instance containing the power consumed between
    /// last and previous measurement, in microwatts.
    pub fn get_records_diff_power_microwatts(&self) -> Option<Record> {
        if self.record_buffer.len() > 1 {
            let last_record = self.record_buffer.last().unwrap();
            let previous_record = self
                .record_buffer
                .get(self.record_buffer.len() - 2)
                .unwrap();
            match previous_record.value.trim().parse::<u128>() {
                Ok(previous_microjoules) => match last_record.value.trim().parse::<u128>() {
                    Ok(last_microjoules) => {
                        if previous_microjoules > last_microjoules {
                            return None;
                        }
                        let microjoules = last_microjoules - previous_microjoules;
                        let time_diff = last_record.timestamp.as_secs_f64()
                            - previous_record.timestamp.as_secs_f64();
                        let microwatts = microjoules as f64 / time_diff;
                        return Some(Record::new(
                            last_record.timestamp,
                            (microwatts as u64).to_string(),
                            units::Unit::MicroWatt,
                        ));
                    }
                    Err(e) => {
                        warn!(
                            "Couldn't get previous_microjoules - value : '{}' - error : {:?}",
                            previous_record.value, e
                        );
                    }
                },
                Err(e) => {
                    warn!(
                        "Couldn't parse previous_microjoules - value : '{}' - error : {:?}",
                        previous_record.value.trim(),
                        e
                    );
                }
            }
        }
        None
    }

    /// Returns a CPUStat instance containing the difference between last
    /// and previous stats measurement (from stat_buffer), attribute by attribute.
    pub fn get_stats_diff(&self) -> Option<CPUStat> {
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
                && let Some(prev_iowait) = previous.iowait
            {
                iowait = Some(last_iowait - prev_iowait);
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

    /// Reads content from /proc/stat and extracts the stats of the whole CPU topology.
    pub fn read_stats(&self) -> Option<CPUStat> {
        let kernelstats_or_not = KernelStats::current();
        if let Ok(res_cputime) = kernelstats_or_not {
            return Some(CPUStat {
                user: res_cputime.total.user,
                guest: res_cputime.total.guest,
                guest_nice: res_cputime.total.guest_nice,
                idle: res_cputime.total.idle,
                iowait: res_cputime.total.iowait,
                irq: res_cputime.total.irq,
                nice: res_cputime.total.nice,
                softirq: res_cputime.total.softirq,
                steal: res_cputime.total.steal,
                system: res_cputime.total.system,
            });
        }
        None
    }

    /// Returns the number of processes currently available
    pub fn read_nb_process_total_count(&self) -> Option<u64> {
        if let Ok(result) = KernelStats::current() {
            return Some(result.processes);
        }
        None
    }

    /// Returns the number of processes currently in a running state
    pub fn read_nb_process_running_current(&self) -> Option<u32> {
        if let Ok(result) = KernelStats::current()
            && let Some(procs_running) = result.procs_running
        {
            return Some(procs_running);
        }
        None
    }
    /// Returns the number of processes currently blocked waiting
    pub fn read_nb_process_blocked_current(&self) -> Option<u32> {
        if let Ok(result) = KernelStats::current()
            && let Some(procs_blocked) = result.procs_blocked
        {
            return Some(procs_blocked);
        }
        None
    }
    /// Returns the current number of context switches
    pub fn read_nb_context_switches_total_count(&self) -> Option<u64> {
        if let Ok(result) = KernelStats::current() {
            return Some(result.ctxt);
        }
        None
    }

    pub fn get_cpu_frequency(&self) -> Record {
        Record::new(
            current_system_time_since_epoch(),
            self.proc_tracker.get_cpu_frequency().to_string(),
            units::Unit::MegaHertz,
        )
    }

    pub fn get_load_avg(&self) -> Option<Vec<Record>> {
        let load = sysinfo::System::load_average().expect("failed to get load average");
        let timestamp = current_system_time_since_epoch();
        Some(vec![
            Record::new(timestamp, load.one.to_string(), units::Unit::Numeric),
            Record::new(timestamp, load.five.to_string(), units::Unit::Numeric),
            Record::new(timestamp, load.fifteen.to_string(), units::Unit::Numeric),
        ])
    }

    pub fn get_disks(&self) -> HashMap<String, (String, HashMap<String, String>, Record)> {
        let timestamp = current_system_time_since_epoch();
        let mut res = HashMap::new();
        let disks = Disks::new_with_refreshed_list();
        for d in disks.iter().flatten() {
            let mut attributes = HashMap::new();
            if let Some(file_system) = d.file_system().to_str() {
                attributes.insert(String::from("disk_file_system"), String::from(file_system));
            }
            if let Some(mount_point) = d.mount_point().to_str() {
                attributes.insert(String::from("disk_mount_point"), String::from(mount_point));
            }
            attributes.insert(
                String::from("disk_is_removable"),
                d.is_removable().to_string(),
            );
            if let Some(disk_name) = d.name().to_str() {
                attributes.insert(String::from("disk_name"), String::from(disk_name));
            }
            res.insert(
                String::from("scaph_host_disk_total_bytes"),
                (
                    String::from("Total disk size, in bytes."),
                    attributes.clone(),
                    Record::new(timestamp, d.total_space().to_string(), units::Unit::Bytes),
                ),
            );
            res.insert(
                String::from("scaph_host_disk_available_bytes"),
                (
                    String::from("Available disk space, in bytes."),
                    attributes.clone(),
                    Record::new(
                        timestamp,
                        d.available_space().to_string(),
                        units::Unit::Bytes,
                    ),
                ),
            );
        }
        res
    }

    pub fn get_total_memory_bytes(&self) -> Record {
        Record {
            timestamp: current_system_time_since_epoch(),
            value: self.proc_tracker.sysinfo.total_memory().to_string(),
            unit: units::Unit::Bytes,
        }
    }

    pub fn get_available_memory_bytes(&self) -> Record {
        Record {
            timestamp: current_system_time_since_epoch(),
            value: self.proc_tracker.sysinfo.available_memory().to_string(),
            unit: units::Unit::Bytes,
        }
    }

    pub fn get_free_memory_bytes(&self) -> Record {
        Record {
            timestamp: current_system_time_since_epoch(),
            value: self.proc_tracker.sysinfo.free_memory().to_string(),
            unit: units::Unit::Bytes,
        }
    }

    pub fn get_total_swap_bytes(&self) -> Record {
        Record {
            timestamp: current_system_time_since_epoch(),
            value: self.proc_tracker.sysinfo.total_swap().to_string(),
            unit: units::Unit::Bytes,
        }
    }

    pub fn get_free_swap_bytes(&self) -> Record {
        Record {
            timestamp: current_system_time_since_epoch(),
            value: self.proc_tracker.sysinfo.free_swap().to_string(),
            unit: units::Unit::Bytes,
        }
    }

    /// Returns the power consumed between last and previous measurement for a given process ID, in microwatts
    pub fn get_process_power_consumption_microwatts(&self, pid: Pid) -> Option<Record> {
        if let Some(record) = self.get_proc_tracker().get_process_last_record(pid) {
            let process_cpu_percentage = self.get_process_cpu_usage_percentage(pid).unwrap();
            let topo_conso = self.get_records_diff_power_microwatts();
            if let Some(conso) = &topo_conso {
                let conso_f64 = conso.value.parse::<f64>().unwrap();
                let result =
                    (conso_f64 * process_cpu_percentage.value.parse::<f64>().unwrap()) / 100.0_f64;
                return Some(Record::new(
                    record.timestamp,
                    result.to_string(),
                    units::Unit::MicroWatt,
                ));
            }
        } else {
            trace!("Couldn't find records for PID: {}", pid);
        }
        None
    }

    pub fn get_all_per_process(&self, pid: Pid) -> Option<HashMap<String, (String, Record)>> {
        let mut res = HashMap::new();
        if let Some(record) = self.get_proc_tracker().get_process_last_record(pid) {
            let process_cpu_percentage =
                record.process.cpu_usage_percentage / self.proc_tracker.nb_cores as f32;
            res.insert(
                String::from("scaph_process_cpu_usage_percentage"),
                (String::from("CPU time consumed by the process, as a percentage of the capacity of all the CPU Cores"),
                Record::new(
                    record.timestamp,
                    process_cpu_percentage.to_string(),
                    units::Unit::Percentage,
                    )
                )
            );
            res.insert(
                String::from("scaph_process_memory_virtual_bytes"),
                (
                    String::from("Virtual RAM usage by the process, in bytes"),
                    Record::new(
                        record.timestamp,
                        record.process.virtual_memory.to_string(),
                        units::Unit::Percentage,
                    ),
                ),
            );
            res.insert(
                String::from("scaph_process_memory_bytes"),
                (
                    String::from("Physical RAM usage by the process, in bytes"),
                    Record::new(
                        record.timestamp,
                        record.process.memory.to_string(),
                        units::Unit::Bytes,
                    ),
                ),
            );
            res.insert(
                String::from("scaph_process_disk_write_bytes"),
                (
                    String::from("Data written on disk by the process, in bytes"),
                    Record::new(
                        record.timestamp,
                        record.process.disk_written.to_string(),
                        units::Unit::Bytes,
                    ),
                ),
            );
            res.insert(
                String::from("scaph_process_disk_read_bytes"),
                (
                    String::from("Data read on disk by the process, in bytes"),
                    Record::new(
                        record.timestamp,
                        record.process.disk_read.to_string(),
                        units::Unit::Bytes,
                    ),
                ),
            );
            res.insert(
                String::from("scaph_process_disk_total_write_bytes"),
                (
                    String::from("Total data written on disk by the process, in bytes"),
                    Record::new(
                        record.timestamp,
                        record.process.total_disk_written.to_string(),
                        units::Unit::Bytes,
                    ),
                ),
            );
            res.insert(
                String::from("scaph_process_disk_total_read_bytes"),
                (
                    String::from("Total data read on disk by the process, in bytes"),
                    Record::new(
                        record.timestamp,
                        record.process.total_disk_read.to_string(),
                        units::Unit::Bytes,
                    ),
                ),
            );
            let topo_conso = self.get_records_diff_power_microwatts();
            if let Some(conso) = &topo_conso {
                let conso_f64 = conso.value.parse::<f64>().unwrap();
                let result = (conso_f64 * process_cpu_percentage as f64) / 100.0_f64;
                res.insert(
                    String::from("scaph_process_power_consumption_microwatts"),
                    (
                        String::from("Total data read on disk by the process, in bytes"),
                        Record::new(record.timestamp, result.to_string(), units::Unit::MicroWatt),
                    ),
                );
            }
        }
        Some(res)
    }

    // Per process metrics, from ProcessRecord during last refresh, returned in Record structs

    pub fn get_process_cpu_usage_percentage(&self, pid: Pid) -> Option<Record> {
        if let Some(record) = self.get_proc_tracker().get_process_last_record(pid) {
            return Some(Record::new(
                record.timestamp,
                (record.process.cpu_usage_percentage / self.proc_tracker.nb_cores as f32)
                    .to_string(),
                units::Unit::Percentage,
            ));
        }
        None
    }

    pub fn get_process_memory_virtual_bytes(&self, pid: Pid) -> Option<Record> {
        if let Some(record) = self.get_proc_tracker().get_process_last_record(pid) {
            return Some(Record::new(
                record.timestamp,
                record.process.virtual_memory.to_string(),
                units::Unit::Bytes,
            ));
        }
        None
    }

    pub fn get_process_memory_bytes(&self, pid: Pid) -> Option<Record> {
        if let Some(record) = self.get_proc_tracker().get_process_last_record(pid) {
            return Some(Record::new(
                record.timestamp,
                record.process.memory.to_string(),
                units::Unit::Bytes,
            ));
        }
        None
    }

    pub fn get_process_disk_written_bytes(&self, pid: Pid) -> Option<Record> {
        if let Some(record) = self.get_proc_tracker().get_process_last_record(pid) {
            return Some(Record::new(
                record.timestamp,
                record.process.disk_written.to_string(),
                units::Unit::Bytes,
            ));
        }
        None
    }

    pub fn get_process_disk_read_bytes(&self, pid: Pid) -> Option<Record> {
        if let Some(record) = self.get_proc_tracker().get_process_last_record(pid) {
            return Some(Record::new(
                record.timestamp,
                record.process.disk_read.to_string(),
                units::Unit::Bytes,
            ));
        }
        None
    }
    pub fn get_process_disk_total_read_bytes(&self, pid: Pid) -> Option<Record> {
        if let Some(record) = self.get_proc_tracker().get_process_last_record(pid) {
            return Some(Record::new(
                record.timestamp,
                record.process.total_disk_read.to_string(),
                units::Unit::Bytes,
            ));
        }
        None
    }

    pub fn get_process_disk_total_write_bytes(&self, pid: Pid) -> Option<Record> {
        if let Some(record) = self.get_proc_tracker().get_process_last_record(pid) {
            return Some(Record::new(
                record.timestamp,
                record.process.total_disk_written.to_string(),
                units::Unit::Bytes,
            ));
        }
        None
    }

    pub fn get_rapl_psys_energy_microjoules(&self) -> Option<Record> {
        if let Some(psys) = self._sensor_data.get("psys") {
            match &fs::read_to_string(format!("{psys}/energy_uj")) {
                Ok(val) => {
                    debug!("Read PSYS from {psys}/energy_uj: {}", val.to_string());
                    return Some(Record::new(
                        current_system_time_since_epoch(),
                        val.to_string(),
                        units::Unit::MicroJoule,
                    ));
                }
                Err(e) => {
                    warn!("PSYS Error: {:?}", e);
                }
            }
        } else {
            debug!("Asked for PSYS but there is no psys entry in sensor_data.");
        }
        None
    }

}
