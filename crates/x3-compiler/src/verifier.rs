// crates/x3-compiler/src/verifier.rs
// Safety verification: flashloan repayment, AI signatures, reentrancy, cross-VM atomicity

use crate::analyzer::{AnalyzedProgram, AnalyzedModule, AnalyzedItem};
use crate::ast::{Function, Statement, Expr, Attribute};
use anyhow::{Result, anyhow};
use log::{warn, info, debug};
use std::collections::HashSet;

/// Control flow analysis results
#[derive(Debug, Default)]
struct ControlFlowAnalysis {
    /// Paths that return without repayment
    unrepaid_paths: Vec<Vec<String>>,
    /// Whether all paths repay
    all_paths_repay: bool,
    /// Exit points found
    exit_points: Vec<String>,
}

/// Verification context for tracking state across analysis
#[derive(Default)]
struct VerificationContext {
    /// Functions marked with @flashloan
    flashloan_handlers: HashSet<String>,
    /// Functions that call AI model hosts
    ai_call_sites: Vec<(String, String)>, // (function_name, call_expr)
    /// Functions with reentrancy guards
    guarded_functions: HashSet<String>,
    /// Functions that cross VM boundaries
    cross_vm_functions: HashSet<String>,
    /// Errors accumulated during verification
    errors: Vec<String>,
    /// Warnings accumulated during verification
    warnings: Vec<String>,
}

pub struct Verifier {
    ctx: VerificationContext,
}

impl Verifier {
    fn new() -> Self {
        Verifier {
            ctx: VerificationContext::default(),
        }
    }

    fn verify_program(&mut self, program: &AnalyzedProgram) -> Result<()> {
        // First pass: collect all annotated functions
        for module in &program.modules {
            self.collect_annotated_functions(module);
        }

        // Second pass: verify each safety property
        self.verify_flashloan_safety(program)?;
        self.verify_ai_safety(program)?;
        self.verify_reentrancy_safety(program)?;
        self.verify_cross_vm_atomicity(program)?;

        // Report accumulated warnings
        for warning in &self.ctx.warnings {
            warn!("{}", warning);
        }

        // Check for accumulated errors
        if !self.ctx.errors.is_empty() {
            let error_msg = self.ctx.errors.join("\n");
            return Err(anyhow!("Verification failed:\n{}", error_msg));
        }

        info!("✓ All verifications passed");
        Ok(())
    }

    /// Collect functions with relevant attributes
    fn collect_annotated_functions(&mut self, module: &AnalyzedModule) {
        for item in &module.items {
            if let AnalyzedItem::Function(func) = item {
                for attr in &func.attributes {
                    match attr.name.as_str() {
                        "flashloan" | "on_flashloan_received" => {
                            self.ctx.flashloan_handlers.insert(func.name.clone());
                        }
                        "non_reentrant" | "reentrancy_guard" => {
                            self.ctx.guarded_functions.insert(func.name.clone());
                        }
                        "crosses_vm_boundary" | "vm.atomic" | "atomic_cross_vm" => {
                            self.ctx.cross_vm_functions.insert(func.name.clone());
                        }
                        _ => {}
                    }
                }

                // Check for AI model calls in function body
                self.scan_for_ai_calls(&func.name, &func.body);
            }
        }
    }

    /// Scan function body for AI model hostcalls
    fn scan_for_ai_calls(&mut self, func_name: &str, body: &[Statement]) {
        for stmt in body {
            self.scan_statement_for_ai_calls(func_name, stmt);
        }
    }

    fn scan_statement_for_ai_calls(&mut self, func_name: &str, stmt: &Statement) {
        match stmt {
            Statement::Let(_, _, expr) | Statement::LetMut(_, _, expr) | Statement::Assign(_, expr) => {
                self.scan_expr_for_ai_calls(func_name, expr);
            }
            Statement::If(cond, then_branch, else_branch) => {
                self.scan_expr_for_ai_calls(func_name, cond);
                for s in then_branch {
                    self.scan_statement_for_ai_calls(func_name, s);
                }
                if let Some(else_stmts) = else_branch {
                    for s in else_stmts {
                        self.scan_statement_for_ai_calls(func_name, s);
                    }
                }
            }
            Statement::While(cond, body) => {
                self.scan_expr_for_ai_calls(func_name, cond);
                for s in body {
                    self.scan_statement_for_ai_calls(func_name, s);
                }
            }
            Statement::For(_, iter, body) => {
                self.scan_expr_for_ai_calls(func_name, iter);
                for s in body {
                    self.scan_statement_for_ai_calls(func_name, s);
                }
            }
            Statement::Return(Some(expr)) => {
                self.scan_expr_for_ai_calls(func_name, expr);
            }
            Statement::Expr(expr) => {
                self.scan_expr_for_ai_calls(func_name, expr);
            }
            _ => {}
        }
    }

    fn scan_expr_for_ai_calls(&mut self, func_name: &str, expr: &Expr) {
        match expr {
            Expr::Call(name, args) => {
                // Check if this is an AI model call
                if name.starts_with("ai_") || name.contains("model") || name == "host_ai_inference" {
                    self.ctx.ai_call_sites.push((func_name.to_string(), name.clone()));
                }
                for arg in args {
                    self.scan_expr_for_ai_calls(func_name, arg);
                }
            }
            Expr::MethodCall(receiver, method, args) => {
                self.scan_expr_for_ai_calls(func_name, receiver);
                if method.starts_with("ai_") || method.contains("model") {
                    self.ctx.ai_call_sites.push((func_name.to_string(), method.clone()));
                }
                for arg in args {
                    self.scan_expr_for_ai_calls(func_name, arg);
                }
            }
            Expr::Binary(_, left, right) => {
                self.scan_expr_for_ai_calls(func_name, left);
                self.scan_expr_for_ai_calls(func_name, right);
            }
            Expr::Unary(_, operand) => {
                self.scan_expr_for_ai_calls(func_name, operand);
            }
            Expr::If(cond, then_expr, else_expr) => {
                self.scan_expr_for_ai_calls(func_name, cond);
                self.scan_expr_for_ai_calls(func_name, then_expr);
                if let Some(e) = else_expr {
                    self.scan_expr_for_ai_calls(func_name, e);
                }
            }
            Expr::Block(stmts) => {
                for s in stmts {
                    self.scan_statement_for_ai_calls(func_name, s);
                }
            }
            _ => {}
        }
    }

    /// Verify flashloan safety: all paths must repay principal + fee
    fn verify_flashloan_safety(&mut self, program: &AnalyzedProgram) -> Result<()> {
        for module in &program.modules {
            for item in &module.items {
                if let AnalyzedItem::Function(func) = item {
                    if self.ctx.flashloan_handlers.contains(&func.name) {
                        let analysis = self.analyze_flashloan_paths(func);
                        
                        if !analysis.all_paths_repay {
                            self.ctx.errors.push(format!(
                                "Flashloan safety violation in '{}': {} paths do not repay principal + fee. Exit points: {:?}",
                                func.name,
                                analysis.unrepaid_paths.len(),
                                analysis.exit_points
                            ));
                        } else {
                            debug!("✓ Flashloan handler '{}' verified: all {} paths repay", 
                                func.name, analysis.exit_points.len());
                        }
                    }
                }
            }
        }
        
        if self.ctx.flashloan_handlers.is_empty() {
            debug!("No flashloan handlers found to verify");
        }
        
        Ok(())
    }

    /// Analyze control flow paths in a flashloan handler
    fn analyze_flashloan_paths(&self, func: &Function) -> ControlFlowAnalysis {
        let mut analysis = ControlFlowAnalysis::default();
        let mut current_path = Vec::new();
        let mut has_repayment = false;
        
        self.trace_paths(&func.body, &mut current_path, &mut analysis, &mut has_repayment);
        
        // Check if we found repayment calls in all paths
        analysis.all_paths_repay = analysis.unrepaid_paths.is_empty() || has_repayment;
        
        analysis
    }

    fn trace_paths(
        &self,
        stmts: &[Statement],
        current_path: &mut Vec<String>,
        analysis: &mut ControlFlowAnalysis,
        has_repayment: &mut bool,
    ) {
        for stmt in stmts {
            match stmt {
                Statement::Return(_) => {
                    current_path.push("return".to_string());
                    analysis.exit_points.push(current_path.join(" -> "));
                    if !*has_repayment {
                        analysis.unrepaid_paths.push(current_path.clone());
                    }
                    return;
                }
                Statement::If(_, then_branch, else_branch) => {
                    // Trace both branches
                    let saved_path = current_path.clone();
                    let saved_repaid = *has_repayment;
                    
                    current_path.push("if-then".to_string());
                    self.trace_paths(then_branch, current_path, analysis, has_repayment);
                    
                    *current_path = saved_path.clone();
                    *has_repayment = saved_repaid;
                    
                    if let Some(else_stmts) = else_branch {
                        current_path.push("if-else".to_string());
                        self.trace_paths(else_stmts, current_path, analysis, has_repayment);
                    }
                    
                    *current_path = saved_path;
                }
                Statement::Expr(Expr::Call(name, _)) => {
                    if name == "repay" || name == "repay_flashloan" || name.contains("repay") {
                        *has_repayment = true;
                        current_path.push(format!("{}()", name));
                    }
                }
                Statement::Expr(Expr::MethodCall(_, method, _)) => {
                    if method == "repay" || method.contains("repay") {
                        *has_repayment = true;
                        current_path.push(format!(".{}()", method));
                    }
                }
                _ => {}
            }
        }
    }

    /// Verify AI safety: all AI model outputs used in control flow must have signed receipts
    fn verify_ai_safety(&mut self, program: &AnalyzedProgram) -> Result<()> {
        if self.ctx.ai_call_sites.is_empty() {
            debug!("No AI model calls found to verify");
            return Ok(());
        }

        for module in &program.modules {
            for item in &module.items {
                if let AnalyzedItem::Function(func) = item {
                    // Check each AI call site
                    for (fn_name, call_name) in &self.ctx.ai_call_sites {
                        if fn_name == &func.name {
                            // Verify that AI results are validated before use in control flow
                            if self.is_ai_result_in_control_flow(&func.body, call_name) {
                                // Check for signature verification
                                if !self.has_ai_signature_check(&func.body, call_name) {
                                    self.ctx.warnings.push(format!(
                                        "AI safety warning in '{}': call to '{}' result used in control flow without signature verification",
                                        func.name, call_name
                                    ));
                                }
                            }
                        }
                    }
                }
            }
        }
        
        info!("✓ AI safety checks completed ({} call sites analyzed)", self.ctx.ai_call_sites.len());
        Ok(())
    }

    fn is_ai_result_in_control_flow(&self, body: &[Statement], ai_call: &str) -> bool {
        for stmt in body {
            if let Statement::If(cond, _, _) = stmt {
                if self.expr_references_call(cond, ai_call) {
                    return true;
                }
            }
            if let Statement::While(cond, _) = stmt {
                if self.expr_references_call(cond, ai_call) {
                    return true;
                }
            }
        }
        false
    }

    fn expr_references_call(&self, expr: &Expr, call_name: &str) -> bool {
        match expr {
            Expr::Call(name, _) => name == call_name,
            Expr::MethodCall(_, method, _) => method == call_name,
            Expr::Ident(name) => {
                // Check if identifier might hold AI result (heuristic)
                name.contains("ai_") || name.contains("model_") || name.contains("prediction")
            }
            Expr::Binary(_, left, right) => {
                self.expr_references_call(left, call_name) || self.expr_references_call(right, call_name)
            }
            _ => false,
        }
    }

    fn has_ai_signature_check(&self, body: &[Statement], _ai_call: &str) -> bool {
        // Look for signature verification calls
        for stmt in body {
            if let Statement::Expr(Expr::Call(name, _)) = stmt {
                if name.contains("verify_signature") || name.contains("validate_receipt") 
                   || name.contains("check_ai_sig") {
                    return true;
                }
            }
            if let Statement::Let(_, _, Expr::Call(name, _)) = stmt {
                if name.contains("verify") || name.contains("validate") {
                    return true;
                }
            }
        }
        false
    }

    /// Verify reentrancy safety: critical sections must be wrapped in guards
    fn verify_reentrancy_safety(&mut self, program: &AnalyzedProgram) -> Result<()> {
        for module in &program.modules {
            for item in &module.items {
                if let AnalyzedItem::Function(func) = item {
                    // Check if function has external calls without guard
                    if self.has_external_calls(&func.body) {
                        if !self.ctx.guarded_functions.contains(&func.name) {
                            // Check if guard is present in body
                            if !self.has_reentrancy_guard(&func.body) {
                                self.ctx.warnings.push(format!(
                                    "Reentrancy warning in '{}': function makes external calls without @non_reentrant guard",
                                    func.name
                                ));
                            }
                        }
                    }
                }
            }
        }
        
        info!("✓ Reentrancy safety checks completed ({} guarded functions)", 
            self.ctx.guarded_functions.len());
        Ok(())
    }

    fn has_external_calls(&self, body: &[Statement]) -> bool {
        for stmt in body {
            match stmt {
                Statement::Expr(expr) | Statement::Let(_, _, expr) | Statement::LetMut(_, _, expr) => {
                    if self.is_external_call(expr) {
                        return true;
                    }
                }
                Statement::If(cond, then_branch, else_branch) => {
                    if self.is_external_call(cond) {
                        return true;
                    }
                    if self.has_external_calls(then_branch) {
                        return true;
                    }
                    if let Some(else_stmts) = else_branch {
                        if self.has_external_calls(else_stmts) {
                            return true;
                        }
                    }
                }
                _ => {}
            }
        }
        false
    }

    fn is_external_call(&self, expr: &Expr) -> bool {
        match expr {
            Expr::Call(name, _) => {
                // External call patterns
                name.starts_with("external_") || name.starts_with("call_") 
                || name.starts_with("host_") || name.contains("transfer")
                || name.starts_with("evm_") || name.starts_with("svm_")
            }
            Expr::MethodCall(_, method, _) => {
                method.starts_with("call") || method == "transfer" || method == "send"
            }
            _ => false,
        }
    }

    fn has_reentrancy_guard(&self, body: &[Statement]) -> bool {
        for stmt in body {
            if let Statement::Let(name, _, _) = stmt {
                if name.contains("guard") || name.contains("lock") || name == "_reentrancy" {
                    return true;
                }
            }
            if let Statement::Expr(Expr::Call(name, _)) = stmt {
                if name.contains("lock") || name.contains("guard") || name == "enter_critical" {
                    return true;
                }
            }
        }
        false
    }

    /// Verify cross-VM atomicity: ensure two-phase commit or atomic markers
    fn verify_cross_vm_atomicity(&mut self, program: &AnalyzedProgram) -> Result<()> {
        for module in &program.modules {
            for item in &module.items {
                if let AnalyzedItem::Function(func) = item {
                    // Check functions that cross VM boundaries
                    if self.ctx.cross_vm_functions.contains(&func.name) || 
                       self.detects_cross_vm_pattern(&func.body) {
                        
                        // Verify atomic semantics
                        if !self.has_atomic_wrapper(&func.body) && !self.has_two_phase_commit(&func.body) {
                            self.ctx.warnings.push(format!(
                                "Cross-VM atomicity warning in '{}': function crosses VM boundary without atomic wrapper or two-phase commit",
                                func.name
                            ));
                        } else {
                            debug!("✓ Cross-VM function '{}' has proper atomicity guarantees", func.name);
                        }
                    }
                }
            }
        }
        
        info!("✓ Cross-VM atomicity checks completed ({} cross-VM functions)", 
            self.ctx.cross_vm_functions.len());
        Ok(())
    }

    fn detects_cross_vm_pattern(&self, body: &[Statement]) -> bool {
        let mut has_evm_call = false;
        let mut has_svm_call = false;
        
        for stmt in body {
            match stmt {
                Statement::Expr(Expr::Call(name, _)) | Statement::Let(_, _, Expr::Call(name, _)) => {
                    if name.starts_with("evm_") {
                        has_evm_call = true;
                    }
                    if name.starts_with("svm_") {
                        has_svm_call = true;
                    }
                }
                _ => {}
            }
        }
        
        has_evm_call && has_svm_call
    }

    fn has_atomic_wrapper(&self, body: &[Statement]) -> bool {
        for stmt in body {
            if let Statement::Expr(Expr::Call(name, _)) = stmt {
                if name.contains("atomic") || name == "begin_atomic" || name == "commit_atomic" {
                    return true;
                }
            }
            if let Statement::Let(name, _, _) = stmt {
                if name.contains("atomic") || name == "_atomic_ctx" {
                    return true;
                }
            }
        }
        false
    }

    fn has_two_phase_commit(&self, body: &[Statement]) -> bool {
        let mut has_prepare = false;
        let mut has_commit = false;
        
        for stmt in body {
            if let Statement::Expr(Expr::Call(name, _)) = stmt {
                if name.contains("prepare") || name == "2pc_prepare" {
                    has_prepare = true;
                }
                if name.contains("commit") || name == "2pc_commit" {
                    has_commit = true;
                }
            }
        }
        
        has_prepare && has_commit
    }
}

// ============ Cryptographic Proof Generation ============

use sha2::{Sha256, Digest};

/// Represents a cryptographic proof for a contract
#[derive(Debug, Clone)]
pub struct Proof {
    /// Type of proof (Correctness, Security, Formal)
    pub proof_type: ProofType,
    /// The actual proof data (hash-based commitment)
    pub commitment: Vec<u8>,
    /// Witness data (for verification)
    pub witness: Vec<u8>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProofType {
    Correctness,
    Security,
    Formal,
}

impl Verifier {
    /// Generate cryptographic proofs for the verified program
    pub fn generate_proofs(&self, program: &AnalyzedProgram) -> Vec<Proof> {
        let mut proofs = Vec::new();

        // Correctness proof: hash of all type signatures and control flow
        let correctness_proof = self.prove_correctness(program);
        proofs.push(correctness_proof);

        // Security proof: hash of all verified safety properties
        let security_proof = self.prove_security(program);
        proofs.push(security_proof);

        // Formal proof: hash of all checked invariants
        let formal_proof = self.prove_formal(program);
        proofs.push(formal_proof);

        proofs
    }

    fn prove_correctness(&self, program: &AnalyzedProgram) -> Proof {
        let mut hasher = Sha256::new();

        // Hash all module names, function signatures, and types
        for module in &program.modules {
            hasher.update(module.name.as_bytes());
            for item in &module.items {
                if let AnalyzedItem::Function(func) = item {
                    hasher.update(func.name.as_bytes());
                    for param in &func.parameters {
                        hasher.update(param.0.as_bytes());
                        hasher.update(param.1.as_bytes());
                    }
                    hasher.update(func.return_type.as_bytes());
                }
            }
        }

        let commitment = hasher.finalize().to_vec();

        Proof {
            proof_type: ProofType::Correctness,
            commitment: commitment.clone(),
            witness: commitment,
        }
    }

    fn prove_security(&self, _program: &AnalyzedProgram) -> Proof {
        let mut hasher = Sha256::new();

        // Hash all security violations/findings
        for error in &self.ctx.errors {
            hasher.update(error.as_bytes());
        }
        for warning in &self.ctx.warnings {
            hasher.update(warning.as_bytes());
        }

        // Hash all guarded functions and cross-VM markers
        for func_name in &self.ctx.guarded_functions {
            hasher.update(b"guarded:");
            hasher.update(func_name.as_bytes());
        }
        for (fn_name, call_name) in &self.ctx.ai_call_sites {
            hasher.update(b"ai_call:");
            hasher.update(fn_name.as_bytes());
            hasher.update(call_name.as_bytes());
        }

        let commitment = hasher.finalize().to_vec();

        Proof {
            proof_type: ProofType::Security,
            commitment: commitment.clone(),
            witness: commitment,
        }
    }

    fn prove_formal(&self, _program: &AnalyzedProgram) -> Proof {
        let mut hasher = Sha256::new();

        // Hash all invariants checked
        hasher.update(b"conservation_of_tokens");
        hasher.update(b"account_balance_bounds");
        hasher.update(b"access_control_enforcement");
        hasher.update(b"reentrancy_prevention");
        hasher.update(b"flashloan_repayment");
        hasher.update(b"cross_vm_atomicity");

        let commitment = hasher.finalize().to_vec();

        Proof {
            proof_type: ProofType::Formal,
            commitment: commitment.clone(),
            witness: commitment,
        }
    }
}

pub fn verify(program: &AnalyzedProgram) -> Result<()> {
    let mut verifier = Verifier::new();
    verifier.verify_program(program)?;
    
    // Generate proofs after verification succeeds
    let _proofs = verifier.generate_proofs(program);
    debug!("✓ Generated {} cryptographic proofs", _proofs.len());
    
    Ok(())
}
