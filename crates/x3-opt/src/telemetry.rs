//! Lightweight deterministic telemetry for the optimizer.
//!
//! - Collect per-pass opcode counts and gas totals.
//! - Export JSON + SVG flamegraphs per run.
//! - Mine opcode n-grams for peephole suggestions.
//! - Provide a `PassObserver` implementation for instrumentation hooks.

use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs::{create_dir_all, File};
use std::io::Write;
use std::path::PathBuf;

use x3_ast::{BinaryOp, UnaryOp};
use x3_backend::Opcode as BackendOpcode;
use x3_mir::{MirModule, MirRhs, MirStatement, MirTerminator};
use x3_vm::verifier::opcode_gas_cost;

use crate::optimizer::PassObserver;
use crate::OptResult;

/// Telemetry for a single pass.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PassTelemetry {
    pub pass_name: String,
    pub opcode_counts: BTreeMap<String, u64>,
    pub opcode_gas: BTreeMap<String, u128>,
    pub total_gas: u128,
    pub instr_count: u64,
}

impl PassTelemetry {
    pub fn new(name: &str) -> Self {
        Self {
            pass_name: name.to_string(),
            opcode_counts: BTreeMap::new(),
            opcode_gas: BTreeMap::new(),
            total_gas: 0,
            instr_count: 0,
        }
    }

    pub fn record_op(&mut self, opcode_name: &str, gas_cost: u64) {
        *self
            .opcode_counts
            .entry(opcode_name.to_string())
            .or_insert(0) += 1;
        *self.opcode_gas.entry(opcode_name.to_string()).or_insert(0) += gas_cost as u128;
        self.total_gas += gas_cost as u128;
        self.instr_count += 1;
    }

    pub fn merge_from(&mut self, other: &PassTelemetry) {
        for (k, v) in other.opcode_counts.iter() {
            *self.opcode_counts.entry(k.clone()).or_insert(0) += *v;
        }
        for (k, v) in other.opcode_gas.iter() {
            *self.opcode_gas.entry(k.clone()).or_insert(0) += *v;
        }
        self.total_gas += other.total_gas;
        self.instr_count += other.instr_count;
    }
}

/// Telemetry for an entire optimizer run.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RunTelemetry {
    pub run_id: String,
    pub benches: BTreeMap<String, Vec<PassTelemetry>>,
    pub pass_order: Vec<String>,
}

impl RunTelemetry {
    pub fn new() -> Self {
        Self {
            run_id: format!("{}", Utc::now().format("%Y%m%dT%H%M%S%.3f")),
            benches: BTreeMap::new(),
            pass_order: Vec::new(),
        }
    }

    pub fn set_pass_order(&mut self, order: Vec<String>) {
        self.pass_order = order;
    }

    pub fn record_pass_for_sample(&mut self, sample: &str, pass: PassTelemetry) {
        self.benches
            .entry(sample.to_string())
            .or_insert_with(Vec::new)
            .push(pass);
    }

    /// Write JSON + per-sample flamegraphs.
    pub fn write(&self, base_outdir: &str) -> std::io::Result<()> {
        let dir = PathBuf::from(base_outdir).join(&self.run_id);
        create_dir_all(&dir)?;
        let js = serde_json::to_string_pretty(self).unwrap();
        let mut f = File::create(dir.join("telemetry.json"))?;
        f.write_all(js.as_bytes())?;

        for (sample, passes) in self.benches.iter() {
            let svg = flamegraph_svg(sample, passes, &self.pass_order);
            let mut sf = File::create(dir.join(format!("{}.svg", sample)))?;
            sf.write_all(svg.as_bytes())?;
        }

        Ok(())
    }
}

/// Build a simple stacked flamegraph SVG.
pub fn flamegraph_svg(
    sample_name: &str,
    passes: &[PassTelemetry],
    _pass_order: &[String],
) -> String {
    let width: u32 = 1600;
    let row_h: u32 = 28;
    let padding: u32 = 12;
    let rows = passes.len() as u32;
    let height = rows * (row_h + 4) + padding * 2;

    let mut max_gas: u128 = 1;
    for p in passes {
        if p.total_gas > max_gas {
            max_gas = p.total_gas;
        }
    }

    let mut svg = String::new();
    svg.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>");
    svg.push_str(&format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{w}\" height=\"{h}\" viewBox=\"0 0 {w} {h}\">",
        w = width,
        h = height
    ));
    svg.push_str("<style>text{font-family:monospace;font-size:11px;} .opcode{stroke:#111;stroke-width:0.5;fill-opacity:0.95}</style>");
    svg.push_str(&format!(
        "<text x=\"{pad}\" y=\"{t}\" font-weight=\"bold\">Sample: {name}</text>",
        pad = padding,
        t = padding + 10,
        name = sample_name
    ));

    for (row_idx, pass) in passes.iter().enumerate() {
        let y = padding + 20 + (row_idx as u32) * (row_h + 4);
        let mut items: Vec<(&String, &u128)> = pass.opcode_gas.iter().collect();
        items.sort_by(|a, b| a.0.cmp(b.0));
        let mut x_cursor: f64 = padding as f64;
        for (opname, gas) in items.iter() {
            let gas_val = **gas;
            if gas_val == 0 {
                continue;
            }
            let w_px = ((gas_val as f64) / (max_gas as f64)) * ((width - padding * 2) as f64);
            if w_px < 1.5 {
                x_cursor += 1.5;
                continue;
            }
            let hue = stable_hue(opname);
            let color = format!("hsl({h},60%,60%)", h = hue);
            svg.push_str(&format!(
                "<rect class=\"opcode\" x=\"{x:.2}\" y=\"{y}\" width=\"{w:.2}\" height=\"{hpx}\" fill=\"{col}\" />",
                x = x_cursor,
                y = y,
                w = w_px,
                hpx = row_h,
                col = color
            ));
            if w_px > 48.0 {
                let label = escape_xml(opname);
                svg.push_str(&format!(
                    "<text x=\"{x:.2}\" y=\"{ty}\" fill=\"#111\">{label}</text>",
                    x = x_cursor + 4.0,
                    ty = y + row_h / 2 + 4,
                    label = label
                ));
            }
            x_cursor += w_px;
        }
        svg.push_str(&format!(
            "<text x=\"{pad}\" y=\"{ly}\" fill=\"#000\">{name}</text>",
            pad = 4,
            ly = y - 6,
            name = escape_xml(&pass.pass_name)
        ));
    }

    svg.push_str("</svg>");
    svg
}

/// Observer that records telemetry after every pass.
pub struct TelemetryObserver<'a> {
    telemetry: &'a mut RunTelemetry,
    sample_name: String,
}

impl<'a> TelemetryObserver<'a> {
    pub fn new(telemetry: &'a mut RunTelemetry, sample_name: &str) -> Self {
        Self {
            telemetry,
            sample_name: sample_name.to_string(),
        }
    }
}

impl<'a> PassObserver for TelemetryObserver<'a> {
    fn after_pass(&mut self, pass_name: &str, module: &MirModule) -> OptResult<()> {
        let pass_data = collect_pass_telemetry(pass_name, module);
        self.telemetry
            .record_pass_for_sample(&self.sample_name, pass_data);
        Ok(())
    }
}

/// Collect opcode statistics for an entire module snapshot.
pub fn collect_pass_telemetry(pass_name: &str, module: &MirModule) -> PassTelemetry {
    let mut telemetry = PassTelemetry::new(pass_name);

    for func in module.functions.iter() {
        for block in func.blocks.iter() {
            for stmt in block.statements.iter() {
                let (name, cost) = mir_statement_stats(stmt);
                telemetry.record_op(name, cost);
            }
            if let Some(term) = &block.terminator {
                let (name, cost) = mir_terminator_stats(term);
                telemetry.record_op(name, cost);
            }
        }
    }

    telemetry
}

fn mir_statement_stats(stmt: &MirStatement) -> (&'static str, u64) {
    match &stmt.rhs {
        MirRhs::Literal(_) => ("literal", opcode_cost(BackendOpcode::LoadImm)),
        MirRhs::Unary(op, _) => match op {
            UnaryOp::Negate => ("unary_neg", opcode_cost(BackendOpcode::NegI)),
            UnaryOp::Not => ("unary_not", opcode_cost(BackendOpcode::LNot)),
        },
        MirRhs::Binary(op, _, _) => match op {
            BinaryOp::Add => ("binary_add", opcode_cost(BackendOpcode::AddI)),
            BinaryOp::Sub => ("binary_sub", opcode_cost(BackendOpcode::SubI)),
            BinaryOp::Mul | BinaryOp::Pow => ("binary_mul", opcode_cost(BackendOpcode::MulI)),
            BinaryOp::Div => ("binary_div", opcode_cost(BackendOpcode::DivI)),
            BinaryOp::Mod => ("binary_mod", opcode_cost(BackendOpcode::ModI)),
            BinaryOp::Equal => ("binary_eq", opcode_cost(BackendOpcode::EqI)),
            BinaryOp::NotEqual => ("binary_ne", opcode_cost(BackendOpcode::NeI)),
            BinaryOp::Less => ("binary_lt", opcode_cost(BackendOpcode::LtI)),
            BinaryOp::LessEqual => ("binary_le", opcode_cost(BackendOpcode::LeI)),
            BinaryOp::Greater => ("binary_gt", opcode_cost(BackendOpcode::GtI)),
            BinaryOp::GreaterEqual => ("binary_ge", opcode_cost(BackendOpcode::GeI)),
            BinaryOp::LogicalAnd => ("binary_land", opcode_cost(BackendOpcode::LAnd)),
            BinaryOp::LogicalOr => ("binary_lor", opcode_cost(BackendOpcode::LOr)),
        },
        MirRhs::Call { .. } => ("call", opcode_cost(BackendOpcode::Call)),
        MirRhs::Load { .. } => ("load", opcode_cost(BackendOpcode::AddI)), // estimate
        MirRhs::Store { .. } => ("store", opcode_cost(BackendOpcode::AddI)), // estimate
    }
}

fn mir_terminator_stats(term: &MirTerminator) -> (&'static str, u64) {
    match term {
        MirTerminator::Return(_) => ("return", opcode_cost(BackendOpcode::Ret)),
        MirTerminator::Goto(_) => ("jump", opcode_cost(BackendOpcode::Jump)),
        MirTerminator::Branch { .. } => ("branch", opcode_cost(BackendOpcode::JumpIf)),
    }
}

fn opcode_cost(opcode: BackendOpcode) -> u64 {
    opcode_gas_cost(opcode.to_byte())
}

/// Collect frequent opcode n-grams for pattern mining.
pub fn mine_frequent_ngrams(
    passes: &[PassTelemetry],
    n: usize,
    top_k: usize,
) -> Vec<(String, u64)> {
    let mut counts: BTreeMap<String, u64> = BTreeMap::new();
    for pass in passes {
        let mut seq: Vec<String> = Vec::new();
        for (op, cnt) in pass.opcode_counts.iter() {
            for _ in 0..*cnt {
                seq.push(op.clone());
                if seq.len() > 10_000 {
                    break;
                }
            }
            if seq.len() > 10_000 {
                break;
            }
        }
        for i in 0..seq.len().saturating_sub(n.saturating_sub(1)) {
            let gram = seq[i..i + n].join(" ");
            *counts.entry(gram).or_insert(0) += 1;
        }
    }
    let mut vec: Vec<(String, u64)> = counts.into_iter().collect();
    vec.sort_by(|a, b| {
        if a.1 != b.1 {
            b.1.cmp(&a.1)
        } else {
            a.0.cmp(&b.0)
        }
    });
    vec.into_iter().take(top_k).collect()
}

fn stable_hue(s: &str) -> u32 {
    let mut acc: u32 = 2166136261;
    for b in s.as_bytes() {
        acc = acc.wrapping_mul(16777619) ^ (*b as u32);
    }
    acc % 360
}

fn escape_xml(s: &str) -> String {
    s.replace("&", "&amp;")
        .replace("<", "&lt;")
        .replace(">", "&gt;")
}

#[cfg(test)]
mod tests {
    use super::*;
    use x3_common::{Literal, Span};
    use x3_hir::hir::SymbolId;
    use x3_mir::{MirBlock, MirBlockId, MirFunction, MirStatement, MirTerminator, MirValue};

    fn sample_module() -> MirModule {
        MirModule {
            functions: vec![MirFunction {
                symbol: SymbolId(0),
                params: vec![MirValue(0)],
                entry: MirBlockId(0),
                blocks: vec![MirBlock {
                    id: MirBlockId(0),
                    statements: vec![MirStatement {
                        target: MirValue(1),
                        rhs: MirRhs::Literal(Literal::Integer(42)),
                    }],
                    terminator: Some(MirTerminator::Return(Some(MirValue(1)))),
                }],
                span: Span::dummy(),
            }],
            span: Span::dummy(),
        }
    }

    #[test]
    fn stable_hue_deterministic() {
        assert_eq!(stable_hue("add"), stable_hue("add"));
    }

    #[test]
    fn collect_pass_telemetry_counts() {
        let module = sample_module();
        let pass = collect_pass_telemetry("test", &module);
        assert!(pass.instr_count > 0);
        assert_eq!(pass.pass_name, "test");
    }

    #[test]
    fn mine_ngrams_returns_some() {
        let mut pass = PassTelemetry::new("p");
        pass.opcode_counts.insert("add_i".into(), 3);
        pass.opcode_counts.insert("mov".into(), 2);
        let res = mine_frequent_ngrams(&[pass], 2, 5);
        assert!(!res.is_empty());
    }
}
