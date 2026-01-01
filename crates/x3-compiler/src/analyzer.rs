// crates/x3-compiler/src/analyzer.rs
// Type checking and semantic analysis

use crate::ast::*;
use anyhow::Result;
use std::collections::HashMap;

pub struct Analyzer {
    scopes: Vec<HashMap<String, Type>>,
}

impl Analyzer {
    fn new() -> Self {
        Analyzer {
            scopes: vec![HashMap::new()],
        }
    }

    fn enter_scope(&mut self) {
        self.scopes.push(HashMap::new());
    }

    fn exit_scope(&mut self) {
        self.scopes.pop();
    }

    fn define(&mut self, name: String, ty: Type) {
        if let Some(scope) = self.scopes.last_mut() {
            scope.insert(name, ty);
        }
    }

    fn lookup(&self, name: &str) -> Option<Type> {
        for scope in self.scopes.iter().rev() {
            if let Some(ty) = scope.get(name) {
                return Some(ty.clone());
            }
        }
        None
    }

    fn analyze_program(&mut self, program: &Program) -> Result<AnalyzedProgram> {
        let mut analyzed_modules = Vec::new();
        for module in &program.modules {
            analyzed_modules.push(self.analyze_module(module)?);
        }
        Ok(AnalyzedProgram {
            modules: analyzed_modules,
        })
    }

    fn analyze_module(&mut self, module: &Module) -> Result<AnalyzedModule> {
        let mut analyzed_items = Vec::new();
        for item in &module.items {
            analyzed_items.push(self.analyze_item(item)?);
        }
        Ok(AnalyzedModule {
            name: module.name.clone(),
            items: analyzed_items,
        })
    }

    fn analyze_item(&mut self, item: &Item) -> Result<AnalyzedItem> {
        match item {
            Item::Function(f) => Ok(AnalyzedItem::Function(f.clone())),
            Item::Struct(s) => Ok(AnalyzedItem::Struct(s.clone())),
            Item::Enum(e) => Ok(AnalyzedItem::Enum(e.clone())),
            Item::Event(e) => Ok(AnalyzedItem::Event(e.clone())),
            Item::Error(e) => Ok(AnalyzedItem::Error(e.clone())),
            Item::Strategy(s) => Ok(AnalyzedItem::Strategy(s.clone())),
        }
    }
}

#[derive(Debug, Clone)]
pub struct AnalyzedProgram {
    pub modules: Vec<AnalyzedModule>,
}

#[derive(Debug, Clone)]
pub struct AnalyzedModule {
    pub name: String,
    pub items: Vec<AnalyzedItem>,
}

#[derive(Debug, Clone)]
pub enum AnalyzedItem {
    Function(Function),
    Struct(StructDef),
    Enum(EnumDef),
    Event(EventDef),
    Error(ErrorDef),
    Strategy(Strategy),
}

pub fn analyze(program: &Program) -> Result<AnalyzedProgram> {
    let mut analyzer = Analyzer::new();
    analyzer.analyze_program(program)
}
