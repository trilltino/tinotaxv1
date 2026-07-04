use std::fs::File;
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::marker::PhantomData;

use anyhow::{Context, Result};
use camino::Utf8Path;
use serde::de::DeserializeOwned;
use serde::Serialize;

/// One-record-per-line JSON writer. JSONL keeps derived data streamable,
/// appendable, and diffable.
pub struct JsonlWriter<T> {
    writer: BufWriter<File>,
    written: u64,
    _marker: PhantomData<T>,
}

impl<T: Serialize> JsonlWriter<T> {
    /// Truncate/create the file.
    pub fn create(path: &Utf8Path) -> Result<Self> {
        let file = File::create(path).with_context(|| format!("creating {path}"))?;
        Ok(Self {
            writer: BufWriter::new(file),
            written: 0,
            _marker: PhantomData,
        })
    }

    pub fn write(&mut self, record: &T) -> Result<()> {
        serde_json::to_writer(&mut self.writer, record)?;
        self.writer.write_all(b"\n")?;
        self.written += 1;
        Ok(())
    }

    pub fn written(&self) -> u64 {
        self.written
    }

    pub fn finish(mut self) -> Result<u64> {
        self.writer.flush()?;
        Ok(self.written)
    }
}

pub fn read_jsonl<T: DeserializeOwned>(path: &Utf8Path) -> Result<Vec<T>> {
    let file = File::open(path).with_context(|| format!("opening {path}"))?;
    let reader = BufReader::new(file);
    let mut records = Vec::new();
    for (i, line) in reader.lines().enumerate() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        let record = serde_json::from_str(&line)
            .with_context(|| format!("parsing {path} line {}", i + 1))?;
        records.push(record);
    }
    Ok(records)
}
