/// Code generator: transforms AST into Rust source code

use crate::ast::*;

pub struct CodeGen {
    indent: usize,
    output: String,
}

impl CodeGen {
    pub fn new() -> Self {
        CodeGen {
            indent: 0,
            output: String::new(),
        }
    }

    pub fn generate(&mut self, program: &Program) -> String {
        for item in &program.items {
            self.gen_top_level(item);
            self.output.push('\n');
        }
        self.output.clone()
    }

    fn write(&mut self, s: &str) {
        self.output.push_str(s);
    }

    fn writeln(&mut self, s: &str) {
        self.write_indent();
        self.output.push_str(s);
        self.output.push('\n');
    }

    fn write_indent(&mut self) {
        for _ in 0..self.indent {
            self.output.push_str("    ");
        }
    }

    fn indent(&mut self) {
        self.indent += 1;
    }

    fn dedent(&mut self) {
        self.indent = self.indent.saturating_sub(1);
    }

    // ── Top-level items ─────────────────────────────────────────

    fn gen_top_level(&mut self, item: &TopLevel) {
        match item {
            TopLevel::Use(u) => {
                self.writeln(&format!("use {};", u.path));
            }
            TopLevel::ExternCrate(name) => {
                self.writeln(&format!("extern crate {};", name));
            }
            TopLevel::Def(d) => self.gen_def(d),
            TopLevel::Defn(f) => self.gen_defn(f),
            TopLevel::DefStruct(s) => self.gen_struct(s),
            TopLevel::DefEnum(e) => self.gen_enum(e),
            TopLevel::DefTrait(t) => self.gen_trait(t),
            TopLevel::Impl(i) => self.gen_impl(i),
            TopLevel::Mod(m) => self.gen_mod(m),
            TopLevel::Attr(attr, inner) => {
                self.writeln(&format!("#[{}]", attr));
                self.gen_top_level(inner);
            }
            TopLevel::Expr(e) => {
                self.write_indent();
                self.gen_expr(e);
                self.write(";\n");
            }
        }
    }

    // ── def (static/const/let at top level) ─────────────────────

    fn gen_def(&mut self, d: &Def) {
        self.write_indent();
        if d.is_pub {
            self.write("pub ");
        }
        if d.is_const {
            self.write("const ");
            self.write(&d.name.to_uppercase());
        } else if d.mutable {
            self.write("static mut ");
            self.write(&d.name.to_uppercase());
        } else {
            self.write("static ");
            self.write(&d.name.to_uppercase());
        }
        if let Some(ref ty) = d.type_ann {
            self.write(&format!(": {}", ty));
        }
        self.write(" = ");
        self.gen_expr(&d.value);
        self.write(";\n");
    }

    // ── defn ────────────────────────────────────────────────────

    fn gen_defn(&mut self, f: &Defn) {
        self.write_indent();
        if f.is_pub {
            self.write("pub ");
        }
        if f.is_async {
            self.write("async ");
        }
        self.write("fn ");
        self.write(&f.name);
        if !f.generics.is_empty() {
            self.write(&format!("<{}>", f.generics.join(", ")));
        }
        self.write("(");
        for (i, p) in f.params.iter().enumerate() {
            if i > 0 {
                self.write(", ");
            }
            self.gen_param(p);
        }
        self.write(")");
        if let Some(ref ret) = f.ret_type {
            self.write(&format!(" -> {}", ret));
        }
        self.write(" {\n");
        self.indent();
        self.gen_body(&f.body);
        self.dedent();
        self.writeln("}");
    }

    fn gen_param(&mut self, p: &Param) {
        if p.is_ref {
            self.write("&self");
            return;
        }
        if p.is_mut_ref {
            self.write("&mut self");
            return;
        }
        // Check if this is "self"
        if p.name == "self" {
            if p.is_mut {
                self.write("mut self");
            } else {
                self.write("self");
            }
            return;
        }
        if p.is_mut {
            self.write("mut ");
        }
        self.write(&p.name);
        if let Some(ref ty) = p.type_ann {
            self.write(&format!(": {}", ty));
        }
    }

    // ── struct ──────────────────────────────────────────────────

    fn gen_struct(&mut self, s: &DefStruct) {
        if !s.derives.is_empty() {
            self.writeln(&format!("#[derive({})]", s.derives.join(", ")));
        }
        self.write_indent();
        if s.is_pub {
            self.write("pub ");
        }
        self.write(&format!("struct {}", s.name));
        if !s.generics.is_empty() {
            self.write(&format!("<{}>", s.generics.join(", ")));
        }
        self.write(" {\n");
        self.indent();
        for field in &s.fields {
            self.write_indent();
            if field.is_pub {
                self.write("pub ");
            }
            self.write(&format!("{}: {},\n", field.name, field.type_ann));
        }
        self.dedent();
        self.writeln("}");
    }

    // ── enum ────────────────────────────────────────────────────

    fn gen_enum(&mut self, e: &DefEnum) {
        if !e.derives.is_empty() {
            self.writeln(&format!("#[derive({})]", e.derives.join(", ")));
        }
        self.write_indent();
        if e.is_pub {
            self.write("pub ");
        }
        self.write(&format!("enum {}", e.name));
        if !e.generics.is_empty() {
            self.write(&format!("<{}>", e.generics.join(", ")));
        }
        self.write(" {\n");
        self.indent();
        for v in &e.variants {
            self.write_indent();
            self.write(&v.name);
            match &v.fields {
                VariantFields::Unit => {}
                VariantFields::Tuple(types) => {
                    self.write(&format!("({})", types.join(", ")));
                }
                VariantFields::Struct(fields) => {
                    self.write(" { ");
                    for (i, f) in fields.iter().enumerate() {
                        if i > 0 {
                            self.write(", ");
                        }
                        self.write(&format!("{}: {}", f.name, f.type_ann));
                    }
                    self.write(" }");
                }
            }
            self.write(",\n");
        }
        self.dedent();
        self.writeln("}");
    }

    // ── trait ────────────────────────────────────────────────────

    fn gen_trait(&mut self, t: &DefTrait) {
        self.write_indent();
        if t.is_pub {
            self.write("pub ");
        }
        self.write(&format!("trait {}", t.name));
        if !t.generics.is_empty() {
            self.write(&format!("<{}>", t.generics.join(", ")));
        }
        self.write(" {\n");
        self.indent();
        for m in &t.methods {
            if m.body.is_empty() {
                // Trait method signature only
                self.write_indent();
                self.write("fn ");
                self.write(&m.name);
                self.write("(");
                for (i, p) in m.params.iter().enumerate() {
                    if i > 0 {
                        self.write(", ");
                    }
                    self.gen_param(p);
                }
                self.write(")");
                if let Some(ref ret) = m.ret_type {
                    self.write(&format!(" -> {}", ret));
                }
                self.write(";\n");
            } else {
                self.gen_defn(m);
            }
        }
        self.dedent();
        self.writeln("}");
    }

    // ── impl ────────────────────────────────────────────────────

    fn gen_impl(&mut self, i: &Impl) {
        self.write_indent();
        self.write("impl");
        if !i.generics.is_empty() {
            self.write(&format!("<{}>", i.generics.join(", ")));
        }
        self.write(" ");
        if let Some(ref trait_name) = i.trait_name {
            self.write(&format!("{} for ", trait_name));
        }
        self.write(&i.type_name);
        if !i.generics.is_empty() {
            self.write(&format!("<{}>", i.generics.join(", ")));
        }
        self.write(" {\n");
        self.indent();
        for m in &i.methods {
            self.gen_defn(m);
            self.write("\n");
        }
        self.dedent();
        self.writeln("}");
    }

    // ── mod ─────────────────────────────────────────────────────

    fn gen_mod(&mut self, m: &Mod) {
        self.write_indent();
        if m.is_pub {
            self.write("pub ");
        }
        self.write(&format!("mod {} {{\n", m.name));
        self.indent();
        for item in &m.items {
            self.gen_top_level(item);
            self.write("\n");
        }
        self.dedent();
        self.writeln("}");
    }

    // ── Body (list of exprs, last is return value) ──────────────

    fn gen_body(&mut self, body: &[Expr]) {
        if body.is_empty() {
            return;
        }
        for (i, expr) in body.iter().enumerate() {
            let is_last = i == body.len() - 1;
            if is_last {
                // Last expression is the return value (no semicolon)
                self.write_indent();
                self.gen_expr(expr);
                self.write("\n");
            } else {
                self.gen_statement(expr);
            }
        }
    }

    fn gen_statement(&mut self, expr: &Expr) {
        match expr {
            Expr::Let { .. } | Expr::For { .. } | Expr::While { .. } | Expr::Loop { .. } => {
                self.write_indent();
                self.gen_expr(expr);
                self.write("\n");
            }
            Expr::If { else_: None, .. } => {
                self.write_indent();
                self.gen_expr(expr);
                self.write("\n");
            }
            Expr::Set { .. } | Expr::Return(_) | Expr::Break(_) | Expr::Continue => {
                self.write_indent();
                self.gen_expr(expr);
                self.write(";\n");
            }
            _ => {
                self.write_indent();
                self.gen_expr(expr);
                self.write(";\n");
            }
        }
    }

    // ── Expressions ─────────────────────────────────────────────

    fn gen_expr(&mut self, expr: &Expr) {
        match expr {
            Expr::Integer(n) => self.write(&n.to_string()),
            Expr::Float(f) => {
                let s = f.to_string();
                self.write(&s);
                if !s.contains('.') {
                    self.write(".0");
                }
            }
            Expr::Str(s) => {
                self.write(&format!("\"{}\"", escape_str(s)));
            }
            Expr::Char(c) => {
                self.write(&format!("'{}'", escape_char(*c)));
            }
            Expr::Bool(b) => self.write(if *b { "true" } else { "false" }),
            Expr::Nil => self.write("()"),
            Expr::Symbol(s) => self.write(s),
            Expr::Keyword(k) => {
                // Keywords become string literals in most contexts
                self.write(&format!("\"{}\"", k));
            }

            // ── Collections ─────────────────────────────────
            Expr::Vec(elems) => {
                self.write("vec![");
                for (i, e) in elems.iter().enumerate() {
                    if i > 0 {
                        self.write(", ");
                    }
                    self.gen_expr(e);
                }
                self.write("]");
            }
            Expr::HashMap(pairs) => {
                self.write("std::collections::HashMap::from([");
                for (i, (k, v)) in pairs.iter().enumerate() {
                    if i > 0 {
                        self.write(", ");
                    }
                    self.write("(");
                    self.gen_expr(k);
                    self.write(", ");
                    self.gen_expr(v);
                    self.write(")");
                }
                self.write("])");
            }
            Expr::Tuple(elems) => {
                self.write("(");
                for (i, e) in elems.iter().enumerate() {
                    if i > 0 {
                        self.write(", ");
                    }
                    self.gen_expr(e);
                }
                if elems.len() == 1 {
                    self.write(",");
                }
                self.write(")");
            }

            // ── Let ─────────────────────────────────────────
            Expr::Let { bindings, body } => {
                self.write("{\n");
                self.indent();
                for b in bindings {
                    self.write_indent();
                    self.write("let ");
                    if b.mutable {
                        self.write("mut ");
                    }
                    self.write(&b.name);
                    if let Some(ref ty) = b.type_ann {
                        self.write(&format!(": {}", ty));
                    }
                    self.write(" = ");
                    self.gen_expr(&b.value);
                    self.write(";\n");
                }
                self.gen_body(body);
                self.dedent();
                self.write_indent();
                self.write("}");
            }

            // ── If ──────────────────────────────────────────
            Expr::If { cond, then, else_ } => {
                self.write("if ");
                self.gen_expr(cond);
                self.write(" {\n");
                self.indent();
                self.write_indent();
                self.gen_expr(then);
                self.write("\n");
                self.dedent();
                self.write_indent();
                self.write("}");
                if let Some(ref e) = else_ {
                    self.write(" else {\n");
                    self.indent();
                    self.write_indent();
                    self.gen_expr(e);
                    self.write("\n");
                    self.dedent();
                    self.write_indent();
                    self.write("}");
                }
            }

            // ── Match ───────────────────────────────────────
            Expr::Match { expr, arms } => {
                self.write("match ");
                self.gen_expr(expr);
                self.write(" {\n");
                self.indent();
                for arm in arms {
                    self.write_indent();
                    self.gen_pattern(&arm.pattern);
                    if let Some(ref guard) = arm.guard {
                        self.write(" if ");
                        self.gen_expr(guard);
                    }
                    self.write(" => ");
                    match &arm.body {
                        Expr::Do(stmts) => {
                            self.write("{\n");
                            self.indent();
                            self.gen_body(stmts);
                            self.dedent();
                            self.write_indent();
                            self.write("}");
                        }
                        other => {
                            self.gen_expr(other);
                        }
                    }
                    self.write(",\n");
                }
                self.dedent();
                self.write_indent();
                self.write("}");
            }

            // ── Do (block) ──────────────────────────────────
            Expr::Do(exprs) => {
                self.write("{\n");
                self.indent();
                self.gen_body(exprs);
                self.dedent();
                self.write_indent();
                self.write("}");
            }
            Expr::Block(exprs) => {
                self.write("{\n");
                self.indent();
                self.gen_body(exprs);
                self.dedent();
                self.write_indent();
                self.write("}");
            }

            // ── Loop / Recur ────────────────────────────────
            Expr::Loop { bindings, body } => {
                if !bindings.is_empty() {
                    // Clojure-style loop/recur → Rust loop with mutable vars
                    self.write("{\n");
                    self.indent();
                    let names: Vec<String> = bindings.iter().map(|b| b.name.clone()).collect();
                    for b in bindings {
                        self.write_indent();
                        self.write(&format!("let mut {} = ", b.name));
                        self.gen_expr(&b.value);
                        self.write(";\n");
                    }
                    self.writeln("loop {");
                    self.indent();
                    // Transform recur calls into assignments + continue
                    self.gen_loop_body(body, &names);
                    self.dedent();
                    self.writeln("}");
                    self.dedent();
                    self.write_indent();
                    self.write("}");
                } else {
                    self.write("loop {\n");
                    self.indent();
                    self.gen_body(body);
                    self.dedent();
                    self.write_indent();
                    self.write("}");
                }
            }
            Expr::Recur(args) => {
                // This shouldn't be reached if loop transformation works,
                // but as fallback:
                self.write("/* recur */ continue");
                let _ = args;
            }

            // ── For ─────────────────────────────────────────
            Expr::For { binding, iter, body } => {
                self.write("for ");
                self.write(binding);
                self.write(" in ");
                self.gen_expr(iter);
                self.write(" {\n");
                self.indent();
                self.gen_body(body);
                self.dedent();
                self.write_indent();
                self.write("}");
            }

            // ── While ───────────────────────────────────────
            Expr::While { cond, body } => {
                self.write("while ");
                self.gen_expr(cond);
                self.write(" {\n");
                self.indent();
                self.gen_body(body);
                self.dedent();
                self.write_indent();
                self.write("}");
            }

            // ── Fn (closure) ────────────────────────────────
            Expr::Fn { params, ret_type, body } => {
                self.write("|");
                for (i, p) in params.iter().enumerate() {
                    if i > 0 {
                        self.write(", ");
                    }
                    if p.is_mut {
                        self.write("mut ");
                    }
                    self.write(&p.name);
                    if let Some(ref ty) = p.type_ann {
                        self.write(&format!(": {}", ty));
                    }
                }
                self.write("|");
                if let Some(ref ret) = ret_type {
                    self.write(&format!(" -> {}", ret));
                }
                if body.len() == 1 {
                    self.write(" ");
                    self.gen_expr(&body[0]);
                } else {
                    self.write(" {\n");
                    self.indent();
                    self.gen_body(body);
                    self.dedent();
                    self.write_indent();
                    self.write("}");
                }
            }

            // ── Call ────────────────────────────────────────
            Expr::Call { func, args } => {
                self.gen_expr(func);
                self.write("(");
                for (i, a) in args.iter().enumerate() {
                    if i > 0 {
                        self.write(", ");
                    }
                    self.gen_expr(a);
                }
                self.write(")");
            }

            // ── Macro call ──────────────────────────────────
            Expr::MacroCall { name, args } => {
                self.write(name);
                self.write("(");
                for (i, a) in args.iter().enumerate() {
                    if i > 0 {
                        self.write(", ");
                    }
                    self.gen_expr(a);
                }
                self.write(")");
            }

            // ── Method call ─────────────────────────────────
            Expr::MethodCall { object, method, args } => {
                self.gen_expr(object);
                self.write(".");
                self.write(method);
                self.write("(");
                for (i, a) in args.iter().enumerate() {
                    if i > 0 {
                        self.write(", ");
                    }
                    self.gen_expr(a);
                }
                self.write(")");
            }

            // ── Field access ────────────────────────────────
            Expr::FieldAccess { object, field } => {
                self.gen_expr(object);
                self.write(".");
                self.write(field);
            }

            // ── Static call ─────────────────────────────────
            Expr::StaticCall { type_name, method, args } => {
                self.write(type_name);
                self.write("::");
                self.write(method);
                self.write("(");
                for (i, a) in args.iter().enumerate() {
                    if i > 0 {
                        self.write(", ");
                    }
                    self.gen_expr(a);
                }
                self.write(")");
            }

            // ── Struct init ─────────────────────────────────
            Expr::StructInit { name, fields } => {
                self.write(name);
                self.write(" { ");
                for (i, (fname, fval)) in fields.iter().enumerate() {
                    if i > 0 {
                        self.write(", ");
                    }
                    self.write(fname);
                    self.write(": ");
                    self.gen_expr(fval);
                }
                self.write(" }");
            }

            // ── Operators ───────────────────────────────────
            Expr::BinOp { op, left, right } => {
                self.write("(");
                self.gen_expr(left);
                self.write(&format!(" {} ", op.to_rust()));
                self.gen_expr(right);
                self.write(")");
            }
            Expr::UnaryOp { op, operand } => {
                self.write(op.to_rust());
                self.gen_expr(operand);
            }

            // ── Set (assignment) ────────────────────────────
            Expr::Set { target, value } => {
                self.gen_expr(target);
                self.write(" = ");
                self.gen_expr(value);
            }

            // ── References ──────────────────────────────────
            Expr::Ref(inner) => {
                self.write("&");
                self.gen_expr(inner);
            }
            Expr::RefMut(inner) => {
                self.write("&mut ");
                self.gen_expr(inner);
            }
            Expr::Deref(inner) => {
                self.write("*");
                self.gen_expr(inner);
            }

            // ── Type cast ───────────────────────────────────
            Expr::As { expr, type_name } => {
                self.write("(");
                self.gen_expr(expr);
                self.write(&format!(" as {})", type_name));
            }

            // ── Range ───────────────────────────────────────
            Expr::Range { start, end, inclusive } => {
                self.gen_expr(start);
                if *inclusive {
                    self.write("..=");
                } else {
                    self.write("..");
                }
                self.gen_expr(end);
            }

            // ── Return / Break / Continue ───────────────────
            Expr::Return(val) => {
                self.write("return");
                if let Some(v) = val {
                    self.write(" ");
                    self.gen_expr(v);
                }
            }
            Expr::Break(val) => {
                self.write("break");
                if let Some(v) = val {
                    self.write(" ");
                    self.gen_expr(v);
                }
            }
            Expr::Continue => {
                self.write("continue");
            }

            // ── Await ───────────────────────────────────────
            Expr::Await(inner) => {
                self.gen_expr(inner);
                self.write(".await");
            }

            // ── Try (?) ─────────────────────────────────────
            Expr::Try(inner) => {
                self.gen_expr(inner);
                self.write("?");
            }

            // ── Raw Rust ────────────────────────────────────
            Expr::RawRust(code) => {
                self.write(code);
            }

            // ── Index ───────────────────────────────────────
            Expr::Index { object, index } => {
                self.gen_expr(object);
                self.write("[");
                self.gen_expr(index);
                self.write("]");
            }
        }
    }

    // ── Loop body with recur transformation ─────────────────────

    fn gen_loop_body(&mut self, body: &[Expr], loop_vars: &[String]) {
        if body.is_empty() {
            return;
        }
        for (i, expr) in body.iter().enumerate() {
            let is_last = i == body.len() - 1;
            if is_last {
                self.gen_loop_expr(expr, loop_vars, true);
            } else {
                self.gen_loop_expr(expr, loop_vars, false);
            }
        }
    }

    fn gen_loop_expr(&mut self, expr: &Expr, loop_vars: &[String], is_tail: bool) {
        match expr {
            Expr::Recur(args) if is_tail => {
                // Transform recur into assignments + continue
                for (i, (name, val)) in loop_vars.iter().zip(args.iter()).enumerate() {
                    self.write_indent();
                    let tmp = format!("__recur_{}", i);
                    self.write(&format!("let {} = ", tmp));
                    self.gen_expr(val);
                    self.write(";\n");
                }
                for (i, name) in loop_vars.iter().enumerate() {
                    self.write_indent();
                    self.write(&format!("{} = __recur_{};\n", name, i));
                }
                self.writeln("continue;");
            }
            Expr::If { cond, then, else_ } if is_tail => {
                self.write_indent();
                self.write("if ");
                self.gen_expr(cond);
                self.write(" {\n");
                self.indent();
                self.gen_loop_expr(then, loop_vars, true);
                self.dedent();
                self.write_indent();
                self.write("}");
                if let Some(ref e) = else_ {
                    self.write(" else {\n");
                    self.indent();
                    self.gen_loop_expr(e, loop_vars, true);
                    self.dedent();
                    self.write_indent();
                    self.write("}");
                }
                self.write("\n");
            }
            _ => {
                if is_tail {
                    self.write_indent();
                    self.write("break ");
                    self.gen_expr(expr);
                    self.write(";\n");
                } else {
                    self.gen_statement(expr);
                }
            }
        }
    }

    // ── Pattern ─────────────────────────────────────────────────

    fn gen_pattern(&mut self, pat: &Pattern) {
        match pat {
            Pattern::Wildcard => self.write("_"),
            Pattern::Literal(expr) => self.gen_expr(expr),
            Pattern::Binding(name) => self.write(name),
            Pattern::Tuple(pats) => {
                self.write("(");
                for (i, p) in pats.iter().enumerate() {
                    if i > 0 {
                        self.write(", ");
                    }
                    self.gen_pattern(p);
                }
                self.write(")");
            }
            Pattern::Struct { name, fields } => {
                self.write(name);
                self.write(" { ");
                for (i, (fname, fpat)) in fields.iter().enumerate() {
                    if i > 0 {
                        self.write(", ");
                    }
                    self.write(fname);
                    self.write(": ");
                    self.gen_pattern(fpat);
                }
                self.write(" }");
            }
            Pattern::TupleStruct { name, fields } => {
                self.write(name);
                self.write("(");
                for (i, p) in fields.iter().enumerate() {
                    if i > 0 {
                        self.write(", ");
                    }
                    self.gen_pattern(p);
                }
                self.write(")");
            }
            Pattern::Ref(inner) => {
                self.write("&");
                self.gen_pattern(inner);
            }
            Pattern::Or(pats) => {
                for (i, p) in pats.iter().enumerate() {
                    if i > 0 {
                        self.write(" | ");
                    }
                    self.gen_pattern(p);
                }
            }
        }
    }
}

// ── Helpers ─────────────────────────────────────────────────────

fn escape_str(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('\t', "\\t")
}

fn escape_char(c: char) -> String {
    match c {
        '\n' => "\\n".to_string(),
        '\r' => "\\r".to_string(),
        '\t' => "\\t".to_string(),
        '\\' => "\\\\".to_string(),
        '\'' => "\\'".to_string(),
        c => c.to_string(),
    }
}
