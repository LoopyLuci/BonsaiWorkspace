//! Scalar and rank-polymorphic primitive operations.

use crate::array::NdArray;
use crate::error::ArrayError;

// ── Scalar map ────────────────────────────────────────────────────────────────

/// Apply a scalar function element-wise to a single array.
pub fn map1(a: &NdArray, f: impl Fn(f64) -> f64) -> NdArray {
    NdArray { shape: a.shape.clone(), data: a.data.iter().map(|&x| f(x)).collect() }
}

/// Apply a scalar dyadic function element-wise to two conformable arrays.
/// Broadcasting: scalars extend to any shape.
pub fn map2(a: &NdArray, b: &NdArray, f: impl Fn(f64, f64) -> f64) -> Result<NdArray, ArrayError> {
    if a.is_scalar() {
        let av = a.data[0];
        return Ok(NdArray { shape: b.shape.clone(), data: b.data.iter().map(|&x| f(av, x)).collect() });
    }
    if b.is_scalar() {
        let bv = b.data[0];
        return Ok(NdArray { shape: a.shape.clone(), data: a.data.iter().map(|&x| f(x, bv)).collect() });
    }
    if a.shape != b.shape {
        return Err(ArrayError::LengthError { left: a.shape.clone(), right: b.shape.clone() });
    }
    Ok(NdArray {
        shape: a.shape.clone(),
        data: a.data.iter().zip(b.data.iter()).map(|(&x, &y)| f(x, y)).collect(),
    })
}

// ── Arithmetic primitives ─────────────────────────────────────────────────────

pub fn add(a: &NdArray, b: &NdArray) -> Result<NdArray, ArrayError>  { map2(a, b, |x, y| x + y) }
pub fn sub(a: &NdArray, b: &NdArray) -> Result<NdArray, ArrayError>  { map2(a, b, |x, y| x - y) }
pub fn mul(a: &NdArray, b: &NdArray) -> Result<NdArray, ArrayError>  { map2(a, b, |x, y| x * y) }
pub fn div(a: &NdArray, b: &NdArray) -> Result<NdArray, ArrayError>  { map2(a, b, |x, y| x / y) }
pub fn rem(a: &NdArray, b: &NdArray) -> Result<NdArray, ArrayError>  { map2(a, b, |x, y| x % y) }
pub fn pow(a: &NdArray, b: &NdArray) -> Result<NdArray, ArrayError>  { map2(a, b, |x, y| x.powf(y)) }
pub fn neg(a: &NdArray) -> NdArray                                   { map1(a, |x| -x) }
pub fn abs(a: &NdArray) -> NdArray                                   { map1(a, f64::abs) }
pub fn ceil(a: &NdArray) -> NdArray                                  { map1(a, f64::ceil) }
pub fn floor(a: &NdArray) -> NdArray                                 { map1(a, f64::floor) }
pub fn signum(a: &NdArray) -> NdArray                                { map1(a, f64::signum) }
pub fn exp(a: &NdArray) -> NdArray                                   { map1(a, f64::exp) }
pub fn ln(a: &NdArray) -> NdArray                                    { map1(a, f64::ln) }
pub fn sqrt(a: &NdArray) -> NdArray                                  { map1(a, f64::sqrt) }
pub fn sin(a: &NdArray) -> NdArray                                   { map1(a, f64::sin) }
pub fn cos(a: &NdArray) -> NdArray                                   { map1(a, f64::cos) }

// ── Comparison (returns 0.0/1.0) ──────────────────────────────────────────────

pub fn eq(a: &NdArray, b: &NdArray) -> Result<NdArray, ArrayError>  { map2(a, b, |x, y| if x == y { 1.0 } else { 0.0 }) }
pub fn ne(a: &NdArray, b: &NdArray) -> Result<NdArray, ArrayError>  { map2(a, b, |x, y| if x != y { 1.0 } else { 0.0 }) }
pub fn lt(a: &NdArray, b: &NdArray) -> Result<NdArray, ArrayError>  { map2(a, b, |x, y| if x <  y { 1.0 } else { 0.0 }) }
pub fn le(a: &NdArray, b: &NdArray) -> Result<NdArray, ArrayError>  { map2(a, b, |x, y| if x <= y { 1.0 } else { 0.0 }) }
pub fn gt(a: &NdArray, b: &NdArray) -> Result<NdArray, ArrayError>  { map2(a, b, |x, y| if x >  y { 1.0 } else { 0.0 }) }
pub fn ge(a: &NdArray, b: &NdArray) -> Result<NdArray, ArrayError>  { map2(a, b, |x, y| if x >= y { 1.0 } else { 0.0 }) }

// ── Boolean ───────────────────────────────────────────────────────────────────

pub fn and(a: &NdArray, b: &NdArray) -> Result<NdArray, ArrayError> {
    map2(a, b, |x, y| if x != 0.0 && y != 0.0 { 1.0 } else { 0.0 })
}
pub fn or(a: &NdArray, b: &NdArray) -> Result<NdArray, ArrayError> {
    map2(a, b, |x, y| if x != 0.0 || y != 0.0 { 1.0 } else { 0.0 })
}
pub fn not(a: &NdArray) -> NdArray {
    map1(a, |x| if x == 0.0 { 1.0 } else { 0.0 })
}

// ── Reductions ────────────────────────────────────────────────────────────────

/// Reduce along the last axis using a binary function.
pub fn reduce(a: &NdArray, f: impl Fn(f64, f64) -> f64, identity: f64) -> NdArray {
    if a.is_scalar() { return a.clone(); }
    let cols = *a.shape.last().unwrap();
    if cols == 0 { return NdArray::scalar(identity); }
    let n_rows: usize = a.data.len() / cols;
    let result: Vec<f64> = (0..n_rows).map(|r| {
        let row = &a.data[r * cols..(r + 1) * cols];
        row.iter().skip(1).fold(row[0], |acc, &x| f(acc, x))
    }).collect();
    if a.shape.len() == 1 {
        NdArray::scalar(result[0])
    } else {
        let mut new_shape = a.shape.clone();
        new_shape.pop();
        NdArray { shape: new_shape, data: result }
    }
}

pub fn sum(a: &NdArray) -> NdArray       { reduce(a, |x, y| x + y, 0.0) }
pub fn product(a: &NdArray) -> NdArray   { reduce(a, |x, y| x * y, 1.0) }
pub fn max_reduce(a: &NdArray) -> NdArray { reduce(a, f64::max, f64::NEG_INFINITY) }
pub fn min_reduce(a: &NdArray) -> NdArray { reduce(a, f64::min, f64::INFINITY) }

// ── Scans ─────────────────────────────────────────────────────────────────────

/// Prefix scan along the last axis.
pub fn scan(a: &NdArray, f: impl Fn(f64, f64) -> f64) -> NdArray {
    if a.is_scalar() { return a.clone(); }
    let cols = *a.shape.last().unwrap();
    let n_rows = a.data.len() / cols.max(1);
    let mut result = Vec::with_capacity(a.data.len());
    for r in 0..n_rows {
        let row = &a.data[r * cols..(r + 1) * cols];
        let mut acc = row[0];
        result.push(acc);
        for &x in &row[1..] {
            acc = f(acc, x);
            result.push(acc);
        }
    }
    NdArray { shape: a.shape.clone(), data: result }
}

pub fn scan_sum(a: &NdArray) -> NdArray     { scan(a, |x, y| x + y) }
pub fn scan_product(a: &NdArray) -> NdArray { scan(a, |x, y| x * y) }

// ── Inner product ─────────────────────────────────────────────────────────────

/// Matrix inner product: (+.×) for matrices (last axis of A, first axis of B must match).
pub fn inner_product(
    a: &NdArray,
    b: &NdArray,
    f_agg: impl Fn(f64, f64) -> f64 + Copy,
    f_combine: impl Fn(f64, f64) -> f64 + Copy,
    identity: f64,
) -> Result<NdArray, ArrayError> {
    // Only 2-D for now
    if a.rank() != 2 || b.rank() != 2 {
        return Err(ArrayError::DomainError("inner product requires rank-2 arrays".into()));
    }
    let (m, k1) = (a.shape[0], a.shape[1]);
    let (k2, n) = (b.shape[0], b.shape[1]);
    if k1 != k2 {
        return Err(ArrayError::LengthError { left: a.shape.clone(), right: b.shape.clone() });
    }
    let mut result = vec![identity; m * n];
    for i in 0..m {
        for j in 0..n {
            let row = &a.data[i * k1..(i + 1) * k1];
            let col: Vec<f64> = (0..k2).map(|r| b.data[r * n + j]).collect();
            let dot = row.iter().zip(col.iter()).skip(1)
                .fold(f_combine(row[0], col[0]), |acc, (&x, &y)| f_agg(acc, f_combine(x, y)));
            result[i * n + j] = dot;
        }
    }
    Ok(NdArray { shape: vec![m, n], data: result })
}

/// Standard matrix multiply (+.×).
pub fn matmul(a: &NdArray, b: &NdArray) -> Result<NdArray, ArrayError> {
    inner_product(a, b, |x, y| x + y, |x, y| x * y, 0.0)
}

// ── Outer product ─────────────────────────────────────────────────────────────

/// Outer product: apply f to every pair (a[i], b[j]).
pub fn outer_product(a: &NdArray, b: &NdArray, f: impl Fn(f64, f64) -> f64) -> NdArray {
    let mut shape = a.shape.clone();
    shape.extend_from_slice(&b.shape);
    let mut data = Vec::with_capacity(a.data.len() * b.data.len());
    for &x in &a.data {
        for &y in &b.data {
            data.push(f(x, y));
        }
    }
    NdArray { shape, data }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vector_add() {
        let a = NdArray::vector(vec![1.0, 2.0, 3.0]);
        let b = NdArray::vector(vec![10.0, 20.0, 30.0]);
        let c = add(&a, &b).unwrap();
        assert_eq!(c.data, vec![11.0, 22.0, 33.0]);
    }

    #[test]
    fn scalar_broadcast() {
        let a = NdArray::scalar(2.0);
        let b = NdArray::vector(vec![1.0, 2.0, 3.0]);
        let c = mul(&a, &b).unwrap();
        assert_eq!(c.data, vec![2.0, 4.0, 6.0]);
    }

    #[test]
    fn sum_reduce() {
        let v = NdArray::vector(vec![1.0, 2.0, 3.0, 4.0]);
        let s = sum(&v);
        assert_eq!(s.scalar_val(), Some(10.0));
    }

    #[test]
    fn matmul_2x2() {
        // [1 2; 3 4] × [5 6; 7 8] = [19 22; 43 50]
        let a = NdArray::from_shape_data(vec![2,2], vec![1.0,2.0,3.0,4.0]).unwrap();
        let b = NdArray::from_shape_data(vec![2,2], vec![5.0,6.0,7.0,8.0]).unwrap();
        let c = matmul(&a, &b).unwrap();
        assert_eq!(c.data, vec![19.0, 22.0, 43.0, 50.0]);
    }

    #[test]
    fn outer_product_mul() {
        let a = NdArray::vector(vec![1.0, 2.0]);
        let b = NdArray::vector(vec![1.0, 2.0, 3.0]);
        let c = outer_product(&a, &b, |x, y| x * y);
        assert_eq!(c.shape, vec![2, 3]);
        assert_eq!(c.data, vec![1.0, 2.0, 3.0, 2.0, 4.0, 6.0]);
    }
}
