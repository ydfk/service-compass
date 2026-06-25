use std::{
    collections::VecDeque,
    fs::{self, OpenOptions},
    io::{self, Write},
    path::{Path, PathBuf},
    sync::{Mutex, OnceLock},
};

use chrono::{Duration, NaiveDate, Utc};
use serde::Serialize;

const MAX_LOGS: usize = 1_000;
static LOGS: OnceLock<Mutex<VecDeque<String>>> = OnceLock::new();
static FILE_LOGS: OnceLock<FileLogConfig> = OnceLock::new();
static LAST_CLEANUP: OnceLock<Mutex<NaiveDate>> = OnceLock::new();

#[derive(Clone)]
struct FileLogConfig {
    dir: PathBuf,
    retention_days: i64,
}

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
        append_file(&line);
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

pub fn init(log_dir: &Path, retention_days: i64) -> io::Result<()> {
    fs::create_dir_all(log_dir)?;
    cleanup_files(log_dir, retention_days.max(1))?;
    let _ = FILE_LOGS.set(FileLogConfig {
        dir: log_dir.to_path_buf(),
        retention_days: retention_days.max(1),
    });
    let _ = LAST_CLEANUP.set(Mutex::new(Utc::now().date_naive()));
    Ok(())
}

fn append_file(line: &str) {
    let Some(config) = FILE_LOGS.get() else {
        return;
    };
    let date = Utc::now().date_naive();
    let path = config
        .dir
        .join(format!("service-compass-{}.log", date.format("%Y-%m-%d")));
    let Ok(mut file) = OpenOptions::new().create(true).append(true).open(path) else {
        return;
    };
    let _ = writeln!(file, "{line}");
    let mut last_cleanup = LAST_CLEANUP
        .get_or_init(|| Mutex::new(date))
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    if *last_cleanup != date {
        let _ = cleanup_files(&config.dir, config.retention_days);
        *last_cleanup = date;
    }
}

fn cleanup_files(log_dir: &Path, retention_days: i64) -> io::Result<()> {
    let cutoff = Utc::now().date_naive() - Duration::days(retention_days);
    for entry in fs::read_dir(log_dir)? {
        let entry = entry?;
        let path = entry.path();
        if log_date(&path).is_some_and(|date| date < cutoff) {
            let _ = fs::remove_file(path);
        }
    }
    Ok(())
}

fn log_date(path: &Path) -> Option<NaiveDate> {
    let name = path.file_name()?.to_str()?;
    let date = name
        .strip_prefix("service-compass-")?
        .strip_suffix(".log")?;
    NaiveDate::parse_from_str(date, "%Y-%m-%d").ok()
}
