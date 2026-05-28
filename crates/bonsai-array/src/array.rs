//! N-dimensional array type.

use serde::{Deserialize, Serialize};
use crate::error::ArrayError;

/// Rank-polymorphic N-dimensional array of `f64`.
///
/// Shape `[]` = scalar, `[n]` = vector, `[m,n]` = matrix, etc.
/// Data is stored in row-major (C) order.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NdArray {
    pub shape: Vec<usize>,
    pub data:  Vec<f64>,
}

impl NdArray {
    // ── Constructors ─────────────────────────────────────────────────────────

    /// Scalar value.
    pub fn scalar(v: f64) -> Self {
        Self { shape: vec![], data: vec![v] }
    }

    /// 1-D vector.
    pub fn vector(data: Vec<f64>) -> Self {
        let n = data.len();
        Self { shape: vec![n], data }
    }

    /// Create from shape + flat data (row-major).
    pub fn from_shape_data(shape: Vec<usize>, data: Vec<f64>) -> Result<Self, ArrayError> {
        let expected: usize = shape.iter().product();
        let expected = expected.max(1); // empty shape → 1 element (scalar)
        if data.len() != expected {
            return Err(ArrayError::LengthError { left: shape, right: vec![data.len()] });
        }
        Ok(Self { shape, data })
    }

    /// Reshape: data is recycled or truncated to fill the new shape.
    pub fn reshape(&self, new_shape: Vec<usize>) -> Self {
        let n: usize = new_shape.iter().product();
        let n = n.max(1);
        let mut data = Vec::with_capacity(n);
        for i in 0..n {
            data.push(self.data[i % self.data.len()]);
        }
        Self { shape: new_shape, data }
    }

    // ── Inspection ───────────────────────────────────────────────────────────

    pub fn rank(&self) -> usize { self.shape.len() }
    pub fn len(&self) -> usize { self.data.len() }
    pub fn is_empty(&self) -> bool { self.data.is_empty() }

    pub fn is_scalar(&self) -> bool { self.shape.is_empty() }

    pub fn scalar_val(&self) -> Option<f64> {
        if self.is_scalar() { self.data.first().copied() } else { None }
    }

    // ── Indexing ─────────────────────────────────────────────────────────────

    /// Flat index into data (row-major).
    pub fn get(&self, flat_idx: usize) -> Option<f64> {
        self.data.get(flat_idx).copied()
    }

    // ── Structural ───────────────────────────────────────────────────────────

    /// Ravel: return 1-D copy of all elements.
    pub fn ravel(&self) -> Self {
        Self::vector(self.data.clone())
    }

    /// Reverse along the last axis.
    pub fn reverse(&self) -> Self {
        if self.shape.is_empty() { return self.clone(); }
        let cols = *self.shape.last().unwrap();
        let mut out = self.data.clone();
        for chunk in out.chunks_mut(cols) {
            chunk.reverse();
        }
        Self { shape: self.shape.clone(), data: out }
    }

    /// Transpose (reverses axis order).
    pub fn transpose(&self) -> Self {
        if self.rank() < 2 { return self.clone(); }
        if self.rank() == 2 {
            let rows = self.shape[0];
            let cols = self.shape[1];
            let mut out = vec![0.0f64; self.data.len()];
            for r in 0..rows {
                for c in 0..cols {
                    out[c * rows + r] = self.data[r * cols + c];
                }
            }
            return Self { shape: vec![cols, rows], data: out };
        }
        // General: reverse shape, rearrange data
        let mut new_shape = self.shape.clone();
        new_shape.reverse();
        // For higher ranks: full permutation (slow but correct)
        let n = self.data.len();
        let rank = self.rank();
        let mut out = vec![0.0f64; n];
        let old_strides = compute_strides(&self.shape);
        let new_strides = compute_strides(&new_shape);
        for flat in 0..n {
            let old_idx = flat_to_indices(flat, &self.shape);
            let mut rev_idx = old_idx.clone();
            rev_idx.reverse();
            let new_flat: usize = rev_idx.iter().zip(&new_strides).map(|(i, s)| i * s).sum();
            let _ = rank; // suppress warning
            out[new_flat] = self.data[flat];
        }
        Self { shape: new_shape, data: out }
    }

    /// Take first `n` elements along the first axis. Negative = from end.
    pub fn take(&self, n: i64) -> Result<Self, ArrayError> {
        if self.shape.is_empty() { return Ok(self.clone()); }
        let axis_len = self.shape[0] as i64;
        let n = n.clamp(-axis_len, axis_len);
        let (start, count) = if n >= 0 {
            (0usize, n as usize)
        } else {
            ((axis_len + n) as usize, (-n) as usize)
        };
        let sub_len: usize = self.shape[1..].iter().product::<usize>().max(1);
        let data: Vec<f64> = self.data[start * sub_len..(start + count) * sub_len].to_vec();
        let mut new_shape = self.shape.clone();
        new_shape[0] = count;
        Ok(Self { shape: new_shape, data })
    }

    /// Drop first `n` elements along the first axis. Negative = from end.
    pub fn drop(&self, n: i64) -> Result<Self, ArrayError> {
        if self.shape.is_empty() { return Ok(self.clone()); }
        let axis_len = self.shape[0] as i64;
        let remaining = (axis_len - n.abs()).max(0) as i64;
        let from_end = n < 0;
        let start_n = if from_end { remaining } else { n.abs() };
        self.take(if from_end { -remaining } else { remaining })
            .map(|_| ()) // discard to avoid unused variable warning
            .ok();
        // Drop n from front or back
        let keep = remaining;
        if from_end { self.take(keep) } else { self.take(-keep) }
    }

    // ── Grade (argsort) ───────────────────────────────────────────────────────

    pub fn grade_up(&self) -> Self {
        let mut indices: Vec<usize> = (0..self.data.len()).collect();
        indices.sort_by(|&a, &b| self.data[a].partial_cmp(&self.data[b]).unwrap_or(std::cmp::Ordering::Equal));
        Self::vector(indices.iter().map(|&i| i as f64).collect())
    }

    pub fn grade_down(&self) -> Self {
        let mut indices: Vec<usize> = (0..self.data.len()).collect();
        indices.sort_by(|&a, &b| self.data[b].partial_cmp(&self.data[a]).unwrap_or(std::cmp::Ordering::Equal));
        Self::vector(indices.iter().map(|&i| i as f64).collect())
    }

    // ── Catenation ───────────────────────────────────────────────────────────

    /// Catenate two arrays along the first axis.
    pub fn catenate(&self, other: &NdArray) -> Result<Self, ArrayError> {
        // Both must be compatible in all axes except the first
        if self.rank() != other.rank() {
            return Err(ArrayError::RankError { expected: self.rank(), got: other.rank() });
        }
        if self.rank() > 1 {
            let ok = self.shape[1..] == other.shape[1..];
            if !ok {
                return Err(ArrayError::LengthError {
                    left: self.shape.clone(),
                    right: other.shape.clone(),
                });
            }
        }
        let mut data = self.data.clone();
        data.extend_from_slice(&other.data);
        let mut new_shape = self.shape.clone();
        if new_shape.is_empty() {
            // scalar catenation → length-2 vector
            return Ok(Self::vector(data));
        }
        new_shape[0] += other.shape[0];
        Ok(Self { shape: new_shape, data })
    }
}

// ── Strides & indices ─────────────────────────────────────────────────────────

pub fn compute_strides(shape: &[usize]) -> Vec<usize> {
    let mut strides = vec![1usize; shape.len()];
    for i in (0..shape.len().saturating_sub(1)).rev() {
        strides[i] = strides[i + 1] * shape[i + 1];
    }
    strides
}

pub fn flat_to_indices(flat: usize, shape: &[usize]) -> Vec<usize> {
    let strides = compute_strides(shape);
    let mut idx = Vec::with_capacity(shape.len());
    let mut rem = flat;
    for s in &strides {
        idx.push(rem / s);
        rem %= s;
    }
    idx
}

// ── Display ───────────────────────────────────────────────────────────────────

impl std::fmt::Display for NdArray {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.is_scalar() {
            return write!(f, "{}", self.data[0]);
        }
        if self.rank() == 1 {
            let parts: Vec<String> = self.data.iter().map(|v| format!("{v}")).collect();
            return write!(f, "{}", parts.join(" "));
        }
        // Matrix: newline-separated rows
        let cols = self.shape[self.rank() - 1];
        for (i, chunk) in self.data.chunks(cols).enumerate() {
            if i > 0 { writeln!(f)?; }
            let parts: Vec<String> = chunk.iter().map(|v| format!("{v:>10}")).collect();
            write!(f, "{}", parts.join(""))?;
        }
        Ok(())
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reshape_recycles() {
        let v = NdArray::vector(vec![1.0, 2.0, 3.0]);
        let m = v.reshape(vec![2, 3]);
        assert_eq!(m.shape, vec![2, 3]);
        assert_eq!(m.data, vec![1.0, 2.0, 3.0, 1.0, 2.0, 3.0]);
    }

    #[test]
    fn transpose_2d() {
        let m = NdArray::from_shape_data(vec![2, 3], vec![1.0,2.0,3.0,4.0,5.0,6.0]).unwrap();
        let t = m.transpose();
        assert_eq!(t.shape, vec![3, 2]);
        assert_eq!(t.data[0], 1.0);
        assert_eq!(t.data[1], 4.0);
    }

    #[test]
    fn grade_up_sorts() {
        let v = NdArray::vector(vec![3.0, 1.0, 2.0]);
        let g = v.grade_up();
        assert_eq!(g.data, vec![1.0, 2.0, 0.0]);
    }

    #[test]
    fn reverse_vector() {
        let v = NdArray::vector(vec![1.0, 2.0, 3.0]);
        assert_eq!(v.reverse().data, vec![3.0, 2.0, 1.0]);
    }
}
