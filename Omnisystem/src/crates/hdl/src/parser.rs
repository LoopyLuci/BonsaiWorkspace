use crate::DeviceDefinition;
use anyhow::Result;

pub fn parse(source: &str) -> Result<DeviceDefinition> {
    let def: DeviceDefinition = serde_yaml::from_str(source)?;
    Ok(def)
}
