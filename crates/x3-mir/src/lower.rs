use std::collections::HashMap;

use x3_common::Span;
use x3_hir::hir::{AssignTarget, HirExpr, HirExprKind, HirFunction, HirModule, HirStmt, SymbolId};

use crate::error::MirError;
use crate::mir::*;

/// Converts HIR into deterministic SSA-form MIR.
pub struct MirLowerer;

impl MirLowerer {
    pub fn lower(module: &HirModule) -> Result<MirModule, MirError> {
        let mut functions = Vec::new();
        for function in &module.functions {
            let mut builder = MirFunctionBuilder::new(function.symbol, function.span);
            functions.push(builder.lower_function(function)?);
        }
        Ok(MirModule::new(functions, module.span))
    }
}

struct MirFunctionBuilder {
    symbol: SymbolId,
    span: Span,
    blocks: Vec<MirBlock>,
    current_block: MirBlockId,
    params: Vec<MirValue>,
    value_map: HashMap<SymbolId, MirValue>,
    next_value: usize,
}

impl MirFunctionBuilder {
    fn new(symbol: SymbolId, span: Span) -> Self {
        let mut builder = Self {
            symbol,
            span,
            blocks: Vec::new(),
            current_block: MirBlockId(0),
            params: Vec::new(),
            value_map: HashMap::new(),
            next_value: 0,
        };
        let entry = builder.create_block();
        builder.current_block = entry;
        builder
    }

    fn lower_function(mut self, function: &HirFunction) -> Result<MirFunction, MirError> {
        for param in &function.params {
            let value = self.allocate_value();
            self.value_map.insert(param.symbol, value);
            self.params.push(value);
        }
        self.lower_statements(&function.body)?;
        self.ensure_current_block_has_terminator();
        Ok(MirFunction {
            symbol: self.symbol,
            params: self.params,
            entry: MirBlockId(0),
            blocks: self.blocks,
            span: self.span,
        })
    }

    fn lower_statements(&mut self, statements: &[HirStmt]) -> Result<(), MirError> {
        for statement in statements {
            self.lower_statement(statement)?;
        }
        Ok(())
    }

    fn lower_statement(&mut self, statement: &HirStmt) -> Result<(), MirError> {
        match statement {
            HirStmt::Let { symbol, value, .. } => {
                let evaluated = self.lower_expr(value)?;
                self.value_map.insert(*symbol, evaluated);
            }
            HirStmt::Assign {
                target,
                value,
                span: _,
            } => {
                match target {
                    AssignTarget::Variable(symbol) => {
                        if !self.value_map.contains_key(symbol) {
                            return Err(MirError::new(format!(
                                "assignment target not found for symbol {:?}",
                                symbol
                            )));
                        }
                        let evaluated = self.lower_expr(value)?;
                        self.value_map.insert(*symbol, evaluated);
                    }
                    AssignTarget::Field { .. } | AssignTarget::Index { .. } => {
                        // TODO: Handle field/index assignments
                        let _ = self.lower_expr(value)?;
                    }
                }
            }
            HirStmt::Expr(expr) => {
                self.lower_expr(expr)?;
            }
            HirStmt::Return { value, span: _ } => {
                let payload = value
                    .as_ref()
                    .map(|expr| self.lower_expr(expr))
                    .transpose()?;
                self.set_terminator(MirTerminator::Return(payload));
                let next = self.create_block();
                self.current_block = next;
            }
            HirStmt::If {
                condition,
                then_block,
                else_block,
                span: _,
            } => {
                self.lower_if(condition, then_block, else_block)?;
            }
            HirStmt::While {
                condition,
                body,
                label: _,
                span: _,
            } => {
                self.lower_while(condition, body)?;
            }
            HirStmt::Break { .. } | HirStmt::Continue { .. } => {
                // TODO: Handle break/continue with labels
            }
            HirStmt::AtomicBegin { .. } | HirStmt::AtomicEnd { .. } => {
                // TODO: Handle atomic blocks
            }
            HirStmt::Emit { .. } | HirStmt::AgentInit { .. } => {
                // TODO: Handle emit and agent init
            }
        }
        Ok(())
    }

    fn lower_if(
        &mut self,
        condition: &HirExpr,
        then_block: &[HirStmt],
        else_block: &[HirStmt],
    ) -> Result<(), MirError> {
        let cond_value = self.lower_expr(condition)?;
        let then_id = self.create_block();
        let else_id = self.create_block();
        let merge_id = self.create_block();
        self.set_terminator(MirTerminator::Branch {
            cond: cond_value,
            then_block: then_id,
            else_block: else_id,
        });
        self.current_block = then_id;
        self.lower_statements(then_block)?;
        self.ensure_goto(merge_id);
        self.current_block = else_id;
        self.lower_statements(else_block)?;
        self.ensure_goto(merge_id);
        self.current_block = merge_id;
        Ok(())
    }

    fn lower_while(&mut self, condition: &HirExpr, body: &[HirStmt]) -> Result<(), MirError> {
        let cond_id = self.create_block();
        let body_id = self.create_block();
        let merge_id = self.create_block();
        self.set_terminator(MirTerminator::Goto(cond_id));
        self.current_block = cond_id;
        let cond_value = self.lower_expr(condition)?;
        self.set_terminator(MirTerminator::Branch {
            cond: cond_value,
            then_block: body_id,
            else_block: merge_id,
        });
        self.current_block = body_id;
        self.lower_statements(body)?;
        self.ensure_goto(cond_id);
        self.current_block = merge_id;
        Ok(())
    }

    fn lower_expr(&mut self, expr: &HirExpr) -> Result<MirValue, MirError> {
        match &expr.kind {
            HirExprKind::Literal(literal) => Ok(self.emit_literal(literal.clone())),
            HirExprKind::Var(symbol) => self
                .value_map
                .get(symbol)
                .copied()
                .ok_or_else(|| MirError::new(format!("value for symbol {:?} missing", symbol))),
            HirExprKind::Binary { op, left, right } => {
                let left_val = self.lower_expr(left)?;
                let right_val = self.lower_expr(right)?;
                Ok(self.emit_assignment(MirRhs::Binary(*op, left_val, right_val)))
            }
            HirExprKind::Unary { op, operand } => {
                let val = self.lower_expr(operand)?;
                Ok(self.emit_assignment(MirRhs::Unary(*op, val)))
            }
            HirExprKind::Call { callee, args } => {
                let mut mir_args = Vec::new();
                for arg in args {
                    mir_args.push(self.lower_expr(arg)?);
                }
                Ok(self.emit_assignment(MirRhs::Call {
                    target: *callee,
                    args: mir_args,
                }))
            }
            HirExprKind::MethodCall {
                receiver,
                method,
                args,
            } => {
                // TODO: Handle method calls properly
                let _ = self.lower_expr(receiver)?;
                for arg in args {
                    self.lower_expr(arg)?;
                }
                Ok(self.emit_assignment(MirRhs::Literal(x3_common::Literal::Unit)))
            }
            HirExprKind::Field { object, field: _ } => {
                // TODO: Handle field access
                self.lower_expr(object)
            }
            HirExprKind::Index { array, index } => {
                // TODO: Handle index access properly
                let _ = self.lower_expr(array)?;
                let _ = self.lower_expr(index)?;
                Ok(self.emit_assignment(MirRhs::Literal(x3_common::Literal::Unit)))
            }
            HirExprKind::Array(elements) => {
                // TODO: Handle array literals
                for elem in elements {
                    self.lower_expr(elem)?;
                }
                Ok(self.emit_assignment(MirRhs::Literal(x3_common::Literal::Unit)))
            }
            HirExprKind::Tuple(elements) => {
                // TODO: Handle tuple literals
                for elem in elements {
                    self.lower_expr(elem)?;
                }
                Ok(self.emit_assignment(MirRhs::Literal(x3_common::Literal::Unit)))
            }
            HirExprKind::Block { stmts, expr } => {
                self.lower_statements(stmts)?;
                if let Some(e) = expr {
                    self.lower_expr(e)
                } else {
                    Ok(self.emit_assignment(MirRhs::Literal(x3_common::Literal::Unit)))
                }
            }
            HirExprKind::IfExpr {
                condition,
                then_expr,
                else_expr,
            } => {
                // TODO: Handle if-expressions with phi nodes
                let _ = self.lower_expr(condition)?;
                let _ = self.lower_expr(then_expr)?;
                let _ = self.lower_expr(else_expr)?;
                Ok(self.emit_assignment(MirRhs::Literal(x3_common::Literal::Unit)))
            }
            HirExprKind::Cast { expr, target_ty: _ } => {
                // TODO: Handle casts properly
                self.lower_expr(expr)
            }
            HirExprKind::ContextAccess(_field) => {
                // TODO: Handle context access
                Ok(self.emit_assignment(MirRhs::Literal(x3_common::Literal::Unit)))
            }
            HirExprKind::VmIntrinsic {
                vm: _,
                intrinsic: _,
                args,
            } => {
                // TODO: Handle VM intrinsics
                for arg in args {
                    self.lower_expr(arg)?;
                }
                Ok(self.emit_assignment(MirRhs::Literal(x3_common::Literal::Unit)))
            }
            HirExprKind::SelfRef => {
                // TODO: Handle self reference
                Ok(self.emit_assignment(MirRhs::Literal(x3_common::Literal::Unit)))
            }
        }
    }

    fn emit_literal(&mut self, literal: x3_common::Literal) -> MirValue {
        self.emit_assignment(MirRhs::Literal(literal))
    }

    fn emit_assignment(&mut self, rhs: MirRhs) -> MirValue {
        let target = self.allocate_value();
        let block = self.current_block_mut();
        block.statements.push(MirStatement { target, rhs });
        target
    }

    fn ensure_goto(&mut self, target: MirBlockId) {
        let block = self.current_block_mut();
        if block.terminator.is_none() {
            block.terminator = Some(MirTerminator::Goto(target));
        }
    }

    fn ensure_current_block_has_terminator(&mut self) {
        let block = self.current_block_mut();
        if block.terminator.is_none() {
            block.terminator = Some(MirTerminator::Return(None));
        }
    }

    fn set_terminator(&mut self, terminator: MirTerminator) {
        let block = self.current_block_mut();
        if block.terminator.is_none() {
            block.terminator = Some(terminator);
        }
    }

    fn create_block(&mut self) -> MirBlockId {
        let id = MirBlockId(self.blocks.len());
        self.blocks.push(MirBlock {
            id,
            statements: Vec::new(),
            terminator: None,
        });
        id
    }

    fn current_block_mut(&mut self) -> &mut MirBlock {
        &mut self.blocks[self.current_block.0]
    }

    fn allocate_value(&mut self) -> MirValue {
        let value = MirValue(self.next_value);
        self.next_value += 1;
        value
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use x3_hir::lower::HirLowerer;
    use x3_parser::Parser;

    #[test]
    fn lower_function_with_return() {
        let source = "fn add(x: i32, y: i32) -> i32 { return x + y; }";
        let mut parser = Parser::from_source(source);
        let module = parser.parse_module().expect("parse");
        let hir = HirLowerer::lower(module).expect("hir");
        let mir = MirLowerer::lower(&hir).expect("mir");
        assert_eq!(mir.functions.len(), 1);
        let function = &mir.functions[0];
        assert!(function.blocks.iter().any(|block| match block.terminator {
            Some(MirTerminator::Return(Some(_))) => true,
            _ => false,
        }));
    }
}
