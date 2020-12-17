use lazy_static::lazy_static;
use std::collections::VecDeque;
use std::sync::Mutex;

pub trait LogEntry {
    fn log_entry(&self) -> String;
}

pub struct Log {
    entries: VecDeque<String>,
}

impl Log {
    fn create(capacity: usize) -> Log {
        Log {
            entries: VecDeque::with_capacity(capacity),
        }
    }

    fn log<L: LogEntry>(&mut self, entry: L) {
        self.entries.push_back(entry.log_entry())
    }

    fn get_entries(&self) -> &VecDeque<String> {
        &self.entries
    }
}

lazy_static! {
    static ref LOG: Mutex<Log> = Mutex::new(Log::create(10));
}

pub fn log_info<T: LogEntry>(entry: T) {
    LOG.lock().unwrap().log(entry);
}

pub fn log_error<T: LogEntry>(entry: T) {
    log_info(entry);
}

pub fn log_entries() -> VecDeque<String> {
    LOG.lock().unwrap().get_entries().clone()
}

impl LogEntry for &str {
    fn log_entry(&self) -> String {
        return String::from(*self);
    }
}

impl LogEntry for String {
    fn log_entry(&self) -> String {
        LogEntry::log_entry(&self.as_str())
    }
}

#[test]
fn test_static_log() {
    log_info("test");

    let records = log_entries();

    assert!(!records.is_empty());
}
