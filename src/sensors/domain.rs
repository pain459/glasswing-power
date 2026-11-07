use std::collections::HashMap;
use std::fs;
use std::fmt;
use crate::sensors::record::Record;
use crate::sensors::traits::{RecordGenerator, RecordReader};
use crate::sensors::units;
use crate::sensors::utils::current_system_time_since_epoch;

// !!!!!!!!!!!!!!!!! Domain !!!!!!!!!!!!!!!!!!!!!!!
/// Domain struct represents a part of a CPUSocket from the
/// electricity consumption point of view.
#[derive(Debug, Clone)]
pub struct Domain {
    /// Numerical ID of the RAPL domain as indicated in /sys/class/powercap/intel-rapl* folders names
    pub id: u16,
    /// Name of the domain as found in /sys/class/powercap/intel-rapl:X:X/name
    pub name: String,
    /// Path to the domain's energy counter file, microjoules extracted
    pub counter_uj_path: String,
    /// History of energy consumption measurements, stored as Record instances
    pub record_buffer: Vec<Record>,
    /// Maximum size of record_buffer, in kilobytes
    pub buffer_max_kbytes: u16,
    /// Sensor-specific data that has been stored at the topology generation step.
    #[allow(dead_code)]
    pub sensor_data: HashMap<String, String>,
}
impl RecordGenerator for Domain {
    /// Computes a measurement of energy comsumption for this CPU domain,
    /// stores a copy in self.record_buffer and returns it.
    fn refresh_record(&mut self) {
        match self.read_record() {
            Ok(record) => {
                self.record_buffer.push(record);
            }
            Err(e) => {
                warn!(
                    "Couldn't read record from {}. Error was : {:?}.",
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

    /// Removes as many Record instances from self.record_buffer as needed
    /// for record_buffer to take less than 'buffer_max_kbytes' in memory
    fn clean_old_records(&mut self) {
        let record_ptr = &self.record_buffer[0];
        let curr_size = size_of_val(record_ptr) * self.record_buffer.len();
        if curr_size > (self.buffer_max_kbytes * 1000) as usize {
            let size_diff = curr_size - (self.buffer_max_kbytes * 1000) as usize;
            if size_diff > size_of_val(&self.record_buffer[0]) {
                let nb_records_to_delete =
                    size_diff as f32 / size_of_val(&self.record_buffer[0]) as f32;
                for _ in 1..nb_records_to_delete as u32 {
                    if !self.record_buffer.is_empty() {
                        self.record_buffer.remove(0);
                    }
                }
            }
        }
    }

    /// Returns a copy of self.record_buffer
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
impl Domain {
    /// Instanciates Domain and returns the instance
    pub fn new(
        id: u16,
        name: String,
        counter_uj_path: String,
        buffer_max_kbytes: u16,
        sensor_data: HashMap<String, String>,
    ) -> Domain {
        Domain {
            id,
            name,
            counter_uj_path,
            record_buffer: vec![],
            buffer_max_kbytes,
            sensor_data,
        }
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
            if let (Ok(last_microjoules), Ok(previous_microjoules)) = (
                last_record.value.trim().parse::<u64>(),
                previous_record.value.trim().parse::<u64>(),
            ) {
                if previous_microjoules > last_microjoules {
                    return None;
                }
                let microjoules = last_microjoules - previous_microjoules;
                let time_diff =
                    last_record.timestamp.as_secs_f64() - previous_record.timestamp.as_secs_f64();
                let microwatts = microjoules as f64 / time_diff;
                return Some(Record::new(
                    last_record.timestamp,
                    (microwatts as u64).to_string(),
                    units::Unit::MicroWatt,
                ));
            }
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
                    debug!("MMIO Error in get microjoules: {:?}", e);
                }
            }
        }
        None
    }
}
impl fmt::Display for Domain {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Domain: {}", self.name)
    }
}
