mod ast;
mod codegen;
mod lexer;
mod parser;

use codegen::CodeGen;
use lexer::Lexer;
use parser::Parser;
use std::env;
use std::fs;
use std::io::{self, Read};
use std::path::PathBuf;
use std::process::Command;

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        print_usage();
        std::process::exit(1);
    }

    match args[1].as_str() {
        "compile" | "c" => {
            if args.len() < 3 {
                eprintln!("Usage: cljrust compile <file.cljr> [--emit rust|binary] [-o output]");
                std::process::exit(1);
            }
            cmd_compile(&args[2..]);
        }
        "run" | "r" => {
            if args.len() < 3 {
                eprintln!("Usage: cljrust run <file.cljr>");
                std::process::exit(1);
            }
            cmd_run(&args[2..]);
        }
        "new" => {
            if args.len() < 3 {
                eprintln!("Usage: cljrust new <project-name>");
                std::process::exit(1);
            }
            cmd_new(&args[2]);
        }
        "emit" | "e" => {
            if args.len() < 3 {
                eprintln!("Usage: cljrust emit <file.cljr>");
                std::process::exit(1);
            }
            cmd_emit(&args[2]);
        }
        "repl" => {
            cmd_repl();
        }
        "--help" | "-h" | "help" => {
            print_usage();
        }
        "--version" | "-v" => {
            println!("cljrust 0.1.0 — Clojure syntax → Rust compiler");
        }
        file if file.ends_with(".cljr") => {
            cmd_run(&args[1..]);
        }
        other => {
            eprintln!("Unknown command: {}", other);
            print_usage();
            std::process::exit(1);
        }
    }
}

fn print_usage() {
    println!(
        r#"cljrust 0.1.0 — Clojure syntax that compiles to Rust

USAGE:
    cljrust <command> [options]

COMMANDS:
    new <name>           Create a new cljrust project (Cargo project)
    compile <file.cljr>  Compile .cljr to Rust source or binary
      --emit rust        Output generated Rust code (default)
      --emit binary      Compile to binary via rustc
      -o <output>        Output file path
    emit <file.cljr>     Print generated Rust code to stdout
    run <file.cljr>      Compile and run immediately
    repl                 Interactive REPL (emit Rust for each expression)
    help                 Show this help

EXAMPLES:
    cljrust new my-app
    cljrust emit hello.cljr
    cljrust run hello.cljr
    cljrust compile hello.cljr --emit binary -o hello"#
    );
}

// ── Compile pipeline ────────────────────────────────────────────

fn compile_source(source: &str) -> Result<String, String> {
    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize()?;
    let mut parser = Parser::new(tokens);
    let program = parser.parse_program()?;
    let mut codegen = CodeGen::new();
    Ok(codegen.generate(&program))
}

// ── Commands ────────────────────────────────────────────────────

fn cmd_emit(file: &str) {
    let source = read_source(file);
    match compile_source(&source) {
        Ok(rust_code) => print!("{}", rust_code),
        Err(e) => {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    }
}

fn cmd_compile(args: &[String]) {
    let file = &args[0];
    let mut emit_mode = "rust";
    let mut output: Option<String> = None;

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--emit" => {
                i += 1;
                if i < args.len() {
                    emit_mode = if args[i] == "binary" { "binary" } else { "rust" };
                }
            }
            "-o" => {
                i += 1;
                if i < args.len() {
                    output = Some(args[i].clone());
                }
            }
            _ => {}
        }
        i += 1;
    }

    let source = read_source(file);
    let rust_code = match compile_source(&source) {
        Ok(code) => code,
        Err(e) => {
            eprintln!("Compile error: {}", e);
            std::process::exit(1);
        }
    };

    match emit_mode {
        "binary" => {
            let rs_path = output
                .as_deref()
                .map(|o| format!("{}.rs", o))
                .unwrap_or_else(|| file.replace(".cljr", ".rs"));
            let bin_path = output
                .unwrap_or_else(|| file.replace(".cljr", ""));
            fs::write(&rs_path, &rust_code).expect("Failed to write .rs file");
            let status = Command::new("rustc")
                .args(&[&rs_path, "-o", &bin_path])
                .status()
                .expect("Failed to run rustc. Is Rust installed?");
            if !status.success() {
                eprintln!("rustc compilation failed");
                std::process::exit(1);
            }
            let _ = fs::remove_file(&rs_path);
            println!("Compiled to: {}", bin_path);
        }
        _ => {
            let out_path = output.unwrap_or_else(|| file.replace(".cljr", ".rs"));
            fs::write(&out_path, &rust_code).expect("Failed to write output");
            println!("Generated: {}", out_path);
        }
    }
}

fn cmd_run(args: &[String]) {
    let file = &args[0];
    let source = read_source(file);
    let rust_code = match compile_source(&source) {
        Ok(code) => code,
        Err(e) => {
            eprintln!("Compile error: {}", e);
            std::process::exit(1);
        }
    };

    let tmp_dir = env::temp_dir();
    let rs_path = tmp_dir.join("__cljrust_tmp.rs");
    let bin_path = tmp_dir.join("__cljrust_tmp");

    fs::write(&rs_path, &rust_code).expect("Failed to write temp .rs file");

    let compile_status = Command::new("rustc")
        .args(&[
            rs_path.to_str().unwrap(),
            "-o",
            bin_path.to_str().unwrap(),
            "--edition",
            "2021",
        ])
        .status()
        .expect("Failed to run rustc. Is Rust installed?");

    if !compile_status.success() {
        eprintln!("--- Generated Rust code ---");
        for (i, line) in rust_code.lines().enumerate() {
            eprintln!("{:4} | {}", i + 1, line);
        }
        eprintln!("--- End ---");
        eprintln!("Compilation failed.");
        let _ = fs::remove_file(&rs_path);
        std::process::exit(1);
    }

    let _ = fs::remove_file(&rs_path);

    let run_args: Vec<&str> = args.iter().skip(1).map(|s| s.as_str()).collect();
    let status = Command::new(bin_path.to_str().unwrap())
        .args(&run_args)
        .status()
        .expect("Failed to run compiled binary");

    let _ = fs::remove_file(&bin_path);

    std::process::exit(status.code().unwrap_or(1));
}

fn cmd_new(name: &str) {
    let project_dir = PathBuf::from(name);
    if project_dir.exists() {
        eprintln!("Directory '{}' already exists", name);
        std::process::exit(1);
    }

    let src_dir = project_dir.join("src");
    fs::create_dir_all(&src_dir).expect("Failed to create project directories");

    let cargo_toml = format!(
        r#"[package]
name = "{}"
version = "0.1.0"
edition = "2021"

[dependencies]
"#,
        name
    );
    fs::write(project_dir.join("Cargo.toml"), cargo_toml).expect("Failed to write Cargo.toml");

    let main_cljr = r#"; Welcome to cljrust!
; Clojure syntax → Rust compiler

(defn main []
  (println! "Hello from cljrust!"))
"#;
    fs::write(src_dir.join("main.cljr"), main_cljr).expect("Failed to write main.cljr");

    let build_hint = "#!/bin/sh\n# Build script: compile .cljr to .rs then use cargo\ncljrust compile src/main.cljr -o src/main.rs\ncargo build\n";
    fs::write(project_dir.join("build.sh"), build_hint).expect("Failed to write build.sh");

    println!("Created new cljrust project: {}/", name);
    println!("  src/main.cljr — your source code");
    println!("  Cargo.toml    — Rust project config (add dependencies here)");
    println!();
    println!("Quick start:");
    println!("  cd {}", name);
    println!("  cljrust emit src/main.cljr        # see generated Rust");
    println!("  cljrust run src/main.cljr          # compile & run");
    println!("  cljrust compile src/main.cljr -o src/main.rs  # then: cargo build");
}

fn cmd_repl() {
    println!("cljrust REPL — type Clojure expressions, see Rust output");
    println!("Type :quit to exit\n");

    let stdin = io::stdin();
    let mut input = String::new();

    loop {
        eprint!("cljr> ");
        input.clear();
        if stdin.read_line(&mut input).is_err() || input.trim() == ":quit" {
            break;
        }
        let trimmed = input.trim();
        if trimmed.is_empty() {
            continue;
        }

        match compile_source(trimmed) {
            Ok(rust_code) => {
                println!("=> {}", rust_code.trim());
            }
            Err(e) => {
                eprintln!("Error: {}", e);
            }
        }
    }
}

fn read_source(file: &str) -> String {
    if file == "-" {
        let mut buf = String::new();
        io::stdin().read_to_string(&mut buf).expect("Failed to read stdin");
        buf
    } else {
        fs::read_to_string(file).unwrap_or_else(|e| {
            eprintln!("Cannot read '{}': {}", file, e);
            std::process::exit(1);
        })
    }
}
