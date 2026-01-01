//! x3 repl - Interactive X3 REPL (Read-Eval-Print Loop)
//!
//! Provides an interactive shell for experimenting with X3 code.

use crate::error::Result;
use clap::Args;
use colored::Colorize;
use std::io::{self, BufRead, Write};
use x3_compiler::{CompilationOptions, Compiler};
use x3_opt::OptLevel;
use x3_vm::{Value, VM};

/// Arguments for the repl command
#[derive(Args, Debug)]
pub struct ReplArgs {
    /// Enable verbose output (show MIR, bytecode)
    #[arg(short, long)]
    pub verbose: bool,

    /// Optimization level (0-3)
    #[arg(short = 'O', long, default_value = "1")]
    pub opt_level: u8,

    /// Show gas costs after execution
    #[arg(long)]
    pub gas: bool,

    /// Maximum gas limit for execution
    #[arg(long, default_value = "1000000")]
    pub gas_limit: u64,
}

/// REPL state
struct ReplState {
    /// History of entered code
    history: Vec<String>,
    /// Accumulated module code (functions, structs, etc.)
    module_code: String,
    /// Line number counter
    line_number: usize,
    /// Verbose mode
    verbose: bool,
    /// Optimization level
    opt_level: u8,
    /// Show gas
    show_gas: bool,
    /// Gas limit
    gas_limit: u64,
}

impl ReplState {
    fn new(args: &ReplArgs) -> Self {
        Self {
            history: Vec::new(),
            module_code: String::new(),
            line_number: 1,
            verbose: args.verbose,
            opt_level: args.opt_level,
            show_gas: args.gas,
            gas_limit: args.gas_limit,
        }
    }

    /// Wrap expression in a main function for evaluation
    fn wrap_expression(&self, expr: &str) -> String {
        format!(
            r#"{}

fn __repl_main__() -> i64 {{
    {}
}}
"#,
            self.module_code, expr
        )
    }

    /// Wrap statement(s) in a main function
    fn wrap_statements(&self, stmts: &str) -> String {
        format!(
            r#"{}

fn __repl_main__() {{
    {}
}}
"#,
            self.module_code, stmts
        )
    }
}

/// Execute the REPL
pub async fn execute(args: ReplArgs) -> Result<()> {
    print_banner();

    let mut state = ReplState::new(&args);
    let stdin = io::stdin();
    let mut stdout = io::stdout();

    loop {
        // Print prompt
        print!("{} ", format!("x3[{}]>", state.line_number).cyan());
        stdout.flush()?;

        // Read line
        let mut line = String::new();
        if stdin.lock().read_line(&mut line)? == 0 {
            // EOF
            println!("\n{}", "Goodbye!".green());
            break;
        }

        let line = line.trim();

        // Skip empty lines
        if line.is_empty() {
            continue;
        }

        // Handle special commands
        if line.starts_with(':') {
            if handle_command(&mut state, line) {
                break;
            }
            state.line_number += 1;
            continue;
        }

        // Handle multi-line input (ends with \)
        let mut full_input = line.to_string();
        while full_input.ends_with('\\') {
            full_input.pop(); // Remove backslash
            print!("{} ", "...>".cyan());
            stdout.flush()?;
            let mut continuation = String::new();
            stdin.lock().read_line(&mut continuation)?;
            full_input.push_str(continuation.trim());
        }

        // Process input
        process_input(&mut state, &full_input);
        state.history.push(full_input);
        state.line_number += 1;
    }

    Ok(())
}

fn print_banner() {
    println!(
        r#"
{}
{}
{}

Type {} for help, {} to quit.
"#,
        "╔═══════════════════════════════════════════╗".bright_blue(),
        "║     X3 REPL - Atlas Sphere Interactive    ║".bright_blue(),
        "╚═══════════════════════════════════════════╝".bright_blue(),
        ":help".yellow(),
        ":quit".yellow()
    );
}

fn handle_command(state: &mut ReplState, cmd: &str) -> bool {
    let parts: Vec<&str> = cmd.splitn(2, ' ').collect();
    let command = parts[0];
    let arg = parts.get(1).map(|s| s.trim());

    match command {
        ":quit" | ":q" | ":exit" => {
            println!("{}", "Goodbye!".green());
            return true;
        }
        ":help" | ":h" | ":?" => {
            print_help();
        }
        ":clear" | ":c" => {
            state.module_code.clear();
            state.history.clear();
            state.line_number = 1;
            println!("{}", "State cleared.".green());
        }
        ":history" | ":hist" => {
            println!("{}", "History:".yellow().bold());
            for (i, entry) in state.history.iter().enumerate() {
                println!("  {}: {}", i + 1, entry);
            }
        }
        ":verbose" | ":v" => {
            state.verbose = !state.verbose;
            println!(
                "Verbose mode: {}",
                if state.verbose {
                    "ON".green()
                } else {
                    "OFF".red()
                }
            );
        }
        ":gas" => {
            state.show_gas = !state.show_gas;
            println!(
                "Gas display: {}",
                if state.show_gas {
                    "ON".green()
                } else {
                    "OFF".red()
                }
            );
        }
        ":opt" => {
            if let Some(level) = arg {
                if let Ok(l) = level.parse::<u8>() {
                    if l <= 3 {
                        state.opt_level = l;
                        println!("Optimization level set to: {}", l.to_string().green());
                    } else {
                        println!("{}", "Error: optimization level must be 0-3".red());
                    }
                } else {
                    println!("{}", "Error: invalid optimization level".red());
                }
            } else {
                println!("Current optimization level: {}", state.opt_level);
            }
        }
        ":type" | ":t" => {
            if let Some(expr) = arg {
                check_type(state, expr);
            } else {
                println!("{}", "Usage: :type <expression>".yellow());
            }
        }
        ":def" | ":define" => {
            if let Some(code) = arg {
                // Add to module definitions
                state.module_code.push_str(code);
                state.module_code.push('\n');
                println!("{}", "Definition added.".green());
            } else {
                println!("{}", "Usage: :def <function or type definition>".yellow());
            }
        }
        ":show" => {
            println!("{}", "Current definitions:".yellow().bold());
            if state.module_code.is_empty() {
                println!("  (none)");
            } else {
                for line in state.module_code.lines() {
                    println!("  {}", line);
                }
            }
        }
        ":mir" => {
            if let Some(expr) = arg {
                show_mir(state, expr);
            } else {
                println!("{}", "Usage: :mir <expression>".yellow());
            }
        }
        ":bc" | ":bytecode" => {
            if let Some(expr) = arg {
                show_bytecode(state, expr);
            } else {
                println!("{}", "Usage: :bc <expression>".yellow());
            }
        }
        ":load" => {
            if let Some(path) = arg {
                load_file(state, path);
            } else {
                println!("{}", "Usage: :load <file.x3>".yellow());
            }
        }
        _ => {
            println!("{} Unknown command: {}", "Error:".red(), command.yellow());
            println!("Type {} for available commands.", ":help".yellow());
        }
    }

    false
}

fn print_help() {
    println!(
        r#"
{}

{}
  <expression>        Evaluate an expression
  <statement>;        Execute a statement
  let x = <expr>;     Bind a value to a variable

{}
  :help, :h, :?       Show this help message
  :quit, :q, :exit    Exit the REPL
  :clear, :c          Clear all definitions and history
  :history, :hist     Show command history

{}
  :def <code>         Add a function/type definition
  :show               Show current definitions
  :type <expr>        Show the type of an expression
  :load <file>        Load definitions from a file

{}
  :mir <expr>         Show MIR for an expression
  :bc <expr>          Show bytecode for an expression
  :verbose, :v        Toggle verbose output
  :gas                Toggle gas cost display
  :opt [0-3]          Get/set optimization level

{}
  x3[1]> 2 + 2
  => 4

  x3[2]> :def fn square(x: i64) -> i64 {{ x * x }}
  Definition added.

  x3[3]> square(5)
  => 25

  x3[4]> let fib = |n| if n < 2 {{ n }} else {{ fib(n-1) + fib(n-2) }};
  x3[5]> fib(10)
  => 55
"#,
        "X3 REPL Help".yellow().bold(),
        "Expressions:".cyan().bold(),
        "Commands:".cyan().bold(),
        "Definitions:".cyan().bold(),
        "Debug:".cyan().bold(),
        "Examples:".cyan().bold()
    );
}

fn process_input(state: &mut ReplState, input: &str) {
    // Determine if this is an expression or statement
    let is_expression = !input.ends_with(';')
        && !input.starts_with("let ")
        && !input.starts_with("fn ")
        && !input.starts_with("struct ")
        && !input.starts_with("if ")
        && !input.starts_with("while ")
        && !input.starts_with("for ");

    let source = if is_expression {
        state.wrap_expression(input)
    } else {
        state.wrap_statements(input)
    };

    if state.verbose {
        println!("{}", "Generated source:".dimmed());
        for line in source.lines() {
            println!("  {}", line.dimmed());
        }
    }

    // Compile
    match compile_and_run(state, &source, is_expression) {
        Ok(result) => {
            if let Some(value) = result.value {
                println!("{} {}", "=>".green(), format_value(&value));
            }
            if state.show_gas {
                println!(
                    "{}",
                    format!(
                        "   [gas: {}, instructions: {}]",
                        result.gas_used, result.instruction_count
                    )
                    .dimmed()
                );
            }
        }
        Err(e) => {
            println!("{} {}", "Error:".red().bold(), e);
        }
    }
}

struct ExecResult {
    value: Option<String>,
    gas_used: u64,
    instruction_count: u64,
}

fn opt_level_from_u8(level: u8) -> OptLevel {
    match level {
        0 => OptLevel::None,
        1 => OptLevel::Basic,
        2 => OptLevel::Default,
        _ => OptLevel::Aggressive,
    }
}

fn compile_and_run(
    state: &ReplState,
    source: &str,
    _is_expression: bool,
) -> std::result::Result<ExecResult, String> {
    // Build compilation options
    let options = CompilationOptions {
        opt_level: opt_level_from_u8(state.opt_level),
        verbose: state.verbose,
        debug: true,
        emit_hir: false,
        emit_mir: state.verbose,
        emit_mir_opt: state.verbose,
        emit_stats: state.verbose,
        emit_format: x3_compiler::options::EmitFormat::Bytecode,
        analyze_gas: state.show_gas,
        verify_contract: false,
    };

    // Compile source to bytecode
    let compile_result = Compiler::compile(source, options);

    match compile_result {
        Ok(output) => {
            // Serialize bytecode to bytes
            let bytes = output.bytecode.to_bytes();

            // Create VM and execute
            let mut vm = VM::from_bytes(&bytes).map_err(|e| format!("VM load error: {:?}", e))?;

            // Call __repl_main__
            let result = vm
                .call_function_by_name("__repl_main__", &[])
                .map_err(|e| format!("Execution error: {:?}", e))?;

            Ok(ExecResult {
                value: result.value.map(|v| format_value_internal(&v)),
                gas_used: result.gas_used,
                instruction_count: result.instruction_count,
            })
        }
        Err(e) => Err(format!("Compilation failed: {:?}", e)),
    }
}

fn format_value_internal(value: &Value) -> String {
    match value {
        Value::I64(n) => n.to_string(),
        Value::F64(f) => format!("{:.6}", f),
        Value::Bool(b) => b.to_string(),
        Value::String(s) => format!("\"{}\"", s),
        Value::Bytes(b) => format!("0x{}", hex::encode(b)),
        Value::Addr(a) => format!("@{:#x}", a),
        Value::Unit => "()".to_string(),
    }
}

fn format_value(value: &str) -> String {
    value.bright_green().to_string()
}

fn check_type(state: &ReplState, expr: &str) {
    let source = state.wrap_expression(expr);

    // Parse and get basic type info
    match x3_parser::parse_program(&source) {
        Ok(_ast) => {
            // For now, just indicate successful parsing
            // Full type inference would require integrating typeck
            println!("{} expression parses successfully", "Type check:".cyan());
            println!("  (Full type inference not yet available in REPL)");
        }
        Err(e) => {
            println!("{} {:?}", "Parse error:".red(), e);
        }
    }
}

fn show_mir(state: &ReplState, expr: &str) {
    let source = state.wrap_expression(expr);

    // Compile with MIR output enabled
    let options = CompilationOptions {
        opt_level: opt_level_from_u8(state.opt_level),
        verbose: false,
        debug: true,
        emit_hir: false,
        emit_mir: true,
        emit_mir_opt: true,
        emit_stats: false,
        emit_format: x3_compiler::options::EmitFormat::Mir,
        analyze_gas: false,
        verify_contract: false,
    };

    match Compiler::compile(&source, options) {
        Ok(output) => {
            println!("{}", "MIR:".yellow().bold());
            if let Some(artifacts) = output.artifacts {
                if let Some(mir) = artifacts.mir_optimized {
                    // Print MIR functions
                    for func in &mir.functions {
                        println!("  fn {}:", func.symbol);
                        for (i, block) in func.blocks.iter().enumerate() {
                            println!("    bb{}: ({} statements)", i, block.statements.len());
                        }
                    }
                } else {
                    println!("  (MIR not available in artifacts)");
                }
            } else {
                println!("  (No artifacts returned)");
            }
        }
        Err(e) => {
            println!("{} {:?}", "Error:".red(), e);
        }
    }
}

fn show_bytecode(state: &ReplState, expr: &str) {
    let source = state.wrap_expression(expr);

    let options = CompilationOptions {
        opt_level: opt_level_from_u8(state.opt_level),
        verbose: false,
        debug: true,
        emit_hir: false,
        emit_mir: false,
        emit_mir_opt: false,
        emit_stats: false,
        emit_format: x3_compiler::options::EmitFormat::Bytecode,
        analyze_gas: false,
        verify_contract: false,
    };

    match Compiler::compile(&source, options) {
        Ok(output) => {
            let bytecode = output.bytecode.to_bytes();
            println!("{}", "Bytecode:".yellow().bold());
            println!("  Size: {} bytes", bytecode.len());
            println!("  Functions: {}", output.bytecode.functions.len());
            println!("  Code: {} bytes", output.bytecode.code.len());
            // Hexdump first 256 bytes
            for (i, chunk) in bytecode.chunks(16).take(16).enumerate() {
                print!("  {:04x}: ", i * 16);
                for b in chunk {
                    print!("{:02x} ", b);
                }
                println!();
            }
            if bytecode.len() > 256 {
                println!("  ... ({} more bytes)", bytecode.len() - 256);
            }
        }
        Err(e) => {
            println!("{} {:?}", "Compilation failed:".red(), e);
        }
    }
}

fn load_file(state: &mut ReplState, path: &str) {
    match std::fs::read_to_string(path) {
        Ok(content) => {
            state.module_code.push_str(&content);
            state.module_code.push('\n');
            println!("{} Loaded {}", "OK:".green(), path);
        }
        Err(e) => {
            println!("{} Failed to load {}: {}", "Error:".red(), path, e);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_repl_state_wrap_expression() {
        let args = ReplArgs {
            verbose: false,
            opt_level: 1,
            gas: false,
            gas_limit: 1000000,
        };
        let state = ReplState::new(&args);
        let wrapped = state.wrap_expression("2 + 2");
        assert!(wrapped.contains("__repl_main__"));
        assert!(wrapped.contains("2 + 2"));
    }
}
