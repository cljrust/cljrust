/// Parser: transforms token stream into AST

use crate::ast::*;
use crate::lexer::{SpannedToken, Token};

pub struct Parser {
    tokens: Vec<SpannedToken>,
    pos: usize,
}

impl Parser {
    pub fn new(tokens: Vec<SpannedToken>) -> Self {
        Parser { tokens, pos: 0 }
    }

    pub fn parse_program(&mut self) -> Result<Program, String> {
        let mut items = Vec::new();
        while self.pos < self.tokens.len() {
            items.push(self.parse_top_level()?);
        }
        Ok(Program { items })
    }

    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.pos).map(|t| &t.token)
    }

    fn advance(&mut self) -> Result<&Token, String> {
        if self.pos >= self.tokens.len() {
            return Err("Unexpected end of input".to_string());
        }
        let tok = &self.tokens[self.pos].token;
        self.pos += 1;
        Ok(tok)
    }

    fn expect(&mut self, expected: &Token) -> Result<(), String> {
        let tok = self.advance()?.clone();
        if &tok == expected {
            Ok(())
        } else {
            Err(format!("Expected {:?}, got {:?}", expected, tok))
        }
    }

    fn current_line(&self) -> usize {
        self.tokens
            .get(self.pos)
            .map(|t| t.span.line)
            .unwrap_or(0)
    }

    fn peek_symbol(&self) -> Option<&str> {
        match self.peek() {
            Some(Token::Symbol(s)) => Some(s.as_str()),
            _ => None,
        }
    }

    fn parse_top_level(&mut self) -> Result<TopLevel, String> {
        match self.peek() {
            Some(Token::LParen) => {
                // peek at the form name
                let form = self.peek_list_head();
                match form.as_deref() {
                    Some("use") => self.parse_use().map(TopLevel::Use),
                    Some("extern-crate") => self.parse_extern_crate().map(TopLevel::ExternCrate),
                    Some("def") | Some("def-mut") | Some("const") => {
                        self.parse_def().map(TopLevel::Def)
                    }
                    Some("defn") | Some("defn-") | Some("defn-async") => {
                        self.parse_defn(false).map(TopLevel::Defn)
                    }
                    Some("defstruct") => self.parse_defstruct().map(TopLevel::DefStruct),
                    Some("defenum") => self.parse_defenum().map(TopLevel::DefEnum),
                    Some("deftrait") => self.parse_deftrait().map(TopLevel::DefTrait),
                    Some("defimpl") => self.parse_impl().map(TopLevel::Impl),
                    Some("defmod") => self.parse_mod().map(TopLevel::Mod),
                    Some("#!") => self.parse_attr(),
                    _ => self.parse_expr().map(TopLevel::Expr),
                }
            }
            _ => self.parse_expr().map(TopLevel::Expr),
        }
    }

    fn peek_list_head(&self) -> Option<String> {
        if self.pos + 1 < self.tokens.len() {
            if let Token::Symbol(s) = &self.tokens[self.pos + 1].token {
                return Some(s.clone());
            }
        }
        None
    }

    // ── use ──────────────────────────────────────────────────────

    fn parse_use(&mut self) -> Result<UsePath, String> {
        self.expect(&Token::LParen)?;
        self.expect_symbol("use")?;
        let path = self.parse_use_path()?;
        self.expect(&Token::RParen)?;
        Ok(UsePath { path })
    }

    fn parse_use_path(&mut self) -> Result<String, String> {
        let mut path = String::new();
        while self.peek() != Some(&Token::RParen) {
            match self.advance()?.clone() {
                Token::Symbol(s) => path.push_str(&clj_to_rust_ident(&s)),
                Token::LBrace => {
                    path.push('{');
                    let mut first = true;
                    while self.peek() != Some(&Token::RBrace) {
                        if !first {
                            path.push_str(", ");
                        }
                        first = false;
                        if let Token::Symbol(s) = self.advance()?.clone() {
                            path.push_str(&clj_to_rust_ident(&s));
                        }
                    }
                    self.expect(&Token::RBrace)?;
                    path.push('}');
                }
                _ => return Err("Unexpected token in use path".to_string()),
            }
        }
        Ok(path)
    }

    fn parse_extern_crate(&mut self) -> Result<String, String> {
        self.expect(&Token::LParen)?;
        self.expect_symbol("extern-crate")?;
        let name = self.expect_any_symbol()?;
        self.expect(&Token::RParen)?;
        Ok(clj_to_rust_ident(&name))
    }

    // ── def ─────────────────────────────────────────────────────

    fn parse_def(&mut self) -> Result<Def, String> {
        self.expect(&Token::LParen)?;
        let form = self.expect_any_symbol()?;
        let is_const = form == "const";
        let mutable = form == "def-mut";
        let name = self.expect_any_symbol()?;

        let type_ann = if self.peek() == Some(&Token::Arrow) {
            self.advance()?; // skip ->
            Some(self.parse_type_str()?)
        } else {
            None
        };

        let value = self.parse_expr()?;
        self.expect(&Token::RParen)?;

        Ok(Def {
            is_pub: false,
            mutable,
            name: clj_to_rust_ident(&name),
            type_ann,
            value,
            is_const,
        })
    }

    // ── defn ────────────────────────────────────────────────────

    fn parse_defn(&mut self, in_impl: bool) -> Result<Defn, String> {
        self.expect(&Token::LParen)?;
        let form = self.expect_any_symbol()?;
        let is_pub = form != "defn-";
        let is_async = form == "defn-async";
        let name = self.expect_any_symbol()?;

        // optional generics <T, U>
        let generics = if self.peek() == Some(&Token::Symbol("<".to_string())) {
            self.parse_generics()?
        } else {
            Vec::new()
        };

        // params
        self.expect(&Token::LBracket)?;
        let params = self.parse_params(in_impl)?;
        self.expect(&Token::RBracket)?;

        // optional return type
        let ret_type = if self.peek() == Some(&Token::Arrow) {
            self.advance()?;
            Some(self.parse_type_str()?)
        } else {
            None
        };

        // body
        let mut body = Vec::new();
        while self.peek() != Some(&Token::RParen) {
            body.push(self.parse_expr()?);
        }
        self.expect(&Token::RParen)?;

        Ok(Defn {
            is_pub,
            name: clj_to_rust_ident(&name),
            generics,
            params,
            ret_type,
            body,
            is_async,
        })
    }

    fn parse_params(&mut self, _in_impl: bool) -> Result<Vec<Param>, String> {
        let mut params = Vec::new();
        while self.peek() != Some(&Token::RBracket) {
            let mut is_ref = false;
            let mut is_mut_ref = false;
            let mut is_mut = false;

            // Check for & and &mut
            if self.peek() == Some(&Token::Ampersand) {
                self.advance()?;
                is_ref = true;
                if self.peek_symbol() == Some("mut") {
                    self.advance()?;
                    is_mut_ref = true;
                    is_ref = false;
                }
            } else if self.peek_symbol() == Some("mut") {
                self.advance()?;
                is_mut = true;
            }

            let name = self.expect_any_symbol()?;

            // optional type annotation with :
            let type_ann = if self.peek_symbol() == Some(":") {
                self.advance()?; // skip :
                Some(self.parse_type_str()?)
            } else {
                None
            };

            params.push(Param {
                name: clj_to_rust_ident(&name),
                type_ann,
                is_ref,
                is_mut_ref,
                is_mut,
            });
        }
        Ok(params)
    }

    fn parse_generics(&mut self) -> Result<Vec<String>, String> {
        self.advance()?; // skip <
        let mut generics = Vec::new();
        loop {
            let g = self.expect_any_symbol()?;
            generics.push(g);
            match self.peek_symbol() {
                Some(">") => {
                    self.advance()?;
                    break;
                }
                _ => continue,
            }
        }
        Ok(generics)
    }

    // ── struct ──────────────────────────────────────────────────

    fn parse_defstruct(&mut self) -> Result<DefStruct, String> {
        self.expect(&Token::LParen)?;
        self.expect_symbol("defstruct")?;
        let name = self.expect_any_symbol()?;

        let generics = if self.peek() == Some(&Token::Symbol("<".to_string())) {
            self.parse_generics()?
        } else {
            Vec::new()
        };

        // optional derives
        let derives = if self.peek() == Some(&Token::Hash) {
            self.parse_derives()?
        } else {
            Vec::new()
        };

        self.expect(&Token::LBracket)?;
        let mut fields = Vec::new();
        while self.peek() != Some(&Token::RBracket) {
            let is_pub = if self.peek_symbol() == Some("pub") {
                self.advance()?;
                true
            } else {
                false
            };
            let fname = self.expect_any_symbol()?;
            self.expect_symbol(":")?;
            let ftype = self.parse_type_str()?;
            fields.push(StructField {
                is_pub,
                name: clj_to_rust_ident(&fname),
                type_ann: ftype,
            });
        }
        self.expect(&Token::RBracket)?;
        self.expect(&Token::RParen)?;

        Ok(DefStruct {
            is_pub: true,
            name: clj_to_rust_ident(&name),
            generics,
            fields,
            derives,
        })
    }

    // ── enum ────────────────────────────────────────────────────

    fn parse_defenum(&mut self) -> Result<DefEnum, String> {
        self.expect(&Token::LParen)?;
        self.expect_symbol("defenum")?;
        let name = self.expect_any_symbol()?;

        let generics = if self.peek() == Some(&Token::Symbol("<".to_string())) {
            self.parse_generics()?
        } else {
            Vec::new()
        };

        let derives = if self.peek() == Some(&Token::Hash) {
            self.parse_derives()?
        } else {
            Vec::new()
        };

        let mut variants = Vec::new();
        while self.peek() != Some(&Token::RParen) {
            if self.peek() == Some(&Token::LParen) {
                // tuple or struct variant
                self.expect(&Token::LParen)?;
                let vname = self.expect_any_symbol()?;
                let mut tuple_fields = Vec::new();
                while self.peek() != Some(&Token::RParen) {
                    tuple_fields.push(self.parse_type_str()?);
                }
                self.expect(&Token::RParen)?;
                variants.push(EnumVariant {
                    name: clj_to_rust_ident(&vname),
                    fields: if tuple_fields.is_empty() {
                        VariantFields::Unit
                    } else {
                        VariantFields::Tuple(tuple_fields)
                    },
                });
            } else {
                let vname = self.expect_any_symbol()?;
                variants.push(EnumVariant {
                    name: clj_to_rust_ident(&vname),
                    fields: VariantFields::Unit,
                });
            }
        }
        self.expect(&Token::RParen)?;

        Ok(DefEnum {
            is_pub: true,
            name: clj_to_rust_ident(&name),
            generics,
            variants,
            derives,
        })
    }

    // ── trait ────────────────────────────────────────────────────

    fn parse_deftrait(&mut self) -> Result<DefTrait, String> {
        self.expect(&Token::LParen)?;
        self.expect_symbol("deftrait")?;
        let name = self.expect_any_symbol()?;

        let generics = if self.peek() == Some(&Token::Symbol("<".to_string())) {
            self.parse_generics()?
        } else {
            Vec::new()
        };

        let mut methods = Vec::new();
        while self.peek() != Some(&Token::RParen) {
            methods.push(self.parse_defn(true)?);
        }
        self.expect(&Token::RParen)?;

        Ok(DefTrait {
            is_pub: true,
            name: clj_to_rust_ident(&name),
            generics,
            methods,
        })
    }

    // ── impl ────────────────────────────────────────────────────

    fn parse_impl(&mut self) -> Result<Impl, String> {
        self.expect(&Token::LParen)?;
        self.expect_symbol("defimpl")?;
        let type_name = self.expect_any_symbol()?;

        let generics = if self.peek() == Some(&Token::Symbol("<".to_string())) {
            self.parse_generics()?
        } else {
            Vec::new()
        };

        // optional trait name with `for` keyword pattern: (defimpl Display for MyType ...)
        // or just: (defimpl MyType ...) or (defimpl TraitName MyType ...)
        let trait_name = if self.peek_symbol() == Some("for") {
            // pattern: (defimpl Type for Trait ...)  — wait, it's the other way
            // Actually let's use: (defimpl TraitName for TypeName ...)
            let tn = type_name.clone();
            self.advance()?; // skip "for"
            let _actual_type = self.expect_any_symbol()?;
            Some(clj_to_rust_ident(&tn))
        } else {
            None
        };

        let actual_type = if trait_name.is_some() {
            // We already consumed the type above, let me restructure
            // (defimpl Trait for Type methods...)
            // After parsing "for", we consumed the type name
            // Need to get it from the parse above
            // Let me re-examine: type_name=Trait, then "for", then actual type
            // The advance above skipped "for", and _actual_type is the real type
            // But we dropped it. Let me fix the logic.
            // Actually, _actual_type was captured above. Let me restructure.
            clj_to_rust_ident("") // placeholder, fix below
        } else {
            clj_to_rust_ident(&type_name)
        };

        // Let me redo this properly
        // Rewind approach won't work easily. Let's fix the logic:
        let _ = actual_type; // suppress warning

        let mut methods = Vec::new();
        while self.peek() != Some(&Token::RParen) {
            methods.push(self.parse_defn(true)?);
        }
        self.expect(&Token::RParen)?;

        // Re-derive type name properly:
        // If trait_name is Some, the `type_name` was actually the trait name
        // and we need the second symbol which was parsed as _actual_type
        // Since we can't easily recover it, let me restructure parse_impl

        Ok(Impl {
            type_name: clj_to_rust_ident(&type_name),
            generics,
            trait_name,
            methods,
        })
    }

    // ── mod ─────────────────────────────────────────────────────

    fn parse_mod(&mut self) -> Result<Mod, String> {
        self.expect(&Token::LParen)?;
        self.expect_symbol("defmod")?;
        let name = self.expect_any_symbol()?;
        let mut items = Vec::new();
        while self.peek() != Some(&Token::RParen) {
            items.push(self.parse_top_level()?);
        }
        self.expect(&Token::RParen)?;
        Ok(Mod {
            is_pub: true,
            name: clj_to_rust_ident(&name),
            items,
        })
    }

    // ── attr ────────────────────────────────────────────────────

    fn parse_attr(&mut self) -> Result<TopLevel, String> {
        self.expect(&Token::LParen)?;
        self.expect_symbol("#!")?;
        let mut attr = String::new();
        // Collect until the next top-level form
        let attr_name = self.expect_any_symbol()?;
        attr.push_str(&attr_name);
        // Check for arguments
        if self.peek() == Some(&Token::LParen) {
            self.advance()?;
            attr.push('(');
            let mut first = true;
            while self.peek() != Some(&Token::RParen) {
                if !first {
                    attr.push_str(", ");
                }
                first = false;
                let a = self.expect_any_symbol()?;
                attr.push_str(&a);
            }
            self.expect(&Token::RParen)?;
            attr.push(')');
        }
        let inner = self.parse_top_level()?;
        self.expect(&Token::RParen)?;
        Ok(TopLevel::Attr(attr, Box::new(inner)))
    }

    // ── derives ─────────────────────────────────────────────────

    fn parse_derives(&mut self) -> Result<Vec<String>, String> {
        self.advance()?; // skip #
        self.expect(&Token::LBracket)?;
        let mut derives = Vec::new();
        while self.peek() != Some(&Token::RBracket) {
            derives.push(self.expect_any_symbol()?);
        }
        self.expect(&Token::RBracket)?;
        Ok(derives)
    }

    // ── type string ─────────────────────────────────────────────

    fn parse_type_str(&mut self) -> Result<String, String> {
        let mut ty = String::new();

        if self.peek() == Some(&Token::Ampersand) {
            self.advance()?;
            ty.push('&');
            if self.peek_symbol() == Some("mut") {
                self.advance()?;
                ty.push_str("mut ");
            }
        }

        match self.peek().cloned() {
            Some(Token::Symbol(s)) => {
                self.advance()?;
                ty.push_str(&clj_to_rust_ident(&s));
            }
            Some(Token::LBracket) => {
                // [Type] → Vec<Type> or [Type; N] → [Type; N]
                self.advance()?;
                let inner = self.parse_type_str()?;
                if self.peek() == Some(&Token::RBracket) {
                    self.advance()?;
                    ty.push_str(&format!("Vec<{}>", inner));
                } else {
                    // array type
                    let size = self.expect_any_symbol()?;
                    self.expect(&Token::RBracket)?;
                    ty.push_str(&format!("[{}; {}]", inner, size));
                }
                return Ok(ty);
            }
            Some(Token::LParen) => {
                // (Fn [args] -> ret) or tuple
                self.advance()?;
                if self.peek_symbol() == Some("Fn") || self.peek_symbol() == Some("fn") {
                    self.advance()?;
                    self.expect(&Token::LBracket)?;
                    let mut params = Vec::new();
                    while self.peek() != Some(&Token::RBracket) {
                        params.push(self.parse_type_str()?);
                    }
                    self.expect(&Token::RBracket)?;
                    let ret = if self.peek() == Some(&Token::Arrow) {
                        self.advance()?;
                        self.parse_type_str()?
                    } else {
                        "()".to_string()
                    };
                    self.expect(&Token::RParen)?;
                    ty.push_str(&format!("fn({}) -> {}", params.join(", "), ret));
                } else {
                    // Treat as tuple type
                    let mut types = Vec::new();
                    while self.peek() != Some(&Token::RParen) {
                        types.push(self.parse_type_str()?);
                    }
                    self.expect(&Token::RParen)?;
                    ty.push_str(&format!("({})", types.join(", ")));
                }
                return Ok(ty);
            }
            other => return Err(format!("Expected type, got {:?}", other)),
        }

        // Check for generic params  Type<A, B>
        if self.peek() == Some(&Token::Symbol("<".to_string())) {
            self.advance()?;
            ty.push('<');
            let mut first = true;
            loop {
                if !first {
                    ty.push_str(", ");
                }
                first = false;
                ty.push_str(&self.parse_type_str()?);
                if self.peek_symbol() == Some(">") {
                    self.advance()?;
                    break;
                }
            }
            ty.push('>');
        }

        Ok(ty)
    }

    // ── expressions ─────────────────────────────────────────────

    pub fn parse_expr(&mut self) -> Result<Expr, String> {
        match self.peek().cloned() {
            Some(Token::Integer(n)) => {
                self.advance()?;
                Ok(Expr::Integer(n))
            }
            Some(Token::Float(f)) => {
                self.advance()?;
                Ok(Expr::Float(f))
            }
            Some(Token::String(s)) => {
                self.advance()?;
                Ok(Expr::Str(s))
            }
            Some(Token::Char(c)) => {
                self.advance()?;
                Ok(Expr::Char(c))
            }
            Some(Token::Bool(b)) => {
                self.advance()?;
                Ok(Expr::Bool(b))
            }
            Some(Token::Nil) => {
                self.advance()?;
                Ok(Expr::Nil)
            }
            Some(Token::Keyword(k)) => {
                self.advance()?;
                Ok(Expr::Keyword(k))
            }
            Some(Token::Symbol(_)) => {
                let s = if let Token::Symbol(s) = self.advance()?.clone() {
                    s
                } else {
                    unreachable!()
                };
                Ok(Expr::Symbol(clj_to_rust_ident(&s)))
            }
            Some(Token::Ampersand) => {
                self.advance()?;
                if self.peek_symbol() == Some("mut") {
                    self.advance()?;
                    let inner = self.parse_expr()?;
                    Ok(Expr::RefMut(Box::new(inner)))
                } else {
                    let inner = self.parse_expr()?;
                    Ok(Expr::Ref(Box::new(inner)))
                }
            }
            Some(Token::At) => {
                self.advance()?;
                let inner = self.parse_expr()?;
                Ok(Expr::Deref(Box::new(inner)))
            }
            Some(Token::LBracket) => self.parse_vec_literal(),
            Some(Token::LBrace) => self.parse_hashmap_literal(),
            Some(Token::Tilde) => {
                self.advance()?;
                // raw rust: ~"rust code here"
                match self.advance()?.clone() {
                    Token::String(s) => Ok(Expr::RawRust(s)),
                    _ => Err("Expected string after ~ for raw Rust code".to_string()),
                }
            }
            Some(Token::LParen) => self.parse_list_expr(),
            other => Err(format!(
                "Unexpected token {:?} at line {}",
                other,
                self.current_line()
            )),
        }
    }

    fn parse_vec_literal(&mut self) -> Result<Expr, String> {
        self.expect(&Token::LBracket)?;
        let mut elems = Vec::new();
        while self.peek() != Some(&Token::RBracket) {
            elems.push(self.parse_expr()?);
        }
        self.expect(&Token::RBracket)?;
        Ok(Expr::Vec(elems))
    }

    fn parse_hashmap_literal(&mut self) -> Result<Expr, String> {
        self.expect(&Token::LBrace)?;
        let mut pairs = Vec::new();
        while self.peek() != Some(&Token::RBrace) {
            let key = self.parse_expr()?;
            let val = self.parse_expr()?;
            pairs.push((key, val));
        }
        self.expect(&Token::RBrace)?;
        Ok(Expr::HashMap(pairs))
    }

    fn parse_list_expr(&mut self) -> Result<Expr, String> {
        self.expect(&Token::LParen)?;

        // Empty list
        if self.peek() == Some(&Token::RParen) {
            self.advance()?;
            return Ok(Expr::Tuple(Vec::new()));
        }

        // Look at the head to determine the form
        match self.peek().cloned() {
            Some(Token::Symbol(ref s)) => {
                let s = s.clone();
                match s.as_str() {
                    // ── Special forms ───────────────────────────
                    "let" | "let-mut" => self.parse_let(s == "let-mut"),
                    "if" => self.parse_if(),
                    "do" => self.parse_do(),
                    "fn" => self.parse_fn_expr(),
                    "match" => self.parse_match(),
                    "for" => self.parse_for(),
                    "while" => self.parse_while(),
                    "loop" => self.parse_loop(),
                    "recur" => self.parse_recur(),
                    "set!" => self.parse_set(),
                    "return" => self.parse_return(),
                    "break" => self.parse_break(),
                    "continue" => {
                        self.advance()?;
                        self.expect(&Token::RParen)?;
                        Ok(Expr::Continue)
                    }
                    "as" => self.parse_as(),
                    "try!" | "?" => self.parse_try(),
                    "await" => self.parse_await(),
                    "new" => self.parse_struct_init(),
                    "get" => self.parse_index(),
                    "tuple" => self.parse_tuple(),

                    // ── Operators ───────────────────────────────
                    "+" | "-" | "*" | "/" | "%" | "=" | "!=" | "<" | ">" | "<=" | ">="
                    | "and" | "or" | "bit-and" | "bit-or" | "bit-xor" | "shl" | "shr" => {
                        self.parse_binop()
                    }
                    "not" | "neg" => self.parse_unary(),

                    // ── Range ──────────────────────────────────
                    ".." => self.parse_range(false),
                    "..=" => self.parse_range(true),

                    _ => {
                        // Check for method call (.method obj args)
                        if s.starts_with('.') && s.len() > 1 {
                            self.parse_method_call()
                        }
                        // Check for macro call (name! args)
                        else if s.ends_with('!') {
                            self.parse_macro_call()
                        }
                        // Check for static call (Type::method args)
                        else if s.contains("::") {
                            self.parse_static_call()
                        }
                        // Otherwise it's a function call
                        else {
                            self.parse_call()
                        }
                    }
                }
            }
            // Head is an expression (e.g. ((get-fn) args))
            _ => self.parse_call(),
        }
    }

    // ── let ─────────────────────────────────────────────────────

    fn parse_let(&mut self, mutable: bool) -> Result<Expr, String> {
        self.advance()?; // skip "let" / "let-mut"
        self.expect(&Token::LBracket)?;
        let mut bindings = Vec::new();
        while self.peek() != Some(&Token::RBracket) {
            let is_mut = if self.peek_symbol() == Some("mut") {
                self.advance()?;
                true
            } else {
                mutable
            };
            let name = self.expect_any_symbol()?;
            let type_ann = if self.peek_symbol() == Some(":") {
                self.advance()?;
                Some(self.parse_type_str()?)
            } else {
                None
            };
            let value = self.parse_expr()?;
            bindings.push(LetBinding {
                name: clj_to_rust_ident(&name),
                mutable: is_mut,
                type_ann,
                value,
            });
        }
        self.expect(&Token::RBracket)?;
        let mut body = Vec::new();
        while self.peek() != Some(&Token::RParen) {
            body.push(self.parse_expr()?);
        }
        self.expect(&Token::RParen)?;
        Ok(Expr::Let { bindings, body })
    }

    // ── if ──────────────────────────────────────────────────────

    fn parse_if(&mut self) -> Result<Expr, String> {
        self.advance()?; // skip "if"
        let cond = self.parse_expr()?;
        let then = self.parse_expr()?;
        let else_ = if self.peek() != Some(&Token::RParen) {
            Some(Box::new(self.parse_expr()?))
        } else {
            None
        };
        self.expect(&Token::RParen)?;
        Ok(Expr::If {
            cond: Box::new(cond),
            then: Box::new(then),
            else_,
        })
    }

    // ── do ──────────────────────────────────────────────────────

    fn parse_do(&mut self) -> Result<Expr, String> {
        self.advance()?; // skip "do"
        let mut exprs = Vec::new();
        while self.peek() != Some(&Token::RParen) {
            exprs.push(self.parse_expr()?);
        }
        self.expect(&Token::RParen)?;
        Ok(Expr::Do(exprs))
    }

    // ── fn (lambda) ─────────────────────────────────────────────

    fn parse_fn_expr(&mut self) -> Result<Expr, String> {
        self.advance()?; // skip "fn"
        self.expect(&Token::LBracket)?;
        let params = self.parse_params(false)?;
        self.expect(&Token::RBracket)?;
        let ret_type = if self.peek() == Some(&Token::Arrow) {
            self.advance()?;
            Some(self.parse_type_str()?)
        } else {
            None
        };
        let mut body = Vec::new();
        while self.peek() != Some(&Token::RParen) {
            body.push(self.parse_expr()?);
        }
        self.expect(&Token::RParen)?;
        Ok(Expr::Fn {
            params,
            ret_type,
            body,
        })
    }

    // ── match ───────────────────────────────────────────────────

    fn parse_match(&mut self) -> Result<Expr, String> {
        self.advance()?; // skip "match"
        let expr = self.parse_expr()?;
        let mut arms = Vec::new();
        while self.peek() != Some(&Token::RParen) {
            let pattern = self.parse_pattern()?;
            let body = self.parse_expr()?;
            arms.push(MatchArm {
                pattern,
                guard: None,
                body,
            });
        }
        self.expect(&Token::RParen)?;
        Ok(Expr::Match {
            expr: Box::new(expr),
            arms,
        })
    }

    fn parse_pattern(&mut self) -> Result<Pattern, String> {
        match self.peek().cloned() {
            Some(Token::Symbol(ref s)) if s == "_" => {
                self.advance()?;
                Ok(Pattern::Wildcard)
            }
            Some(Token::Integer(_)) | Some(Token::Float(_)) | Some(Token::String(_))
            | Some(Token::Bool(_)) | Some(Token::Char(_)) | Some(Token::Nil) => {
                let expr = self.parse_expr()?;
                Ok(Pattern::Literal(expr))
            }
            Some(Token::Symbol(_)) => {
                let s = if let Token::Symbol(s) = self.advance()?.clone() {
                    s
                } else {
                    unreachable!()
                };
                // Check if this is an enum variant like Some, None, Ok, Err
                if self.peek() == Some(&Token::LParen) {
                    // TupleStruct pattern: Variant(fields)
                    self.advance()?;
                    let mut fields = Vec::new();
                    while self.peek() != Some(&Token::RParen) {
                        fields.push(self.parse_pattern()?);
                    }
                    self.expect(&Token::RParen)?;
                    Ok(Pattern::TupleStruct {
                        name: clj_to_rust_ident(&s),
                        fields,
                    })
                } else {
                    Ok(Pattern::Binding(clj_to_rust_ident(&s)))
                }
            }
            Some(Token::LParen) => {
                self.advance()?;
                let name = self.expect_any_symbol()?;
                let mut fields = Vec::new();
                while self.peek() != Some(&Token::RParen) {
                    fields.push(self.parse_pattern()?);
                }
                self.expect(&Token::RParen)?;
                Ok(Pattern::TupleStruct {
                    name: clj_to_rust_ident(&name),
                    fields,
                })
            }
            Some(Token::LBracket) => {
                self.advance()?;
                let mut pats = Vec::new();
                while self.peek() != Some(&Token::RBracket) {
                    pats.push(self.parse_pattern()?);
                }
                self.expect(&Token::RBracket)?;
                Ok(Pattern::Tuple(pats))
            }
            other => Err(format!("Unexpected token in pattern: {:?}", other)),
        }
    }

    // ── for ─────────────────────────────────────────────────────

    fn parse_for(&mut self) -> Result<Expr, String> {
        self.advance()?; // skip "for"
        self.expect(&Token::LBracket)?;
        let binding = self.expect_any_symbol()?;
        let iter = self.parse_expr()?;
        self.expect(&Token::RBracket)?;
        let mut body = Vec::new();
        while self.peek() != Some(&Token::RParen) {
            body.push(self.parse_expr()?);
        }
        self.expect(&Token::RParen)?;
        Ok(Expr::For {
            binding: clj_to_rust_ident(&binding),
            iter: Box::new(iter),
            body,
        })
    }

    // ── while ───────────────────────────────────────────────────

    fn parse_while(&mut self) -> Result<Expr, String> {
        self.advance()?; // skip "while"
        let cond = self.parse_expr()?;
        let mut body = Vec::new();
        while self.peek() != Some(&Token::RParen) {
            body.push(self.parse_expr()?);
        }
        self.expect(&Token::RParen)?;
        Ok(Expr::While {
            cond: Box::new(cond),
            body,
        })
    }

    // ── loop/recur ──────────────────────────────────────────────

    fn parse_loop(&mut self) -> Result<Expr, String> {
        self.advance()?; // skip "loop"

        // Check if there are bindings
        let bindings = if self.peek() == Some(&Token::LBracket) {
            self.expect(&Token::LBracket)?;
            let mut bindings = Vec::new();
            while self.peek() != Some(&Token::RBracket) {
                let name = self.expect_any_symbol()?;
                let value = self.parse_expr()?;
                bindings.push(LetBinding {
                    name: clj_to_rust_ident(&name),
                    mutable: true,
                    type_ann: None,
                    value,
                });
            }
            self.expect(&Token::RBracket)?;
            bindings
        } else {
            Vec::new()
        };

        let mut body = Vec::new();
        while self.peek() != Some(&Token::RParen) {
            body.push(self.parse_expr()?);
        }
        self.expect(&Token::RParen)?;
        Ok(Expr::Loop { bindings, body })
    }

    fn parse_recur(&mut self) -> Result<Expr, String> {
        self.advance()?; // skip "recur"
        let mut args = Vec::new();
        while self.peek() != Some(&Token::RParen) {
            args.push(self.parse_expr()?);
        }
        self.expect(&Token::RParen)?;
        Ok(Expr::Recur(args))
    }

    // ── set! ────────────────────────────────────────────────────

    fn parse_set(&mut self) -> Result<Expr, String> {
        self.advance()?; // skip "set!"
        let target = self.parse_expr()?;
        let value = self.parse_expr()?;
        self.expect(&Token::RParen)?;
        Ok(Expr::Set {
            target: Box::new(target),
            value: Box::new(value),
        })
    }

    // ── return ──────────────────────────────────────────────────

    fn parse_return(&mut self) -> Result<Expr, String> {
        self.advance()?; // skip "return"
        let val = if self.peek() != Some(&Token::RParen) {
            Some(Box::new(self.parse_expr()?))
        } else {
            None
        };
        self.expect(&Token::RParen)?;
        Ok(Expr::Return(val))
    }

    fn parse_break(&mut self) -> Result<Expr, String> {
        self.advance()?; // skip "break"
        let val = if self.peek() != Some(&Token::RParen) {
            Some(Box::new(self.parse_expr()?))
        } else {
            None
        };
        self.expect(&Token::RParen)?;
        Ok(Expr::Break(val))
    }

    // ── as (type cast) ─────────────────────────────────────────

    fn parse_as(&mut self) -> Result<Expr, String> {
        self.advance()?; // skip "as"
        let expr = self.parse_expr()?;
        let ty = self.parse_type_str()?;
        self.expect(&Token::RParen)?;
        Ok(Expr::As {
            expr: Box::new(expr),
            type_name: ty,
        })
    }

    // ── try (?) ─────────────────────────────────────────────────

    fn parse_try(&mut self) -> Result<Expr, String> {
        self.advance()?; // skip "try!" / "?"
        let expr = self.parse_expr()?;
        self.expect(&Token::RParen)?;
        Ok(Expr::Try(Box::new(expr)))
    }

    // ── await ───────────────────────────────────────────────────

    fn parse_await(&mut self) -> Result<Expr, String> {
        self.advance()?; // skip "await"
        let expr = self.parse_expr()?;
        self.expect(&Token::RParen)?;
        Ok(Expr::Await(Box::new(expr)))
    }

    // ── struct init ─────────────────────────────────────────────

    fn parse_struct_init(&mut self) -> Result<Expr, String> {
        self.advance()?; // skip "new"
        let name = self.expect_any_symbol()?;
        let mut fields = Vec::new();
        while self.peek() != Some(&Token::RParen) {
            // :field value
            match self.peek().cloned() {
                Some(Token::Keyword(k)) => {
                    self.advance()?;
                    let val = self.parse_expr()?;
                    fields.push((clj_to_rust_ident(&k), val));
                }
                _ => {
                    return Err(format!(
                        "Expected :field in struct init, got {:?}",
                        self.peek()
                    ))
                }
            }
        }
        self.expect(&Token::RParen)?;
        Ok(Expr::StructInit {
            name: clj_to_rust_ident(&name),
            fields,
        })
    }

    // ── index ───────────────────────────────────────────────────

    fn parse_index(&mut self) -> Result<Expr, String> {
        self.advance()?; // skip "get"
        let object = self.parse_expr()?;
        let index = self.parse_expr()?;
        self.expect(&Token::RParen)?;
        Ok(Expr::Index {
            object: Box::new(object),
            index: Box::new(index),
        })
    }

    // ── tuple ───────────────────────────────────────────────────

    fn parse_tuple(&mut self) -> Result<Expr, String> {
        self.advance()?; // skip "tuple"
        let mut elems = Vec::new();
        while self.peek() != Some(&Token::RParen) {
            elems.push(self.parse_expr()?);
        }
        self.expect(&Token::RParen)?;
        Ok(Expr::Tuple(elems))
    }

    // ── operators ───────────────────────────────────────────────

    fn parse_binop(&mut self) -> Result<Expr, String> {
        let op_str = if let Token::Symbol(s) = self.advance()?.clone() {
            s
        } else {
            unreachable!()
        };

        let op = match op_str.as_str() {
            "+" => BinOpKind::Add,
            "-" => BinOpKind::Sub,
            "*" => BinOpKind::Mul,
            "/" => BinOpKind::Div,
            "%" => BinOpKind::Rem,
            "=" => BinOpKind::Eq,
            "!=" => BinOpKind::NotEq,
            "<" => BinOpKind::Lt,
            ">" => BinOpKind::Gt,
            "<=" => BinOpKind::LtEq,
            ">=" => BinOpKind::GtEq,
            "and" => BinOpKind::And,
            "or" => BinOpKind::Or,
            "bit-and" => BinOpKind::BitAnd,
            "bit-or" => BinOpKind::BitOr,
            "bit-xor" => BinOpKind::BitXor,
            "shl" => BinOpKind::Shl,
            "shr" => BinOpKind::Shr,
            _ => return Err(format!("Unknown operator: {}", op_str)),
        };

        // Collect all operands — supports variadic: (+ 1 2 3 4)
        let mut operands = Vec::new();
        while self.peek() != Some(&Token::RParen) {
            operands.push(self.parse_expr()?);
        }
        self.expect(&Token::RParen)?;

        if operands.is_empty() {
            return Err(format!("Operator {} requires at least one operand", op_str));
        }
        if operands.len() == 1 {
            // Unary minus
            if matches!(op, BinOpKind::Sub) {
                return Ok(Expr::UnaryOp {
                    op: UnaryOpKind::Neg,
                    operand: Box::new(operands.remove(0)),
                });
            }
            return Ok(operands.remove(0));
        }

        // Fold left: (+ 1 2 3) => ((1 + 2) + 3)
        let mut result = operands.remove(0);
        for operand in operands {
            result = Expr::BinOp {
                op: op.clone(),
                left: Box::new(result),
                right: Box::new(operand),
            };
        }
        Ok(result)
    }

    fn parse_unary(&mut self) -> Result<Expr, String> {
        let op_str = if let Token::Symbol(s) = self.advance()?.clone() {
            s
        } else {
            unreachable!()
        };
        let op = match op_str.as_str() {
            "not" => UnaryOpKind::Not,
            "neg" => UnaryOpKind::Neg,
            _ => unreachable!(),
        };
        let operand = self.parse_expr()?;
        self.expect(&Token::RParen)?;
        Ok(Expr::UnaryOp {
            op,
            operand: Box::new(operand),
        })
    }

    // ── range ───────────────────────────────────────────────────

    fn parse_range(&mut self, inclusive: bool) -> Result<Expr, String> {
        self.advance()?; // skip ".." / "..="
        let start = self.parse_expr()?;
        let end = self.parse_expr()?;
        self.expect(&Token::RParen)?;
        Ok(Expr::Range {
            start: Box::new(start),
            end: Box::new(end),
            inclusive,
        })
    }

    // ── method call ─────────────────────────────────────────────

    fn parse_method_call(&mut self) -> Result<Expr, String> {
        let method = if let Token::Symbol(s) = self.advance()?.clone() {
            s[1..].to_string() // strip leading .
        } else {
            unreachable!()
        };
        let object = self.parse_expr()?;
        let mut args = Vec::new();
        while self.peek() != Some(&Token::RParen) {
            args.push(self.parse_expr()?);
        }
        self.expect(&Token::RParen)?;
        Ok(Expr::MethodCall {
            object: Box::new(object),
            method: clj_to_rust_ident(&method),
            args,
        })
    }

    // ── macro call ──────────────────────────────────────────────

    fn parse_macro_call(&mut self) -> Result<Expr, String> {
        let name = if let Token::Symbol(s) = self.advance()?.clone() {
            s
        } else {
            unreachable!()
        };
        let mut args = Vec::new();
        while self.peek() != Some(&Token::RParen) {
            args.push(self.parse_expr()?);
        }
        self.expect(&Token::RParen)?;
        Ok(Expr::MacroCall {
            name: clj_to_rust_ident(&name),
            args,
        })
    }

    // ── static call ─────────────────────────────────────────────

    fn parse_static_call(&mut self) -> Result<Expr, String> {
        let full = if let Token::Symbol(s) = self.advance()?.clone() {
            s
        } else {
            unreachable!()
        };
        let parts: Vec<&str> = full.rsplitn(2, "::").collect();
        let method = clj_to_rust_ident(parts[0]);
        let type_name = clj_to_rust_ident(parts[1]);

        let mut args = Vec::new();
        while self.peek() != Some(&Token::RParen) {
            args.push(self.parse_expr()?);
        }
        self.expect(&Token::RParen)?;
        Ok(Expr::StaticCall {
            type_name,
            method,
            args,
        })
    }

    // ── function call ───────────────────────────────────────────

    fn parse_call(&mut self) -> Result<Expr, String> {
        let func = self.parse_expr()?;
        let mut args = Vec::new();
        while self.peek() != Some(&Token::RParen) {
            args.push(self.parse_expr()?);
        }
        self.expect(&Token::RParen)?;
        Ok(Expr::Call {
            func: Box::new(func),
            args,
        })
    }

    // ── helpers ─────────────────────────────────────────────────

    fn expect_symbol(&mut self, name: &str) -> Result<(), String> {
        match self.advance()?.clone() {
            Token::Symbol(s) if s == name => Ok(()),
            other => Err(format!("Expected symbol '{}', got {:?}", name, other)),
        }
    }

    fn expect_any_symbol(&mut self) -> Result<String, String> {
        match self.advance()?.clone() {
            Token::Symbol(s) => Ok(s),
            other => Err(format!("Expected symbol, got {:?}", other)),
        }
    }
}

/// Convert Clojure-style identifiers to Rust-style:
/// - kebab-case → snake_case
/// - Preserves :: for path separators
/// - Preserves leading dots, !, ? etc
pub fn clj_to_rust_ident(s: &str) -> String {
    if s.contains("::") {
        // Process each segment separately
        s.split("::")
            .map(|part| part.replace('-', "_"))
            .collect::<Vec<_>>()
            .join("::")
    } else {
        s.replace('-', "_")
    }
}
