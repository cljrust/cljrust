/// Interactive REPL: compile & execute cljrust expressions on the fly
///
/// Maintains accumulated state (use, def, defn, struct, enum, trait, impl)
/// across inputs so the user can build up a program incrementally.

use crate::ast::TopLevel;
use crate::codegen::CodeGen;
use crate::lexer::Lexer;
use crate::parser::Parser;
use std::env;
use std::fs;
use std::io::{self, Write};
use std::process::Command;

/// ANSI color helpers — only used when stdout is a terminal
struct Color;
impl Color {
    fn green(s: &str) -> String { format!("\x1b[32m{}\x1b[0m", s) }
    fn cyan(s: &str) -> String { format!("\x1b[36m{}\x1b[0m", s) }
    fn yellow(s: &str) -> String { format!("\x1b[33m{}\x1b[0m", s) }
    fn red(s: &str) -> String { format!("\x1b[31m{}\x1b[0m", s) }
    fn dim(s: &str) -> String { format!("\x1b[2m{}\x1b[0m", s) }
    fn bold(s: &str) -> String { format!("\x1b[1m{}\x1b[0m", s) }
}

/// Accumulated REPL state — everything needed to reconstruct
/// a compilable Rust program from incremental inputs.
struct ReplState {
    /// `use` declarations
    uses: Vec<String>,
    /// Top-level items (struct, enum, trait, impl, defn, const, static)
    items: Vec<String>,
    /// Counter for expression wrapping
    expr_id: usize,
    /// History of successfully evaluated inputs
    history: Vec<String>,
    /// Show generated Rust before executing
    show_rust: bool,
}

impl ReplState {
    fn new() -> Self {
        ReplState {
            uses: vec![
                // Commonly needed in REPL
                "use std::collections::HashMap;".to_string(),
                "use std::collections::HashSet;".to_string(),
            ],
            items: Vec::new(),
            expr_id: 0,
            history: Vec::new(),
            show_rust: false,
        }
    }

    /// Build a complete Rust program that:
    /// 1. Includes all accumulated uses
    /// 2. Includes all accumulated definitions
    /// 3. Wraps the given expression(s) in main()
    fn build_program(&self, main_body: &str) -> String {
        let mut prog = String::new();
        prog.push_str("#![allow(unused_imports, unused_variables, unused_mut, dead_code, unused_parens)]\n\n");
        for u in &self.uses {
            prog.push_str(u);
            prog.push('\n');
        }
        prog.push('\n');
        for item in &self.items {
            prog.push_str(item);
            prog.push('\n');
        }
        prog.push_str("\nfn main() {\n");
        for line in main_body.lines() {
            prog.push_str("    ");
            prog.push_str(line);
            prog.push('\n');
        }
        prog.push_str("}\n");
        prog
    }

    /// Build a program where the expression result is printed via Debug
    fn build_print_program(&self, expr_rust: &str) -> String {
        let main_body = format!(
            "let __result = {{\n    {}\n}};\nprintln!(\"{{:?}}\", __result);",
            expr_rust.trim().trim_end_matches(';')
        );
        self.build_program(&main_body)
    }

    /// Build a program that just runs statements (for side-effect expressions)
    fn build_exec_program(&self, body_rust: &str) -> String {
        self.build_program(body_rust)
    }
}

/// Compile and run a Rust program string, returning (stdout, stderr, success)
fn compile_and_run(rust_code: &str) -> (String, String, bool) {
    let tmp_dir = env::temp_dir();
    let rs_path = tmp_dir.join("__cljrust_repl.rs");
    let bin_path = tmp_dir.join("__cljrust_repl");

    if fs::write(&rs_path, rust_code).is_err() {
        return (String::new(), "Failed to write temp file".to_string(), false);
    }

    // Compile
    let compile_out = Command::new("rustc")
        .args(&[
            rs_path.to_str().unwrap(),
            "-o",
            bin_path.to_str().unwrap(),
            "--edition",
            "2021",
        ])
        .output();

    let compile_out = match compile_out {
        Ok(o) => o,
        Err(e) => {
            let _ = fs::remove_file(&rs_path);
            return (String::new(), format!("Failed to run rustc: {}", e), false);
        }
    };

    let _ = fs::remove_file(&rs_path);

    if !compile_out.status.success() {
        let stderr = String::from_utf8_lossy(&compile_out.stderr).to_string();
        return (String::new(), stderr, false);
    }

    // Run
    let run_out = Command::new(bin_path.to_str().unwrap()).output();
    let _ = fs::remove_file(&bin_path);

    match run_out {
        Ok(o) => {
            let stdout = String::from_utf8_lossy(&o.stdout).to_string();
            let stderr = String::from_utf8_lossy(&o.stderr).to_string();
            (stdout, stderr, o.status.success())
        }
        Err(e) => (String::new(), format!("Failed to run: {}", e), false),
    }
}

/// Parse a cljrust input string into AST top-level items
fn parse_input(input: &str) -> Result<Vec<TopLevel>, String> {
    let mut lexer = Lexer::new(input);
    let tokens = lexer.tokenize()?;
    let mut parser = Parser::new(tokens);
    let program = parser.parse_program()?;
    Ok(program.items)
}

/// Generate Rust code for a single top-level item
fn gen_top_level(item: &TopLevel) -> String {
    let prog = crate::ast::Program {
        items: vec![item.clone()],
    };
    let mut cg = CodeGen::new();
    cg.generate(&prog)
}

/// Check if parens/brackets/braces are balanced
fn is_balanced(input: &str) -> bool {
    let mut depth = 0i32;
    let mut in_string = false;
    let mut escape = false;

    for ch in input.chars() {
        if escape {
            escape = false;
            continue;
        }
        if ch == '\\' && in_string {
            escape = true;
            continue;
        }
        if ch == '"' {
            in_string = !in_string;
            continue;
        }
        if in_string {
            continue;
        }
        match ch {
            '(' | '[' | '{' => depth += 1,
            ')' | ']' | '}' => depth -= 1,
            ';' => break, // rest is comment
            _ => {}
        }
    }
    depth <= 0
}

/// Classify a top-level item for the REPL
enum InputKind {
    /// use declaration — accumulate
    Use,
    /// Definition (defn, struct, enum, trait, impl, def, const) — accumulate
    Definition,
    /// Expression — evaluate and print result
    Expression,
}

fn classify(item: &TopLevel) -> InputKind {
    match item {
        TopLevel::Use(_) | TopLevel::ExternCrate(_) => InputKind::Use,
        TopLevel::Defn(_)
        | TopLevel::DefStruct(_)
        | TopLevel::DefEnum(_)
        | TopLevel::DefTrait(_)
        | TopLevel::Impl(_)
        | TopLevel::Def(_)
        | TopLevel::Mod(_)
        | TopLevel::Attr(_, _) => InputKind::Definition,
        TopLevel::Expr(_) => InputKind::Expression,
    }
}

/// Is this expression likely a statement (side-effect only, no meaningful return)?
fn is_statement_expr(item: &TopLevel) -> bool {
    match item {
        TopLevel::Expr(e) => matches!(
            e,
            crate::ast::Expr::MacroCall { name, .. } if name == "println!" || name == "print!" || name == "eprintln!" || name == "eprint!"
        ) || matches!(e, crate::ast::Expr::For { .. })
          || matches!(e, crate::ast::Expr::While { .. })
          || matches!(e, crate::ast::Expr::Set { .. }),
        _ => false,
    }
}

fn print_banner() {
    println!("{}", Color::bold("cljrust REPL v0.1.0"));
    println!("{}", Color::dim("Clojure syntax → Rust | Type expressions to evaluate"));
    println!("{}", Color::dim("Multi-line: keep typing until parens are balanced"));
    println!();
    println!("  {}  show this help       {}  show generated Rust (toggle)",
             Color::cyan(":help"), Color::cyan(":rust"));
    println!("  {}  clear definitions    {}  show all definitions",
             Color::cyan(":clear"), Color::cyan(":defs"));
    println!("  {} load a .cljr file     {}  exit",
             Color::cyan(":load <f>"), Color::cyan(":quit"));
    println!();
}

fn print_help() {
    println!("{}", Color::bold("REPL Commands:"));
    println!("  {}        Show this help", Color::cyan(":help"));
    println!("  {}        Toggle showing generated Rust code", Color::cyan(":rust"));
    println!("  {}       Clear all accumulated definitions", Color::cyan(":clear"));
    println!("  {}        Show all accumulated definitions", Color::cyan(":defs"));
    println!("  {}       Show input history", Color::cyan(":history"));
    println!("  {} <file>  Load and evaluate a .cljr file", Color::cyan(":load"));
    println!("  {}     Show the full generated Rust for last eval", Color::cyan(":last-rs"));
    println!("  {}        Exit the REPL", Color::cyan(":quit"));
    println!();
    println!("{}", Color::bold("Tips:"));
    println!("  - Definitions (defn, defstruct, etc.) are accumulated");
    println!("  - Expressions are compiled, run, and the result is printed");
    println!("  - Multi-line input: just keep typing until parens balance");
    println!("  - std::collections::HashMap and HashSet are pre-imported");
    println!();
    println!("{}", Color::bold("Examples:"));
    println!("  {}  (+ 1 2 3)", Color::dim("cljr>"));
    println!("  {}  6", Color::green("=>"));
    println!();
    println!("  {}  (defn square [x : i32] -> i32 (* x x))", Color::dim("cljr>"));
    println!("  {}  defined: square", Color::green("=>"));
    println!();
    println!("  {}  (square 7)", Color::dim("cljr>"));
    println!("  {}  49", Color::green("=>"));
    println!();
}

/// Main REPL loop
pub fn run() {
    print_banner();

    let stdin = io::stdin();
    let mut state = ReplState::new();
    let mut input_buf = String::new();
    let mut continuation = false;
    let mut last_rust = String::new();

    loop {
        // Prompt
        if continuation {
            eprint!("{} ", Color::dim("  ..."));
        } else {
            eprint!("{} ", Color::cyan("cljr>"));
        }
        let _ = io::stderr().flush();

        let mut line = String::new();
        match stdin.read_line(&mut line) {
            Ok(0) => break, // EOF
            Ok(_) => {}
            Err(_) => break,
        }

        if !continuation {
            input_buf.clear();
        }
        input_buf.push_str(&line);

        let trimmed = input_buf.trim();

        // Empty line
        if trimmed.is_empty() {
            if continuation {
                // Cancel multi-line
                continuation = false;
                input_buf.clear();
                eprintln!("{}", Color::dim("  (cancelled)"));
            }
            continue;
        }

        // REPL commands (only on first line)
        if !continuation && trimmed.starts_with(':') {
            let parts: Vec<&str> = trimmed.splitn(2, ' ').collect();
            match parts[0] {
                ":quit" | ":q" | ":exit" => {
                    println!("{}", Color::dim("Goodbye!"));
                    break;
                }
                ":help" | ":h" | ":?" => {
                    print_help();
                }
                ":rust" => {
                    state.show_rust = !state.show_rust;
                    println!(
                        "  {} Show Rust: {}",
                        Color::dim("=>"),
                        if state.show_rust {
                            Color::green("ON")
                        } else {
                            Color::dim("OFF")
                        }
                    );
                }
                ":clear" => {
                    state.uses = vec![
                        "use std::collections::HashMap;".to_string(),
                        "use std::collections::HashSet;".to_string(),
                    ];
                    state.items.clear();
                    state.expr_id = 0;
                    println!("  {} {}", Color::dim("=>"), Color::yellow("State cleared"));
                }
                ":defs" => {
                    if state.items.is_empty() {
                        println!("  {} {}", Color::dim("=>"), Color::dim("(no definitions)"));
                    } else {
                        println!("  {} {}", Color::dim("=>"), Color::bold("Accumulated definitions:"));
                        for u in &state.uses {
                            println!("    {}", Color::dim(u));
                        }
                        for item in &state.items {
                            for line in item.lines() {
                                println!("    {}", line);
                            }
                        }
                    }
                }
                ":history" => {
                    if state.history.is_empty() {
                        println!("  {} {}", Color::dim("=>"), Color::dim("(no history)"));
                    } else {
                        for (i, h) in state.history.iter().enumerate() {
                            println!("  {} {}", Color::dim(&format!("[{}]", i + 1)), h);
                        }
                    }
                }
                ":load" => {
                    if parts.len() < 2 {
                        eprintln!("  {} Usage: :load <file.cljr>", Color::red("!"));
                    } else {
                        let path = parts[1].trim();
                        match fs::read_to_string(path) {
                            Ok(source) => {
                                println!("  {} Loading {}...", Color::dim("=>"), path);
                                eval_input(&source, &mut state, &mut last_rust);
                            }
                            Err(e) => {
                                eprintln!("  {} Cannot read '{}': {}", Color::red("!"), path, e);
                            }
                        }
                    }
                }
                ":last-rs" | ":last" => {
                    if last_rust.is_empty() {
                        println!("  {} {}", Color::dim("=>"), Color::dim("(no previous Rust output)"));
                    } else {
                        println!("{}", Color::dim("--- Generated Rust ---"));
                        for (i, line) in last_rust.lines().enumerate() {
                            println!("{} {}", Color::dim(&format!("{:3}|", i + 1)), line);
                        }
                        println!("{}", Color::dim("--- End ---"));
                    }
                }
                other => {
                    eprintln!("  {} Unknown command: {}  (try :help)", Color::red("!"), other);
                }
            }
            input_buf.clear();
            continue;
        }

        // Check if parens are balanced for multi-line support
        if !is_balanced(trimmed) {
            continuation = true;
            continue;
        }
        continuation = false;

        let input = input_buf.trim().to_string();
        eval_input(&input, &mut state, &mut last_rust);
        input_buf.clear();
    }
}

fn eval_input(input: &str, state: &mut ReplState, last_rust: &mut String) {
    // Parse
    let items = match parse_input(input) {
        Ok(items) => items,
        Err(e) => {
            eprintln!("  {} {}", Color::red("! Parse error:"), e);
            return;
        }
    };

    if items.is_empty() {
        return;
    }

    // Process each item
    let mut exprs_to_eval = Vec::new();
    let mut had_defs = false;

    for item in &items {
        match classify(item) {
            InputKind::Use => {
                let rust = gen_top_level(item).trim().to_string();
                if !state.uses.contains(&rust) {
                    state.uses.push(rust.clone());
                    println!("  {} {}", Color::green("=>"), Color::dim(&format!("added: {}", rust)));
                } else {
                    println!("  {} {}", Color::dim("=>"), Color::dim("(already imported)"));
                }
                had_defs = true;
            }
            InputKind::Definition => {
                let rust = gen_top_level(item).trim().to_string();
                // Extract a name for display
                let name = def_name(item);
                state.items.push(rust.clone());
                if state.show_rust {
                    println!("{}", Color::dim("--- Rust ---"));
                    for line in rust.lines() {
                        println!("  {}", Color::dim(line));
                    }
                    println!("{}", Color::dim("---"));
                }
                println!(
                    "  {} {}",
                    Color::green("=>"),
                    Color::green(&format!("defined: {}", name))
                );
                had_defs = true;
            }
            InputKind::Expression => {
                exprs_to_eval.push(item.clone());
            }
        }
    }

    // Evaluate expression(s)
    if !exprs_to_eval.is_empty() {
        // Check if ALL expressions are side-effect statements
        let all_statements = exprs_to_eval.iter().all(|e| is_statement_expr(e));

        // Generate Rust for expressions
        let mut expr_rust = String::new();
        for item in &exprs_to_eval {
            let code = gen_top_level(item);
            expr_rust.push_str(code.trim());
            expr_rust.push('\n');
        }

        let full_rust = if all_statements {
            state.build_exec_program(expr_rust.trim())
        } else if exprs_to_eval.len() == 1 && !all_statements {
            state.build_print_program(expr_rust.trim())
        } else {
            // Multiple exprs, some might be statements — just run them, print last
            let lines: Vec<&str> = expr_rust.trim().lines().collect();
            if lines.len() > 1 {
                let (stmts, last) = lines.split_at(lines.len() - 1);
                let body = format!(
                    "{}\nlet __result = {{\n    {}\n}};\nprintln!(\"{{:?}}\", __result);",
                    stmts.join("\n"),
                    last[0].trim().trim_end_matches(';')
                );
                state.build_program(&body)
            } else {
                state.build_print_program(expr_rust.trim())
            }
        };

        *last_rust = full_rust.clone();

        if state.show_rust {
            println!("{}", Color::dim("--- Rust ---"));
            for (i, line) in full_rust.lines().enumerate() {
                println!("{} {}", Color::dim(&format!("{:3}|", i + 1)), line);
            }
            println!("{}", Color::dim("---"));
        }

        // Compile and run
        let (stdout, stderr, success) = compile_and_run(&full_rust);

        if success {
            let output = stdout.trim();
            if !output.is_empty() {
                // Color the output
                println!("  {} {}", Color::green("=>"), output);
            } else if !all_statements {
                println!("  {} {}", Color::green("=>"), Color::dim("()"));
            }
            if !stderr.is_empty() {
                for line in stderr.trim().lines() {
                    eprintln!("  {}", Color::yellow(line));
                }
            }
            state.expr_id += 1;
            state.history.push(input.to_string());
        } else {
            // Compilation failed — show a cleaned-up error
            let clean_err = clean_rustc_error(&stderr);
            eprintln!("  {} {}", Color::red("! Compile error:"), clean_err);
            // Hint: show :last-rs
            if !state.show_rust {
                eprintln!("  {} {}", Color::dim("  tip:"), Color::dim("use :rust to see generated code, or :last-rs"));
            }
        }
    }

    if had_defs && exprs_to_eval.is_empty() {
        state.history.push(input.to_string());
    }
}

fn def_name(item: &TopLevel) -> String {
    match item {
        TopLevel::Defn(f) => f.name.clone(),
        TopLevel::Def(d) => d.name.clone(),
        TopLevel::DefStruct(s) => s.name.clone(),
        TopLevel::DefEnum(e) => e.name.clone(),
        TopLevel::DefTrait(t) => t.name.clone(),
        TopLevel::Impl(i) => {
            if let Some(ref t) = i.trait_name {
                format!("{} for {}", t, i.type_name)
            } else {
                i.type_name.clone()
            }
        }
        TopLevel::Mod(m) => format!("mod {}", m.name),
        TopLevel::Attr(_, inner) => def_name(inner),
        _ => "(item)".to_string(),
    }
}

/// Clean up rustc error output for REPL display:
/// - Remove file paths referencing temp files
/// - Keep only the essential error message
fn clean_rustc_error(stderr: &str) -> String {
    let mut lines: Vec<String> = Vec::new();
    for line in stderr.lines() {
        // Skip lines with temp file paths
        if line.contains("__cljrust_repl") && line.contains("-->") {
            continue;
        }
        // Clean up error[EXXXX] lines
        let cleaned = line
            .replace("__cljrust_repl.rs", "<repl>")
            .trim()
            .to_string();
        if !cleaned.is_empty() {
            lines.push(cleaned);
        }
    }
    // Take first few meaningful lines
    let meaningful: Vec<&str> = lines
        .iter()
        .filter(|l| l.starts_with("error") || l.starts_with("  ") || l.starts_with("|") || l.starts_with("help"))
        .take(8)
        .map(|s| s.as_str())
        .collect();
    if meaningful.is_empty() {
        lines.into_iter().take(5).collect::<Vec<_>>().join("\n")
    } else {
        meaningful.join("\n")
    }
}
