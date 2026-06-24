use std::{
    collections::VecDeque,
    io::{self, Write},
    sync::{Mutex, OnceLock},
};

use serde::Serialize;

const MAX_LOGS: usize = 1_000;
static LOGS: OnceLock<Mutex<VecDeque<String>>> = OnceLock::new();

#[derive(Clone, Copy)]
pub struct LogWriterFactory;

pub struct LogWriter {
    buffer: Vec<u8>,
}

#[derive(Serialize)]
pub struct LogEntry {
    pub line: String,
}

impl<'a> tracing_subscriber::fmt::MakeWriter<'a> for LogWriterFactory {
    type Writer = LogWriter;

    fn make_writer(&'a self) -> Self::Writer {
        LogWriter { buffer: Vec::new() }
    }
}

impl Write for LogWriter {
    fn write(&mut self, buffer: &[u8]) -> io::Result<usize> {
        io::stdout().write_all(buffer)?;
        self.buffer.extend_from_slice(buffer);
        Ok(buffer.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        io::stdout().flush()
    }
}

impl Drop for LogWriter {
    fn drop(&mut self) {
        let line = String::from_utf8_lossy(&self.buffer).trim().to_string();
        if line.is_empty() {
            return;
        }
        let mut logs = LOGS
            .get_or_init(|| Mutex::new(VecDeque::with_capacity(MAX_LOGS)))
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if logs.len() == MAX_LOGS {
            logs.pop_front();
        }
        logs.push_back(line);
    }
}

pub fn recent(limit: usize) -> Vec<LogEntry> {
    LOGS.get_or_init(|| Mutex::new(VecDeque::with_capacity(MAX_LOGS)))
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .iter()
        .rev()
        .take(limit.min(MAX_LOGS))
        .map(|line| LogEntry { line: line.clone() })
        .collect()
}
