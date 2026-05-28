//! I/O helpers — CSV, JSON, Parquet reading/writing (100% offline, no network).

use std::path::Path;
use polars::prelude::*;
use crate::error::DfResult;
use crate::frame::BonsaiFrame;

// ── CSV ───────────────────────────────────────────────────────────────────────

pub fn read_csv(path: &Path) -> DfResult<BonsaiFrame> {
    let df = CsvReadOptions::default()
        .with_has_header(true)
        .try_into_reader_with_file_path(Some(path.into()))?
        .finish()?;
    Ok(BonsaiFrame::from_polars(df))
}

pub fn write_csv(frame: &BonsaiFrame, path: &Path) -> DfResult<()> {
    let mut file = std::fs::File::create(path)?;
    CsvWriter::new(&mut file)
        .include_header(true)
        .finish(&mut frame.polars().clone())?;
    Ok(())
}

// ── JSON ──────────────────────────────────────────────────────────────────────

pub fn read_json(path: &Path) -> DfResult<BonsaiFrame> {
    let file = std::fs::File::open(path)?;
    let df = JsonReader::new(file).finish()?;
    Ok(BonsaiFrame::from_polars(df))
}

pub fn write_json(frame: &BonsaiFrame, path: &Path) -> DfResult<()> {
    let mut file = std::fs::File::create(path)?;
    let mut df = frame.polars().clone();
    JsonWriter::new(&mut file)
        .with_json_format(JsonFormat::Json)
        .finish(&mut df)?;
    Ok(())
}

// ── Parquet ───────────────────────────────────────────────────────────────────

pub fn read_parquet(path: &Path) -> DfResult<BonsaiFrame> {
    let file = std::fs::File::open(path)?;
    let df = ParquetReader::new(file).finish()?;
    Ok(BonsaiFrame::from_polars(df))
}

pub fn write_parquet(frame: &BonsaiFrame, path: &Path) -> DfResult<()> {
    let file = std::fs::File::create(path)?;
    let mut df = frame.polars().clone();
    ParquetWriter::new(file).finish(&mut df)?;
    Ok(())
}
