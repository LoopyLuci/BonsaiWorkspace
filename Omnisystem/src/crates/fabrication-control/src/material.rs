use crate::{MaterialType, MaterialSpec, Result, FabricationError};
use dashmap::DashMap;
use std::sync::Arc;

pub struct MaterialDatabase {
    materials: Arc<DashMap<MaterialType, MaterialSpec>>,
}

impl MaterialDatabase {
    pub fn new() -> Self {
        let db = Self {
            materials: Arc::new(DashMap::new()),
        };
        db.init_defaults();
        db
    }

    fn init_defaults(&self) {
        let specs = vec![
            (MaterialType::PLA, MaterialSpec {
                material_type: MaterialType::PLA,
                temp_min: 190.0,
                temp_max: 220.0,
                print_speed: 60.0,
                bed_temp: 60.0,
            }),
            (MaterialType::ABS, MaterialSpec {
                material_type: MaterialType::ABS,
                temp_min: 220.0,
                temp_max: 250.0,
                print_speed: 50.0,
                bed_temp: 100.0,
            }),
            (MaterialType::PETG, MaterialSpec {
                material_type: MaterialType::PETG,
                temp_min: 220.0,
                temp_max: 250.0,
                print_speed: 50.0,
                bed_temp: 80.0,
            }),
        ];

        for (mtype, spec) in specs {
            self.materials.insert(mtype, spec);
        }
    }

    pub fn get_spec(&self, material: MaterialType) -> Result<MaterialSpec> {
        self.materials
            .get(&material)
            .map(|ref_| ref_.value().clone())
            .ok_or_else(|| FabricationError::UnsupportedMaterial(format!("{:?}", material)))
    }

    pub fn add_material(&self, spec: MaterialSpec) -> Result<()> {
        self.materials.insert(spec.material_type, spec);
        Ok(())
    }

    pub fn material_count(&self) -> usize {
        self.materials.len()
    }
}

impl Default for MaterialDatabase {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_material_database() {
        let db = MaterialDatabase::new();
        assert!(db.material_count() > 0);
    }

    #[test]
    fn test_get_material_spec() {
        let db = MaterialDatabase::new();
        let spec = db.get_spec(MaterialType::PLA).unwrap();
        assert_eq!(spec.material_type, MaterialType::PLA);
    }

    #[test]
    fn test_add_custom_material() {
        let db = MaterialDatabase::new();
        let initial_count = db.material_count();
        let spec = MaterialSpec {
            material_type: MaterialType::Custom,
            temp_min: 200.0,
            temp_max: 230.0,
            print_speed: 55.0,
            bed_temp: 70.0,
        };
        assert!(db.add_material(spec).is_ok());
        assert!(db.material_count() > initial_count);
    }
}
