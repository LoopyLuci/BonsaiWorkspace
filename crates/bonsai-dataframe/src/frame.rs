//! `BonsaiFrame` — eager DataFrame operations wrapping `polars::DataFrame`.

use polars::prelude::*;
use serde_json::Value as JsonValue;
use crate::error::{DfError, DfResult};
use crate::ops::{AggExpr, FilterExpr, Scalar, SortSpec};

/// Eager DataFrame wrapper.
#[derive(Debug, Clone)]
pub struct BonsaiFrame(pub DataFrame);

impl BonsaiFrame {
    // ── Construction ─────────────────────────────────────────────────────────

    pub fn empty() -> Self {
        Self(DataFrame::default())
    }

    pub fn from_polars(df: DataFrame) -> Self {
        Self(df)
    }

    pub fn into_polars(self) -> DataFrame {
        self.0
    }

    pub fn polars(&self) -> &DataFrame {
        &self.0
    }

    /// Build from a slice of (column_name, JSON array) pairs.
    pub fn from_json_columns(cols: &[(String, Vec<JsonValue>)]) -> DfResult<Self> {
        let columns: DfResult<Vec<Column>> = cols.iter().map(|(name, vals)| {
            json_array_to_column(name, vals)
        }).collect();
        let df = DataFrame::new(columns?)?;
        Ok(Self(df))
    }

    // ── Inspection ───────────────────────────────────────────────────────────

    pub fn height(&self) -> usize { self.0.height() }
    pub fn width(&self)  -> usize { self.0.width() }

    pub fn column_names(&self) -> Vec<String> {
        self.0.get_column_names().into_iter().map(|s| s.to_string()).collect()
    }

    pub fn head(&self, n: usize) -> DfResult<Self> {
        Ok(Self(self.0.head(Some(n))))
    }

    pub fn tail(&self, n: usize) -> DfResult<Self> {
        Ok(Self(self.0.tail(Some(n))))
    }

    // ── Selection ────────────────────────────────────────────────────────────

    pub fn select(&self, cols: &[&str]) -> DfResult<Self> {
        let exprs: Vec<Expr> = cols.iter().map(|c| col(*c)).collect();
        let result = self.0.clone().lazy().select(exprs).collect()?;
        Ok(Self(result))
    }

    pub fn drop_columns(&self, cols: &[&str]) -> DfResult<Self> {
        let mut df = self.0.clone();
        for c in cols {
            df = df.drop(c)?;
        }
        Ok(Self(df))
    }

    pub fn rename_column(&mut self, old: &str, new: &str) -> DfResult<()> {
        self.0.rename(old, new.into())?;
        Ok(())
    }

    // ── Filtering ────────────────────────────────────────────────────────────

    pub fn filter(&self, expr: &FilterExpr) -> DfResult<Self> {
        let result = self.0.clone().lazy().filter(filter_to_expr(expr)).collect()?;
        Ok(Self(result))
    }

    // ── Sorting ──────────────────────────────────────────────────────────────

    pub fn sort(&self, specs: &[SortSpec]) -> DfResult<Self> {
        if specs.is_empty() { return Ok(self.clone()); }
        let col_names: Vec<PlSmallStr> = specs.iter().map(|s| PlSmallStr::from(s.col.as_str())).collect();
        let opts = SortMultipleOptions::default()
            .with_order_descending_multi(specs.iter().map(|s| s.descending).collect::<Vec<_>>())
            .with_nulls_last_multi(specs.iter().map(|s| s.nulls_last).collect::<Vec<_>>());
        Ok(Self(self.0.sort(col_names, opts)?))
    }

    // ── Grouping & Aggregation ────────────────────────────────────────────────

    pub fn group_by_agg(&self, by: &[&str], aggs: &[AggExpr]) -> DfResult<Self> {
        let group_cols: Vec<Expr> = by.iter().map(|c| col(*c)).collect();
        let agg_exprs: Vec<Expr> = aggs.iter().map(agg_to_expr).collect();
        let result = self.0.clone().lazy().group_by(group_cols).agg(agg_exprs).collect()?;
        Ok(Self(result))
    }

    // ── Join ─────────────────────────────────────────────────────────────────

    pub fn inner_join(&self, other: &BonsaiFrame, left_on: &[&str], right_on: &[&str]) -> DfResult<Self> {
        let left_exprs: Vec<Expr>  = left_on.iter().map(|c| col(*c)).collect();
        let right_exprs: Vec<Expr> = right_on.iter().map(|c| col(*c)).collect();
        let result = self.0.clone().lazy()
            .join(other.0.clone().lazy(), left_exprs, right_exprs, JoinArgs::new(JoinType::Inner))
            .collect()?;
        Ok(Self(result))
    }

    pub fn left_join(&self, other: &BonsaiFrame, left_on: &[&str], right_on: &[&str]) -> DfResult<Self> {
        let left_exprs: Vec<Expr>  = left_on.iter().map(|c| col(*c)).collect();
        let right_exprs: Vec<Expr> = right_on.iter().map(|c| col(*c)).collect();
        let result = self.0.clone().lazy()
            .join(other.0.clone().lazy(), left_exprs, right_exprs, JoinArgs::new(JoinType::Left))
            .collect()?;
        Ok(Self(result))
    }

    // ── Column computation ────────────────────────────────────────────────────

    pub fn with_column_literal(&self, name: &str, val: &Scalar) -> DfResult<Self> {
        let lit_expr = scalar_to_lit(val).alias(name);
        let result = self.0.clone().lazy().with_column(lit_expr).collect()?;
        Ok(Self(result))
    }

    // ── Serialisation ────────────────────────────────────────────────────────

    pub fn to_json_rows(&self) -> DfResult<Vec<serde_json::Map<String, JsonValue>>> {
        let mut out = Vec::with_capacity(self.height());
        for i in 0..self.height() {
            let mut row = serde_json::Map::new();
            for col in self.0.get_columns() {
                let val = column_get_json(col, i);
                row.insert(col.name().to_string(), val);
            }
            out.push(row);
        }
        Ok(out)
    }

    pub fn to_json_columns(&self) -> DfResult<serde_json::Map<String, JsonValue>> {
        let mut out = serde_json::Map::new();
        for col_data in self.0.get_columns() {
            let arr: Vec<JsonValue> = (0..col_data.len()).map(|i| column_get_json(col_data, i)).collect();
            out.insert(col_data.name().to_string(), JsonValue::Array(arr));
        }
        Ok(out)
    }

    pub fn schema_json(&self) -> JsonValue {
        let fields: Vec<JsonValue> = self.0.schema().iter_fields().map(|f| {
            serde_json::json!({ "name": f.name().as_str(), "dtype": format!("{}", f.dtype()) })
        }).collect();
        JsonValue::Array(fields)
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn scalar_to_lit(s: &Scalar) -> Expr {
    match s {
        Scalar::Null      => lit(NULL),
        Scalar::Bool(b)   => lit(*b),
        Scalar::Int(n)    => lit(*n),
        Scalar::Float(f)  => lit(*f),
        Scalar::Str(s)    => lit(s.as_str()),
    }
}

pub(crate) fn filter_to_expr(f: &FilterExpr) -> Expr {
    match f {
        FilterExpr::Eq  { col: c, val } => col(c).eq(scalar_to_lit(val)),
        FilterExpr::Ne  { col: c, val } => col(c).neq(scalar_to_lit(val)),
        FilterExpr::Lt  { col: c, val } => col(c).lt(scalar_to_lit(val)),
        FilterExpr::Le  { col: c, val } => col(c).lt_eq(scalar_to_lit(val)),
        FilterExpr::Gt  { col: c, val } => col(c).gt(scalar_to_lit(val)),
        FilterExpr::Ge  { col: c, val } => col(c).gt_eq(scalar_to_lit(val)),
        FilterExpr::Contains { col: c, pat } => col(c).str().contains(lit(pat.as_str()), false),
        FilterExpr::IsNull   { col: c }  => col(c).is_null(),
        FilterExpr::IsNotNull{ col: c }  => col(c).is_not_null(),
        FilterExpr::And(exprs) => exprs.iter().map(filter_to_expr).reduce(|a, b| a.and(b)).unwrap_or(lit(true)),
        FilterExpr::Or(exprs)  => exprs.iter().map(filter_to_expr).reduce(|a, b| a.or(b)).unwrap_or(lit(false)),
        FilterExpr::Not(inner) => filter_to_expr(inner).not(),
    }
}

pub(crate) fn agg_to_expr(a: &AggExpr) -> Expr {
    let base = match a {
        AggExpr::Sum   { col: c, .. } => col(c).sum(),
        AggExpr::Mean  { col: c, .. } => col(c).mean(),
        AggExpr::Min   { col: c, .. } => col(c).min(),
        AggExpr::Max   { col: c, .. } => col(c).max(),
        AggExpr::Count { col: c, .. } => col(c).count(),
        AggExpr::First { col: c, .. } => col(c).first(),
        AggExpr::Last  { col: c, .. } => col(c).last(),
        AggExpr::Std   { col: c, .. } => col(c).std(1),
        AggExpr::Median{ col: c, .. } => col(c).median(),
    };
    if let Some(alias_name) = a.alias() { base.alias(alias_name) } else { base }
}

fn column_get_json(col: &Column, i: usize) -> JsonValue {
    let s = col.as_materialized_series();
    match s.dtype() {
        DataType::Boolean => s.bool().ok().and_then(|c| c.get(i)).map(JsonValue::Bool)
            .unwrap_or(JsonValue::Null),
        DataType::Int8  | DataType::Int16 | DataType::Int32 | DataType::Int64 |
        DataType::UInt8 | DataType::UInt16| DataType::UInt32| DataType::UInt64 => {
            s.cast(&DataType::Int64).ok()
             .and_then(|s2| s2.i64().ok().and_then(|c| c.get(i)))
             .map(|n| JsonValue::Number(n.into()))
             .unwrap_or(JsonValue::Null)
        }
        DataType::Float32 | DataType::Float64 => {
            s.cast(&DataType::Float64).ok()
             .and_then(|s2| s2.f64().ok().and_then(|c| c.get(i)))
             .and_then(|f| serde_json::Number::from_f64(f))
             .map(JsonValue::Number)
             .unwrap_or(JsonValue::Null)
        }
        DataType::String => s.str().ok().and_then(|c| c.get(i)).map(|s| JsonValue::String(s.into()))
            .unwrap_or(JsonValue::Null),
        _ => s.cast(&DataType::String).ok()
              .and_then(|s2| s2.str().ok().and_then(|c| c.get(i)).map(|v| JsonValue::String(v.into())))
              .unwrap_or(JsonValue::Null),
    }
}

fn json_array_to_column(name: &str, vals: &[JsonValue]) -> DfResult<Column> {
    let first = vals.iter().find(|v| !v.is_null());
    let series = match first {
        Some(JsonValue::Bool(_)) => {
            let v: Vec<Option<bool>> = vals.iter().map(|x| x.as_bool()).collect();
            Series::new(name.into(), v)
        }
        Some(JsonValue::Number(n)) if n.is_i64() => {
            let v: Vec<Option<i64>> = vals.iter().map(|x| x.as_i64()).collect();
            Series::new(name.into(), v)
        }
        Some(JsonValue::Number(_)) => {
            let v: Vec<Option<f64>> = vals.iter().map(|x| x.as_f64()).collect();
            Series::new(name.into(), v)
        }
        _ => {
            let v: Vec<Option<&str>> = vals.iter().map(|x| x.as_str()).collect();
            Series::new(name.into(), v)
        }
    };
    Ok(series.into_column())
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> BonsaiFrame {
        let df = df![
            "name"  => ["Alice", "Bob", "Carol", "Dave"],
            "score" => [88i64, 72, 95, 60],
            "pass"  => [true, true, true, false],
        ].unwrap();
        BonsaiFrame::from_polars(df)
    }

    #[test]
    fn head_tail() {
        let f = sample();
        assert_eq!(f.head(2).unwrap().height(), 2);
        assert_eq!(f.tail(1).unwrap().height(), 1);
    }

    #[test]
    fn select_and_columns() {
        let f = sample().select(&["name", "score"]).unwrap();
        assert_eq!(f.width(), 2);
        assert!(f.column_names().iter().any(|s| s == "name"));
    }

    #[test]
    fn filter_gt() {
        let f = sample().filter(&FilterExpr::Gt {
            col: "score".into(),
            val: crate::ops::Scalar::Int(80),
        }).unwrap();
        assert_eq!(f.height(), 2); // Alice(88) + Carol(95)
    }

    #[test]
    fn json_round_trip() {
        let f = sample();
        let rows = f.to_json_rows().unwrap();
        assert_eq!(rows.len(), 4);
        assert!(rows[0].contains_key("name"));
    }

    #[test]
    fn sort_descending() {
        let f = sample().sort(&[SortSpec { col: "score".into(), descending: true, nulls_last: true }]).unwrap();
        let rows = f.to_json_rows().unwrap();
        assert_eq!(rows[0]["name"], JsonValue::String("Carol".into()));
    }
}
