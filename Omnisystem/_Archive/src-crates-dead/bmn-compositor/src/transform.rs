// Transform — position, rotation, scale

use glam::{Vec2, Vec3, Quat, Mat4};
use serde::{Deserialize, Serialize};

/// 2D/3D transformation
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Transform {
    pub position: Vec2,
    pub scale: Vec2,
    pub rotation: f32, // Rotation in degrees (Z-axis)
    pub anchor_x: f32, // 0.0 = left, 0.5 = center, 1.0 = right
    pub anchor_y: f32, // 0.0 = top, 0.5 = center, 1.0 = bottom
}

impl Transform {
    pub fn identity() -> Self {
        Self::default()
    }

    pub fn with_position(mut self, x: f32, y: f32) -> Self {
        self.position = Vec2::new(x, y);
        self
    }

    pub fn with_scale(mut self, x: f32, y: f32) -> Self {
        self.scale = Vec2::new(x, y);
        self
    }

    pub fn with_rotation(mut self, degrees: f32) -> Self {
        self.rotation = degrees;
        self
    }

    pub fn with_anchor(mut self, x: f32, y: f32) -> Self {
        self.anchor_x = x.max(0.0).min(1.0);
        self.anchor_y = y.max(0.0).min(1.0);
        self
    }

    /// Compute 4x4 transformation matrix
    pub fn matrix(&self, width: f32, height: f32) -> Mat4 {
        let anchor_offset_x = -(width * self.anchor_x);
        let anchor_offset_y = -(height * self.anchor_y);

        let rad = self.rotation.to_radians();
        let cos_r = rad.cos();
        let sin_r = rad.sin();

        // Translation to position
        let tx = self.position.x + width * 0.5;
        let ty = self.position.y + height * 0.5;

        // Build matrix: T * R * S
        Mat4::from_cols(
            [cos_r * self.scale.x, sin_r * self.scale.x, 0.0, 0.0].into(),
            [-sin_r * self.scale.y, cos_r * self.scale.y, 0.0, 0.0].into(),
            [0.0, 0.0, 1.0, 0.0].into(),
            [tx + anchor_offset_x, ty + anchor_offset_y, 0.0, 1.0].into(),
        )
    }

    /// Transform a point
    pub fn transform_point(&self, p: Vec2, width: f32, height: f32) -> Vec2 {
        let mat = self.matrix(width, height);
        let v = mat * Vec3::new(p.x, p.y, 0.0).extend(1.0);
        Vec2::new(v.x, v.y)
    }
}

impl Default for Transform {
    fn default() -> Self {
        Self {
            position: Vec2::ZERO,
            scale: Vec2::ONE,
            rotation: 0.0,
            anchor_x: 0.0,
            anchor_y: 0.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_identity_transform() {
        let t = Transform::identity();
        assert_eq!(t.position, Vec2::ZERO);
        assert_eq!(t.scale, Vec2::ONE);
        assert_eq!(t.rotation, 0.0);
    }

    #[test]
    fn test_transform_with_position() {
        let t = Transform::identity().with_position(100.0, 50.0);
        assert_eq!(t.position, Vec2::new(100.0, 50.0));
    }

    #[test]
    fn test_transform_with_scale() {
        let t = Transform::identity().with_scale(2.0, 0.5);
        assert_eq!(t.scale, Vec2::new(2.0, 0.5));
    }

    #[test]
    fn test_transform_with_rotation() {
        let t = Transform::identity().with_rotation(45.0);
        assert_eq!(t.rotation, 45.0);
    }

    #[test]
    fn test_transform_anchor() {
        let t = Transform::identity()
            .with_anchor(0.5, 0.5)
            .with_position(100.0, 100.0);

        assert_eq!(t.anchor_x, 0.5);
        assert_eq!(t.anchor_y, 0.5);
    }

    #[test]
    fn test_transform_matrix() {
        let t = Transform::identity().with_position(10.0, 20.0);
        let mat = t.matrix(100.0, 100.0);

        // Matrix should represent the transformation
        assert!(mat.col(3).x != 0.0 || mat.col(3).y != 0.0);
    }
}
