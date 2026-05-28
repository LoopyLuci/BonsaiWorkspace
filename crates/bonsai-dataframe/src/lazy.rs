//! `BonsaiLazyFrame` — lazy query builder wrapping `polars::LazyFrame`.

use polars::prelude::*;
use crate::error::DfResult;
use crate::frame::BonsaiFrame;
use crate::ops::{AggExpr, FilterExpr, SortSpec};

pub struct BonsaiLazyFrame(LazyFrame);

impl BonsaiLazyFrame {
    pub fn from_frame(f: BonsaiFrame) -> Self {
        Self(f.into_polars().lazy())
    }

    pub fn from_polars(lf: LazyFrame) -> Self {
        Self(lf)
    }

    pub fn select(self, cols: &[&str]) -> Self {
        let exprs: Vec<Expr> = cols.iter().map(|c| col(*c)).collect();
        Self(self.0.select(exprs))
    }

    pub fn filter(self, expr: &FilterExpr) -> Self {
        Self(self.0.filter(filter_to_expr(expr)))
    }

    pub fn sort(self, specs: &[SortSpec]) -> Self {
        if specs.is_empty() { return self; }
        let opts = SortMultipleOptions::default()
            .with_order_descending_multi(specs.iter().map(|s| s.descending).collect::<Vec<_>>())
            .with_nulls_last_multi(specs.iter().map(|s| s.nulls_last).collect::<Vec<_>>());
        let sort_exprs: Vec<Expr> = specs.iter().map(|s| col(s.col.as_str())).collect();
        Self(self.0.sort_by_exprs(sort_exprs, opts))
    }

    pub fn limit(self, n: u32) -> Self {
        Self(self.0.limit(n))
    }

    pub fn with_column(self, name: &str, expr: Expr) -> Self {
        Self(self.0.with_column(expr.alias(name)))
    }

    pub fn group_by_agg(self, by: &[&str], aggs: &[AggExpr]) -> Self {
        let group_cols: Vec<Expr> = by.iter().map(|c| col(*c)).collect();
        let agg_exprs: Vec<Expr> = aggs.iter().map(agg_to_expr).collect();
        Self(self.0.group_by(group_cols).agg(agg_exprs))
    }

    pub fn collect(self) -> DfResult<BonsaiFrame> {
        Ok(BonsaiFrame::from_polars(self.0.collect()?))
    }
}

fn filter_to_expr(f: &FilterExpr) -> Expr {
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

fn agg_to_expr(a: &AggExpr) -> Expr {
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

fn scalar_to_lit(s: &crate::ops::Scalar) -> Expr {
    use crate::ops::Scalar;
    match s {
        Scalar::Null      => lit(NULL),
        Scalar::Bool(b)   => lit(*b),
        Scalar::Int(n)    => lit(*n),
        Scalar::Float(f)  => lit(*f),
        Scalar::Str(s)    => lit(s.as_str()),
    }
}
