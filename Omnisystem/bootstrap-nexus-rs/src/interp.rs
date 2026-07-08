//! Nexus solver.
//!
//! This is not a tree-walking *program* interpreter — there's nothing to
//! execute step-by-step. It's a **declarative solver**: box/layout
//! properties are equations resolved lazily and memoized (spreadsheet-cell
//! style, with cycle detection — the same technique real constraint/build
//! systems use for dependency resolution), `layout` blocks additionally run
//! a genuine flow-layout algorithm (row/column positioning + cross-axis
//! stretch, i.e. real flexbox-lite semantics, not just arbitrary equations),
//! and `constrain` statements are **checked** after solving — a constraint
//! that doesn't hold is a real, reportable failure, not silently ignored.

use std::collections::{HashMap, HashSet};

use crate::ast::*;
use crate::diag::{OmniError, Phase, Span};

const EPSILON: f64 = 1e-6;

pub struct Solver {
    pub file: String,
    boxes: HashMap<String, BoxDef>,
    layouts: HashMap<String, LayoutDef>,
    solved: HashMap<(String, String), f64>,
    resolving: HashSet<(String, String)>,
    pub out: String,
}

pub type RResult<T> = Result<T, Box<OmniError>>;

impl Solver {
    pub fn new(file: &str) -> Self {
        Solver { file: file.to_string(), boxes: HashMap::new(), layouts: HashMap::new(), solved: HashMap::new(), resolving: HashSet::new(), out: String::new() }
    }

    fn rt(&self, msg: impl Into<String>, span: Span) -> Box<OmniError> {
        Box::new(OmniError::new(Phase::Runtime, msg, span, &self.file))
    }

    fn print(&mut self, s: &str) {
        self.out.push_str(s);
        self.out.push('\n');
    }

    /// Runs the whole module: registers all boxes/layouts, runs flow-layout
    /// for each `layout` (which injects `x`/`y`/stretch cross-axis values
    /// into the solved table for its children), lazily resolves every
    /// declared property, checks every `constrain` statement, and reports
    /// the full solved geometry.
    pub fn solve_module(&mut self, module: &Module) -> RResult<i32> {
        for b in &module.boxes {
            self.boxes.insert(b.name.clone(), b.clone());
        }
        for l in &module.layouts {
            self.layouts.insert(l.name.clone(), l.clone());
        }

        // Layouts' own declared props (width/height) first.
        let layout_names: Vec<String> = module.layouts.iter().map(|l| l.name.clone()).collect();
        for name in &layout_names {
            let prop_names: Vec<String> = self.layouts[name].props.iter().map(|(n, _)| n.clone()).collect();
            for p in prop_names {
                self.resolve(name, &p, module_span(module))?;
            }
        }

        // Real flow-layout: position + stretch each layout's children.
        for name in &layout_names {
            self.flow_layout(name)?;
        }

        // Resolve every declared property on every box and layout so the
        // final report is complete (properties not touched by flow-layout
        // or not yet demanded by another expression still get solved).
        let box_names: Vec<String> = module.boxes.iter().map(|b| b.name.clone()).collect();
        for name in &box_names {
            let prop_names: Vec<String> = self.boxes[name].props.iter().map(|(n, _)| n.clone()).collect();
            for p in prop_names {
                self.resolve(&name.clone(), &p, module_span(module))?;
            }
        }

        // Check every constraint — a real pass/fail, not just a declaration.
        let mut all_ok = true;
        for c in &module.constraints {
            let l = self.eval(&c.left)?;
            let r = self.eval(&c.right)?;
            let ok = match c.op.as_str() {
                "==" => (l - r).abs() < EPSILON,
                ">=" => l >= r - EPSILON,
                "<=" => l <= r + EPSILON,
                _ => return Err(self.rt(format!("unknown constraint operator '{}'", c.op), c.span)),
            };
            all_ok &= ok;
            self.print(&format!("constrain {} -> {} ({} {} {})", describe_constraint(c), if ok { "OK" } else { "FAILED" }, fmt_num(l), c.op, fmt_num(r)));
        }

        self.report(module);

        Ok(if all_ok { 0 } else { 1 })
    }

    fn report(&mut self, module: &Module) {
        for b in &module.boxes {
            let declared: Vec<String> = b.props.iter().map(|(n, _)| n.clone()).collect();
            self.print(&format_solved_entity(&b.name, &declared, &self.solved));
        }
        for l in &module.layouts {
            let declared: Vec<String> = l.props.iter().map(|(n, _)| n.clone()).collect();
            self.print(&format_solved_entity(&l.name, &declared, &self.solved));
        }
    }

    /// Row: children laid left-to-right, `x` accumulates by prior widths,
    /// `y = 0`, and `height` stretches to the layout's height unless the
    /// child box already declares its own `height`. Column is the transpose.
    /// This is genuine flexbox-lite behavior, not arbitrary constraint
    /// declarations — the structural point of `layout` blocks existing at
    /// all, distinct from a plain `box`.
    fn flow_layout(&mut self, layout_name: &str) -> RResult<()> {
        let layout = self.layouts.get(layout_name).cloned().unwrap();
        let main_prop = if layout.direction == Direction::Row { "width" } else { "height" };
        let cross_prop = if layout.direction == Direction::Row { "height" } else { "width" };
        let layout_cross = self.resolve(layout_name, cross_prop, layout.span)?;

        let mut cumulative = 0.0;
        for child in &layout.children {
            if !self.boxes.contains_key(child) {
                return Err(self.rt(format!("layout '{layout_name}' references undefined box '{child}'"), layout.span));
            }
            let child_declares_cross = self.boxes[child].props.iter().any(|(n, _)| n == cross_prop);
            if !child_declares_cross {
                self.solved.insert((child.clone(), cross_prop.to_string()), layout_cross);
            }
            let main_size = self.resolve(child, main_prop, layout.span)?;
            let (x, y) = if layout.direction == Direction::Row { (cumulative, 0.0) } else { (0.0, cumulative) };
            self.solved.insert((child.clone(), "x".to_string()), x);
            self.solved.insert((child.clone(), "y".to_string()), y);
            cumulative += main_size;
        }
        Ok(())
    }

    /// Lazily resolves `obj.prop`, memoizing the result and detecting
    /// dependency cycles (a box property that depends, directly or
    /// transitively, on itself is a real, reportable error — not infinite
    /// recursion or a silently wrong default).
    fn resolve(&mut self, obj: &str, prop: &str, span: Span) -> RResult<f64> {
        let key = (obj.to_string(), prop.to_string());
        if let Some(v) = self.solved.get(&key) {
            return Ok(*v);
        }
        if self.resolving.contains(&key) {
            return Err(self.rt(format!("constraint cycle detected: '{obj}.{prop}' depends on itself"), span));
        }
        let expr = self
            .boxes
            .get(obj)
            .and_then(|b| b.props.iter().find(|(n, _)| n == prop).map(|(_, e)| e.clone()))
            .or_else(|| self.layouts.get(obj).and_then(|l| l.props.iter().find(|(n, _)| n == prop).map(|(_, e)| e.clone())));
        let Some(expr) = expr else {
            return Err(self.rt(format!("'{obj}' has no property '{prop}' (not declared, and not auto-computed by a layout)"), span));
        };
        self.resolving.insert(key.clone());
        let v = self.eval(&expr)?;
        self.resolving.remove(&key);
        self.solved.insert(key, v);
        Ok(v)
    }

    fn eval(&mut self, expr: &Expr) -> RResult<f64> {
        match expr {
            Expr::Num { v, .. } => Ok(*v),
            Expr::PropRef { obj, prop, span } => self.resolve(obj, prop, *span),
            Expr::UnaryOp { op, expr, span } => {
                let v = self.eval(expr)?;
                match op.as_str() {
                    "-" => Ok(-v),
                    _ => Err(self.rt(format!("unknown unary operator '{op}'"), *span)),
                }
            }
            Expr::BinOp { op, left, right, span } => {
                let l = self.eval(left)?;
                let r = self.eval(right)?;
                match op.as_str() {
                    "+" => Ok(l + r),
                    "-" => Ok(l - r),
                    "*" => Ok(l * r),
                    "/" => {
                        if r == 0.0 {
                            Err(self.rt("division by zero in layout expression", *span))
                        } else {
                            Ok(l / r)
                        }
                    }
                    _ => Err(self.rt(format!("unknown operator '{op}'"), *span)),
                }
            }
            Expr::Str { span, .. } => {
                Err(self.rt("a CSS-style string/keyword/token value is not a solvable numeric layout expression", *span))
            }
        }
    }
}

fn module_span(module: &Module) -> Span {
    module
        .boxes
        .first()
        .map(|b| b.span)
        .or_else(|| module.layouts.first().map(|l| l.span))
        .or_else(|| module.constraints.first().map(|c| c.span))
        .unwrap_or(Span::point(1, 1))
}

fn fmt_num(v: f64) -> String {
    if v.fract() == 0.0 && v.is_finite() {
        format!("{v:.1}")
    } else {
        v.to_string()
    }
}

fn describe_constraint(c: &ConstraintStmt) -> String {
    format!("{} {} {}", describe_expr(&c.left), c.op, describe_expr(&c.right))
}

fn describe_expr(e: &Expr) -> String {
    match e {
        Expr::Num { v, .. } => fmt_num(*v),
        Expr::PropRef { obj, prop, .. } => format!("{obj}.{prop}"),
        Expr::UnaryOp { op, expr, .. } => format!("{op}{}", describe_expr(expr)),
        Expr::BinOp { op, left, right, .. } => format!("{} {} {}", describe_expr(left), op, describe_expr(right)),
        Expr::Str { v, .. } => format!("{v:?}"),
    }
}

/// Formats one solved box/layout's report line in a deterministic order:
/// the canonical geometry fields first (only the ones actually solved —
/// e.g. a non-flow-layout box may have no x/y at all), then any other
/// declared properties in their *declaration order* (not HashMap iteration
/// order, which is unstable across runs and would make output
/// non-reproducible for tests).
fn format_solved_entity(name: &str, declared_order: &[String], solved: &HashMap<(String, String), f64>) -> String {
    let mut parts = Vec::new();
    let mut seen = HashSet::new();
    for canon in ["x", "y", "width", "height"] {
        if let Some(v) = solved.get(&(name.to_string(), canon.to_string())) {
            parts.push(format!("{canon}={}", fmt_num(*v)));
            seen.insert(canon.to_string());
        }
    }
    for p in declared_order {
        if seen.contains(p) {
            continue;
        }
        if let Some(v) = solved.get(&(name.to_string(), p.clone())) {
            parts.push(format!("{p}={}", fmt_num(*v)));
            seen.insert(p.clone());
        }
    }
    format!("{name} {{ {} }}", parts.join(" "))
}
