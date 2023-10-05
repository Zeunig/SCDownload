use std::{time::SystemTime, fs::OpenOptions, io::Write};

// Simple logging made by Zeunig

#[allow(dead_code)]
#[derive(Debug, PartialEq)]
pub enum Severities {
    DEBUG,
    OKAY,
    INFO,
    WARNING,
    ERROR,
    CRITICAL,
}

#[track_caller]
pub fn logging<T: ToString>(severity: Severities, text: T) {
    if !(cfg!(debug_assertions)) && severity == Severities::DEBUG {
        return;
    }
    let trace = std::panic::Location::caller();
    let msg = format!(
        "SCDOWNLOAD | {} | {:?} | {} | {}",
        SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_secs_f64(),
        severity,
        trace,
        text.to_string()
    );
    if severity == Severities::ERROR || severity == Severities::CRITICAL {
        let mut p = OpenOptions::new().write(true).create(true).open("errors.log").unwrap();
        let _ = p.write_all(msg.as_bytes());
    }
    println!("{}", msg);
}