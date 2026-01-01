// crates/x3-compiler/src/repl.rs
// Interactive REPL for X3 language development and testing

use std::collections::HashMap;
use std::io::{self, Write};
use std::path::PathBuf;

use crate::analyzer::Analyzer;
use crate::ast::{AstNode, Expression, Statement, Type};
use crate::error::X3Error;
use crate::lexer::Lexer;
use crate::parser::Parser;

/// REPL configuration
#[derive(Debug, Clone)]
pub struct ReplConfig {
    pub load_stdlib: bool,
    pub history_file: Option<PathBuf>,
    pub preload_file: Option<PathBuf>,
    pub show_ast: bool,
    pub show_types: bool,
    pub auto_import: bool,
}

impl Default for ReplConfig {
    fn default() -> Self {
        Self {
            load_stdlib: true,
            history_file: None,
            preload_file: None,
            show_ast: false,
            show_types: true,
            auto_import: true,
        }
    }
}

/// REPL state holding definitions and context
pub struct ReplState {
    /// User-defined variables and their types
    pub variables: HashMap<String, (Type, String)>,
    /// User-defined functions
    pub functions: HashMap<String, AstNode>,
    /// User-defined structs
    pub structs: HashMap<String, AstNode>,
    /// Imported modules
    pub imports: Vec<String>,
    /// Command history
    pub history: Vec<String>,
    /// Current input buffer (for multi-line)
    pub input_buffer: String,
    /// Analyzer instance
    pub analyzer: Analyzer,
}

impl ReplState {
    pub fn new() -> Self {
        Self {
            variables: HashMap::new(),
            functions: HashMap::new(),
            structs: HashMap::new(),
            imports: Vec::new(),
            history: Vec::new(),
            input_buffer: String::new(),
            analyzer: Analyzer::new(),
        }
    }

    /// Reset state to initial
    pub fn reset(&mut self) {
        self.variables.clear();
        self.functions.clear();
        self.structs.clear();
        self.imports.clear();
        self.input_buffer.clear();
    }
}

/// REPL command types
#[derive(Debug)]
pub enum ReplCommand {
    /// Evaluate X3 expression
    Eval(String),
    /// Define a binding (let x = ...)
    Define(String),
    /// Import a module
    Import(String),
    /// Show help
    Help,
    /// Show current state
    State,
    /// Clear state
    Clear,
    /// Show type of expression
    TypeOf(String),
    /// Show AST for expression
    Ast(String),
    /// Load a file
    Load(PathBuf),
    /// Save session to file
    Save(PathBuf),
    /// Show history
    History,
    /// Quit REPL
    Quit,
    /// Set option
    Set(String, String),
    /// Show environment info
    Env,
    /// Run gas analysis
    Gas(String),
}

impl ReplCommand {
    /// Parse a REPL command from input
    pub fn parse(input: &str) -> Self {
        let trimmed = input.trim();
        
        if trimmed.is_empty() {
            return ReplCommand::Eval(String::new());
        }

        // Check for meta-commands (start with :)
        if trimmed.starts_with(':') {
            let parts: Vec<&str> = trimmed[1..].splitn(2, ' ').collect();
            let cmd = parts[0].to_lowercase();
            let arg = parts.get(1).map(|s| s.trim().to_string()).unwrap_or_default();

            match cmd.as_str() {
                "help" | "h" | "?" => ReplCommand::Help,
                "quit" | "q" | "exit" => ReplCommand::Quit,
                "clear" | "reset" => ReplCommand::Clear,
                "state" | "s" => ReplCommand::State,
                "history" | "hist" => ReplCommand::History,
                "type" | "t" => ReplCommand::TypeOf(arg),
                "ast" | "a" => ReplCommand::Ast(arg),
                "load" | "l" => ReplCommand::Load(PathBuf::from(arg)),
                "save" => ReplCommand::Save(PathBuf::from(arg)),
                "import" | "i" => ReplCommand::Import(arg),
                "set" => {
                    let set_parts: Vec<&str> = arg.splitn(2, '=').collect();
                    if set_parts.len() == 2 {
                        ReplCommand::Set(
                            set_parts[0].trim().to_string(),
                            set_parts[1].trim().to_string(),
                        )
                    } else {
                        ReplCommand::Help
                    }
                }
                "env" => ReplCommand::Env,
                "gas" | "g" => ReplCommand::Gas(arg),
                _ => {
                    println!("Unknown command: {}", cmd);
                    ReplCommand::Help
                }
            }
        } else if trimmed.starts_with("import ") {
            ReplCommand::Import(trimmed[7..].trim().to_string())
        } else if trimmed.starts_with("let ") || trimmed.starts_with("const ") {
            ReplCommand::Define(trimmed.to_string())
        } else if trimmed.starts_with("fn ") || trimmed.starts_with("pub fn ") {
            ReplCommand::Define(trimmed.to_string())
        } else if trimmed.starts_with("struct ") || trimmed.starts_with("pub struct ") {
            ReplCommand::Define(trimmed.to_string())
        } else {
            ReplCommand::Eval(trimmed.to_string())
        }
    }
}

/// The X3 REPL
pub struct Repl {
    pub config: ReplConfig,
    pub state: ReplState,
    prompt: String,
    continuation_prompt: String,
}

impl Repl {
    pub fn new(config: ReplConfig) -> Self {
        Self {
            config,
            state: ReplState::new(),
            prompt: "x3> ".to_string(),
            continuation_prompt: "... ".to_string(),
        }
    }

    /// Run the REPL loop
    pub fn run(&mut self) -> Result<(), X3Error> {
        self.print_banner();

        // Load stdlib if configured
        if self.config.load_stdlib {
            self.load_stdlib()?;
        }

        // Preload file if specified
        if let Some(ref path) = self.config.preload_file.clone() {
            self.load_file(path)?;
        }

        loop {
            // Print prompt
            let prompt = if self.state.input_buffer.is_empty() {
                &self.prompt
            } else {
                &self.continuation_prompt
            };
            print!("{}", prompt);
            io::stdout().flush().ok();

            // Read input
            let mut input = String::new();
            match io::stdin().read_line(&mut input) {
                Ok(0) => break, // EOF
                Ok(_) => {}
                Err(e) => {
                    eprintln!("Error reading input: {}", e);
                    continue;
                }
            }

            let input = input.trim_end();

            // Handle multi-line input
            if self.needs_continuation(input) {
                self.state.input_buffer.push_str(input);
                self.state.input_buffer.push('\n');
                continue;
            }

            // Combine buffer with current input
            let full_input = if self.state.input_buffer.is_empty() {
                input.to_string()
            } else {
                let mut combined = std::mem::take(&mut self.state.input_buffer);
                combined.push_str(input);
                combined
            };

            // Skip empty input
            if full_input.trim().is_empty() {
                continue;
            }

            // Add to history
            self.state.history.push(full_input.clone());

            // Parse and execute command
            let cmd = ReplCommand::parse(&full_input);
            match self.execute_command(cmd) {
                Ok(should_quit) => {
                    if should_quit {
                        break;
                    }
                }
                Err(e) => {
                    self.print_error(&e);
                }
            }
        }

        println!("\nGoodbye!");
        Ok(())
    }

    /// Execute a REPL command
    fn execute_command(&mut self, cmd: ReplCommand) -> Result<bool, X3Error> {
        match cmd {
            ReplCommand::Quit => Ok(true),
            
            ReplCommand::Help => {
                self.print_help();
                Ok(false)
            }
            
            ReplCommand::Clear => {
                self.state.reset();
                println!("State cleared.");
                Ok(false)
            }
            
            ReplCommand::State => {
                self.print_state();
                Ok(false)
            }
            
            ReplCommand::History => {
                self.print_history();
                Ok(false)
            }
            
            ReplCommand::TypeOf(expr) => {
                self.show_type(&expr)?;
                Ok(false)
            }
            
            ReplCommand::Ast(expr) => {
                self.show_ast(&expr)?;
                Ok(false)
            }
            
            ReplCommand::Load(path) => {
                self.load_file(&path)?;
                Ok(false)
            }
            
            ReplCommand::Save(path) => {
                self.save_session(&path)?;
                Ok(false)
            }
            
            ReplCommand::Import(module) => {
                self.import_module(&module)?;
                Ok(false)
            }
            
            ReplCommand::Define(code) => {
                self.define(&code)?;
                Ok(false)
            }
            
            ReplCommand::Eval(expr) => {
                if !expr.is_empty() {
                    self.evaluate(&expr)?;
                }
                Ok(false)
            }
            
            ReplCommand::Set(key, value) => {
                self.set_option(&key, &value)?;
                Ok(false)
            }
            
            ReplCommand::Env => {
                self.print_env();
                Ok(false)
            }
            
            ReplCommand::Gas(expr) => {
                self.analyze_gas(&expr)?;
                Ok(false)
            }
        }
    }

    /// Check if input needs continuation (incomplete)
    fn needs_continuation(&self, input: &str) -> bool {
        let combined = format!("{}{}", self.state.input_buffer, input);
        
        // Count braces
        let open_braces = combined.matches('{').count();
        let close_braces = combined.matches('}').count();
        if open_braces > close_braces {
            return true;
        }

        // Count parentheses
        let open_parens = combined.matches('(').count();
        let close_parens = combined.matches(')').count();
        if open_parens > close_parens {
            return true;
        }

        // Check for trailing operators
        let trimmed = combined.trim();
        if trimmed.ends_with('+') || trimmed.ends_with('-') 
            || trimmed.ends_with('*') || trimmed.ends_with('/') 
            || trimmed.ends_with('|') || trimmed.ends_with('&')
            || trimmed.ends_with(',') || trimmed.ends_with("->") {
            return true;
        }

        false
    }

    /// Evaluate an expression
    fn evaluate(&mut self, expr: &str) -> Result<(), X3Error> {
        // Lex the expression
        let mut lexer = Lexer::new(expr);
        let tokens = lexer.tokenize()?;

        // Parse as expression
        let mut parser = Parser::new(tokens);
        let ast = parser.parse_expression()?;

        // Type check
        let expr_type = self.state.analyzer.infer_type(&ast)?;

        // Print result
        if self.config.show_types {
            println!("  : {}", format_type(&expr_type));
        }

        // Show AST if enabled
        if self.config.show_ast {
            println!("  AST: {:?}", ast);
        }

        Ok(())
    }

    /// Define a binding (let, fn, struct)
    fn define(&mut self, code: &str) -> Result<(), X3Error> {
        let mut lexer = Lexer::new(code);
        let tokens = lexer.tokenize()?;
        let mut parser = Parser::new(tokens);
        let ast = parser.parse_statement()?;

        // Process based on statement type
        match &ast {
            AstNode::Statement(Statement::Let { name, type_ann, value, .. }) => {
                let inferred_type = self.state.analyzer.infer_type(value)?;
                let final_type = type_ann.clone().unwrap_or(inferred_type);
                let value_repr = format!("{:?}", value);
                self.state.variables.insert(name.clone(), (final_type.clone(), value_repr));
                println!("{}: {}", name, format_type(&final_type));
            }
            AstNode::Statement(Statement::FunctionDef { name, .. }) => {
                self.state.functions.insert(name.clone(), ast.clone());
                println!("Defined function: {}", name);
            }
            AstNode::Statement(Statement::StructDef { name, .. }) => {
                self.state.structs.insert(name.clone(), ast.clone());
                println!("Defined struct: {}", name);
            }
            _ => {
                return Err(X3Error::parser("Expected definition statement"));
            }
        }

        Ok(())
    }

    /// Show type of expression
    fn show_type(&mut self, expr: &str) -> Result<(), X3Error> {
        let mut lexer = Lexer::new(expr);
        let tokens = lexer.tokenize()?;
        let mut parser = Parser::new(tokens);
        let ast = parser.parse_expression()?;
        let expr_type = self.state.analyzer.infer_type(&ast)?;
        println!("{}", format_type(&expr_type));
        Ok(())
    }

    /// Show AST of expression
    fn show_ast(&self, expr: &str) -> Result<(), X3Error> {
        let mut lexer = Lexer::new(expr);
        let tokens = lexer.tokenize()?;
        let mut parser = Parser::new(tokens);
        let ast = parser.parse_expression()?;
        println!("{:#?}", ast);
        Ok(())
    }

    /// Load a file into the REPL
    fn load_file(&mut self, path: &PathBuf) -> Result<(), X3Error> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| X3Error::io(&format!("Failed to read file: {}", e)))?;
        
        println!("Loading {}...", path.display());
        
        // Parse the file
        let mut lexer = Lexer::new(&content);
        let tokens = lexer.tokenize()?;
        let mut parser = Parser::new(tokens);
        let module = parser.parse_module()?;

        // Process each item
        for item in module.items {
            match item {
                AstNode::Statement(Statement::FunctionDef { name, .. }) => {
                    self.state.functions.insert(name.clone(), item.clone());
                    println!("  Loaded function: {}", name);
                }
                AstNode::Statement(Statement::StructDef { name, .. }) => {
                    self.state.structs.insert(name.clone(), item.clone());
                    println!("  Loaded struct: {}", name);
                }
                _ => {}
            }
        }

        println!("Loaded {} items from {}", 
            self.state.functions.len() + self.state.structs.len(),
            path.display()
        );
        Ok(())
    }

    /// Save session to file
    fn save_session(&self, path: &PathBuf) -> Result<(), X3Error> {
        let mut content = String::new();
        
        // Add imports
        for import in &self.state.imports {
            content.push_str(&format!("import {};\n", import));
        }
        content.push('\n');

        // Add structs
        for (name, _) in &self.state.structs {
            content.push_str(&format!("// struct {} defined\n", name));
        }

        // Add functions
        for (name, _) in &self.state.functions {
            content.push_str(&format!("// fn {} defined\n", name));
        }

        // Add variables
        for (name, (type_, value)) in &self.state.variables {
            content.push_str(&format!("let {}: {} = {};\n", name, format_type(type_), value));
        }

        std::fs::write(path, content)
            .map_err(|e| X3Error::io(&format!("Failed to write file: {}", e)))?;
        
        println!("Session saved to {}", path.display());
        Ok(())
    }

    /// Import a module
    fn import_module(&mut self, module: &str) -> Result<(), X3Error> {
        // Clean module path
        let module = module.trim_end_matches(';').trim();
        
        // Check for wildcard import
        let (module_path, items) = if module.contains("::*") {
            (module.replace("::*", ""), vec!["*".to_string()])
        } else {
            (module.to_string(), vec![])
        };

        self.state.imports.push(module_path.clone());
        
        if items.contains(&"*".to_string()) {
            println!("Imported all from {}", module_path);
        } else {
            println!("Imported {}", module_path);
        }
        
        Ok(())
    }

    /// Set a REPL option
    fn set_option(&mut self, key: &str, value: &str) -> Result<(), X3Error> {
        match key {
            "show_ast" | "ast" => {
                self.config.show_ast = value.parse().unwrap_or(false);
                println!("show_ast = {}", self.config.show_ast);
            }
            "show_types" | "types" => {
                self.config.show_types = value.parse().unwrap_or(true);
                println!("show_types = {}", self.config.show_types);
            }
            "auto_import" => {
                self.config.auto_import = value.parse().unwrap_or(true);
                println!("auto_import = {}", self.config.auto_import);
            }
            _ => {
                println!("Unknown option: {}", key);
            }
        }
        Ok(())
    }

    /// Analyze gas cost of expression
    fn analyze_gas(&self, expr: &str) -> Result<(), X3Error> {
        let mut lexer = Lexer::new(expr);
        let tokens = lexer.tokenize()?;
        let mut parser = Parser::new(tokens);
        let ast = parser.parse_expression()?;

        // Estimate gas based on AST complexity
        let gas = estimate_gas(&ast);
        
        println!("Estimated gas cost:");
        println!("  EVM:  ~{} gas", gas.evm);
        println!("  SVM:  ~{} compute units", gas.svm);
        
        Ok(())
    }

    /// Load standard library
    fn load_stdlib(&mut self) -> Result<(), X3Error> {
        // Auto-import core modules
        self.state.imports.push("core::*".to_string());
        self.state.imports.push("token::*".to_string());
        self.state.imports.push("safety::*".to_string());
        println!("Loaded stdlib (core, token, safety)");
        Ok(())
    }

    /// Print the REPL banner
    fn print_banner(&self) {
        println!(r#"
╔═══════════════════════════════════════════════════════════════╗
║                    X3 Language REPL v0.1.0                    ║
║              DeFi DSL for Atlas Sphere Blockchain             ║
╠═══════════════════════════════════════════════════════════════╣
║  Type :help for commands, :quit to exit                       ║
║  Multi-line input: unclosed braces continue on next line      ║
╚═══════════════════════════════════════════════════════════════╝
"#);
    }

    /// Print help message
    fn print_help(&self) {
        println!(r#"
X3 REPL Commands:
═════════════════════════════════════════════════════════════════

Expression Evaluation:
  <expr>              Evaluate an X3 expression and show type
  let x = <expr>      Define a variable binding
  fn name() {{ ... }}  Define a function
  struct Name {{ }}    Define a struct

Meta Commands (prefix with :):
  :help, :h, :?       Show this help message
  :quit, :q, :exit    Exit the REPL
  :clear, :reset      Clear all definitions
  :state, :s          Show current REPL state
  :history, :hist     Show command history

Type & Analysis:
  :type <expr>        Show the type of an expression
  :ast <expr>         Show the AST of an expression
  :gas <expr>         Estimate gas/compute cost

File Operations:
  :load <path>        Load and evaluate an X3 file
  :save <path>        Save current session to file
  :import <module>    Import a module

Settings:
  :set key=value      Set REPL option
    show_ast=bool     Show AST for expressions
    show_types=bool   Show types for expressions
    auto_import=bool  Auto-import common modules

  :env                Show environment info

Examples:
  let x = 100u128
  safe_add_u128(x, 200u128)
  :type balance_of(token, user)
  :gas swap(dex, amount, min_out)
"#);
    }

    /// Print current state
    fn print_state(&self) {
        println!("\nREPL State:");
        println!("═══════════════════════════════════════");
        
        println!("\nImports ({}):", self.state.imports.len());
        for import in &self.state.imports {
            println!("  import {}", import);
        }

        println!("\nVariables ({}):", self.state.variables.len());
        for (name, (type_, _)) in &self.state.variables {
            println!("  {}: {}", name, format_type(type_));
        }

        println!("\nFunctions ({}):", self.state.functions.len());
        for name in self.state.functions.keys() {
            println!("  fn {}()", name);
        }

        println!("\nStructs ({}):", self.state.structs.len());
        for name in self.state.structs.keys() {
            println!("  struct {}", name);
        }
        println!();
    }

    /// Print command history
    fn print_history(&self) {
        println!("\nCommand History:");
        println!("═══════════════════════════════════════");
        for (i, cmd) in self.state.history.iter().enumerate() {
            println!("{:4}: {}", i + 1, cmd);
        }
        println!();
    }

    /// Print environment info
    fn print_env(&self) {
        println!("\nEnvironment:");
        println!("═══════════════════════════════════════");
        println!("  X3 Version:     0.1.0");
        println!("  Target VMs:     EVM, SVM");
        println!("  Stdlib Loaded:  {}", self.config.load_stdlib);
        println!("  Show AST:       {}", self.config.show_ast);
        println!("  Show Types:     {}", self.config.show_types);
        println!("  Auto Import:    {}", self.config.auto_import);
        if let Some(ref hist) = self.config.history_file {
            println!("  History File:   {}", hist.display());
        }
        println!();
    }

    /// Print an error
    fn print_error(&self, err: &X3Error) {
        eprintln!("\x1b[31mError:\x1b[0m {}", err);
    }
}

/// Gas estimation result
struct GasEstimate {
    evm: u64,
    svm: u64,
}

/// Estimate gas cost for an AST node
fn estimate_gas(node: &AstNode) -> GasEstimate {
    let mut evm = 0u64;
    let mut svm = 0u64;

    match node {
        AstNode::Expression(expr) => {
            match expr {
                Expression::Literal(_) => {
                    evm += 3; // PUSH
                    svm += 100;
                }
                Expression::BinaryOp { left, right, .. } => {
                    let left_gas = estimate_gas(&AstNode::Expression(*left.clone()));
                    let right_gas = estimate_gas(&AstNode::Expression(*right.clone()));
                    evm += left_gas.evm + right_gas.evm + 5; // ADD/MUL/etc
                    svm += left_gas.svm + right_gas.svm + 100;
                }
                Expression::FunctionCall { args, .. } => {
                    evm += 2100; // CALL base
                    svm += 5000;
                    for arg in args {
                        let arg_gas = estimate_gas(&AstNode::Expression(arg.clone()));
                        evm += arg_gas.evm;
                        svm += arg_gas.svm;
                    }
                }
                Expression::FieldAccess { object, .. } => {
                    let obj_gas = estimate_gas(&AstNode::Expression(*object.clone()));
                    evm += obj_gas.evm + 200; // SLOAD
                    svm += obj_gas.svm + 200;
                }
                _ => {
                    evm += 10;
                    svm += 100;
                }
            }
        }
        _ => {
            evm += 21000; // Base tx cost
            svm += 5000;
        }
    }

    GasEstimate { evm, svm }
}

/// Format a type for display
fn format_type(t: &Type) -> String {
    match t {
        Type::U8 => "u8".to_string(),
        Type::U16 => "u16".to_string(),
        Type::U32 => "u32".to_string(),
        Type::U64 => "u64".to_string(),
        Type::U128 => "u128".to_string(),
        Type::U256 => "u256".to_string(),
        Type::Bool => "bool".to_string(),
        Type::Address => "Address".to_string(),
        Type::Bytes => "bytes".to_string(),
        Type::Bytes32 => "bytes32".to_string(),
        Type::String => "String".to_string(),
        Type::Array(inner) => format!("[{}]", format_type(inner)),
        Type::Option(inner) => format!("Option<{}>", format_type(inner)),
        Type::Result(ok, err) => format!("Result<{}, {}>", format_type(ok), format_type(err)),
        Type::Tuple(types) => {
            let inner: Vec<_> = types.iter().map(format_type).collect();
            format!("({})", inner.join(", "))
        }
        Type::Named(name) => name.clone(),
        Type::Function { params, ret } => {
            let params: Vec<_> = params.iter().map(format_type).collect();
            format!("fn({}) -> {}", params.join(", "), format_type(ret))
        }
        Type::Unit => "()".to_string(),
        Type::Never => "!".to_string(),
        Type::Unknown => "?".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_command_parsing() {
        assert!(matches!(ReplCommand::parse(":help"), ReplCommand::Help));
        assert!(matches!(ReplCommand::parse(":quit"), ReplCommand::Quit));
        assert!(matches!(ReplCommand::parse(":q"), ReplCommand::Quit));
        assert!(matches!(ReplCommand::parse("let x = 5"), ReplCommand::Define(_)));
        assert!(matches!(ReplCommand::parse("fn test() {}"), ReplCommand::Define(_)));
        assert!(matches!(ReplCommand::parse("1 + 2"), ReplCommand::Eval(_)));
    }

    #[test]
    fn test_needs_continuation() {
        let repl = Repl::new(ReplConfig::default());
        
        // Unclosed brace should continue
        assert!(repl.needs_continuation("fn test() {"));
        
        // Complete expression should not continue
        assert!(!repl.needs_continuation("let x = 5"));
        
        // Trailing operator should continue
        assert!(repl.needs_continuation("1 +"));
    }

    #[test]
    fn test_format_type() {
        assert_eq!(format_type(&Type::U128), "u128");
        assert_eq!(format_type(&Type::Bool), "bool");
        assert_eq!(format_type(&Type::Array(Box::new(Type::U8))), "[u8]");
        assert_eq!(format_type(&Type::Option(Box::new(Type::Address))), "Option<Address>");
    }
}
