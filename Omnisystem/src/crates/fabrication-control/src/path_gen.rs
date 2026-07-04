use crate::Result;

pub struct PathGenerator {
    resolution: f32,
}

impl PathGenerator {
    pub fn new(resolution: f32) -> Self {
        Self { resolution }
    }

    pub fn generate_line(&self, start: (f32, f32, f32), end: (f32, f32, f32)) -> Vec<(f32, f32, f32)> {
        let mut path = Vec::new();
        let steps = ((distance(start, end) / self.resolution) as usize).max(2);
        
        for i in 0..=steps {
            let t = i as f32 / steps as f32;
            path.push((
                start.0 + (end.0 - start.0) * t,
                start.1 + (end.1 - start.1) * t,
                start.2 + (end.2 - start.2) * t,
            ));
        }
        
        path
    }

    pub fn generate_circle(&self, center: (f32, f32), radius: f32) -> Vec<(f32, f32)> {
        let mut path = Vec::new();
        let steps = (2.0 * std::f32::consts::PI * radius / self.resolution) as usize;
        
        for i in 0..steps {
            let angle = 2.0 * std::f32::consts::PI * i as f32 / steps as f32;
            path.push((
                center.0 + radius * angle.cos(),
                center.1 + radius * angle.sin(),
            ));
        }
        
        path
    }
}

fn distance(a: (f32, f32, f32), b: (f32, f32, f32)) -> f32 {
    let dx = b.0 - a.0;
    let dy = b.1 - a.1;
    let dz = b.2 - a.2;
    (dx * dx + dy * dy + dz * dz).sqrt()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_path_generator() {
        let gen = PathGenerator::new(0.1);
        let path = gen.generate_line((0.0, 0.0, 0.0), (10.0, 0.0, 0.0));
        assert!(path.len() > 1);
    }

    #[test]
    fn test_circle_generation() {
        let gen = PathGenerator::new(0.1);
        let circle = gen.generate_circle((0.0, 0.0), 1.0);
        assert!(circle.len() > 6);
    }
}
