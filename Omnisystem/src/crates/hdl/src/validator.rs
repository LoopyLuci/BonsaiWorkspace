use crate::DeviceDefinition;
use anyhow::Result;

pub fn validate(def: &DeviceDefinition) -> Result<()> {
    if def.instruction_set.is_empty() {
        anyhow::bail!("Device '{}' has no instructions defined", def.name);
    }
    if def.registers.is_empty() {
        anyhow::bail!("Device '{}' has no registers defined", def.name);
    }
    for inst in &def.instruction_set {
        if inst.semantics.is_empty() {
            anyhow::bail!("Instruction '{}' has no semantics", inst.mnemonic);
        }
    }
    Ok(())
}
