use crate::{CNCTech, LaserTech, Printer3DTech};
use anyhow::Result;
use serde::{Deserialize, Serialize};

/// CNC Machine Adapter
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CNCAdapter {
    pub device_id: String,
    pub cnc_type: CNCTech,
    pub axis_count: u8,
    pub spindle_speed_rpm: u32,
    pub feed_rate_mm_min: f32,
}

impl CNCAdapter {
    pub fn new(device_id: String, cnc_type: CNCTech, axis_count: u8) -> Self {
        CNCAdapter {
            device_id,
            cnc_type,
            axis_count,
            spindle_speed_rpm: 10000,
            feed_rate_mm_min: 500.0,
        }
    }

    pub async fn load_tool(&self, tool_number: u8) -> Result<()> {
        tracing::debug!("CNC: Loading tool #{}", tool_number);
        Ok(())
    }

    pub async fn move_axes(&self, x: f32, y: f32, z: f32) -> Result<()> {
        tracing::debug!("CNC: Moving to ({}, {}, {})", x, y, z);
        Ok(())
    }

    pub async fn execute_gcode(&self, gcode_lines: Vec<String>) -> Result<u32> {
        tracing::debug!("CNC: Executing {} lines of G-code", gcode_lines.len());
        Ok(gcode_lines.len() as u32)
    }
}

/// Laser Adapter
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LaserAdapter {
    pub device_id: String,
    pub laser_type: LaserTech,
    pub power_w: u32,
    pub wavelength_nm: u16,
    pub pulse_frequency_hz: Option<u32>,
}

impl LaserAdapter {
    pub fn new(device_id: String, laser_type: LaserTech, power_w: u32) -> Self {
        LaserAdapter {
            device_id,
            laser_type,
            power_w,
            wavelength_nm: 1064,
            pulse_frequency_hz: None,
        }
    }

    pub async fn enable_laser(&self) -> Result<()> {
        tracing::debug!("Laser: Enabling {} W laser", self.power_w);
        Ok(())
    }

    pub async fn disable_laser(&self) -> Result<()> {
        tracing::debug!("Laser: Disabling laser");
        Ok(())
    }

    pub async fn execute_cut(&self, path: Vec<(f32, f32)>) -> Result<()> {
        tracing::debug!("Laser: Executing cut with {} waypoints", path.len());
        Ok(())
    }

    pub async fn execute_engrave(&self, path: Vec<(f32, f32, f32)>) -> Result<()> {
        tracing::debug!("Laser: Executing engrave with {} waypoints", path.len());
        Ok(())
    }
}

/// 3D Printer Adapter
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrinterAdapter {
    pub device_id: String,
    pub printer_type: Printer3DTech,
    pub nozzle_diameter_mm: f32,
    pub bed_size_mm: (f32, f32, f32),
    pub max_nozzle_temp_c: u32,
}

impl PrinterAdapter {
    pub fn new(device_id: String, printer_type: Printer3DTech, bed_size: (f32, f32, f32)) -> Self {
        PrinterAdapter {
            device_id,
            printer_type,
            nozzle_diameter_mm: 0.4,
            bed_size_mm: bed_size,
            max_nozzle_temp_c: 300,
        }
    }

    pub async fn heat_bed(&self, temp_c: u32) -> Result<()> {
        tracing::debug!("Printer: Heating bed to {}°C", temp_c);
        Ok(())
    }

    pub async fn heat_nozzle(&self, temp_c: u32) -> Result<()> {
        tracing::debug!("Printer: Heating nozzle to {}°C", temp_c);
        Ok(())
    }

    pub async fn home_axes(&self) -> Result<()> {
        tracing::debug!("Printer: Homing all axes");
        Ok(())
    }

    pub async fn execute_gcode(&self, gcode_lines: Vec<String>) -> Result<u32> {
        tracing::debug!("Printer: Executing {} lines of G-code", gcode_lines.len());
        Ok(gcode_lines.len() as u32)
    }

    pub async fn extrude(&self, length_mm: f32) -> Result<()> {
        tracing::debug!("Printer: Extruding {} mm", length_mm);
        Ok(())
    }
}

/// Pick & Place Adapter
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PickPlaceAdapter {
    pub device_id: String,
    pub head_count: u8,
    pub placement_speed_mm_sec: f32,
    pub accuracy_microns: u16,
}

impl PickPlaceAdapter {
    pub fn new(device_id: String, head_count: u8) -> Self {
        PickPlaceAdapter {
            device_id,
            head_count,
            placement_speed_mm_sec: 50.0,
            accuracy_microns: 50,
        }
    }

    pub async fn move_head(&self, head: u8, x: f32, y: f32, z: f32) -> Result<()> {
        tracing::debug!("Pick&Place: Moving head {} to ({}, {}, {})", head, x, y, z);
        Ok(())
    }

    pub async fn pick_component(&self, head: u8) -> Result<()> {
        tracing::debug!("Pick&Place: Picking with head {}", head);
        Ok(())
    }

    pub async fn place_component(&self, head: u8) -> Result<()> {
        tracing::debug!("Pick&Place: Placing with head {}", head);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cnc_adapter_creation() {
        let cnc = CNCAdapter::new("cnc1".to_string(), CNCTech::Milling, 3);
        assert_eq!(cnc.axis_count, 3);
    }

    #[test]
    fn test_laser_adapter_creation() {
        let laser = LaserAdapter::new("laser1".to_string(), LaserTech::CO2, 100);
        assert_eq!(laser.power_w, 100);
    }

    #[test]
    fn test_printer_adapter_creation() {
        let printer = PrinterAdapter::new(
            "printer1".to_string(),
            Printer3DTech::FDM,
            (200.0, 200.0, 200.0),
        );
        assert_eq!(printer.bed_size_mm.0, 200.0);
    }

    #[test]
    fn test_pick_place_adapter() {
        let pp = PickPlaceAdapter::new("pp1".to_string(), 2);
        assert_eq!(pp.head_count, 2);
    }
}
