# cljrust

A parasitic language compiler that brings Clojure's syntax to Rust. Write in Clojure S-expressions, compile to native Rust code, and seamlessly use the entire Rust ecosystem.

## Why cljrust?

- **Clojure's elegance**: S-expressions, immutability-first, prefix notation, minimal syntax
- **Rust's power**: Zero-cost abstractions, memory safety, fearless concurrency
- **Full interop**: Call any Rust function, use any crate, implement traits, define structs and enums
- **Zero runtime**: Compiles to plain Rust source code — no runtime, no overhead, no FFI

## Quick Start

```bash
# Build the compiler => ./target/release/cljrust Add to $PATH
cargo build --release

# Try the interactive REPL
cljrust repl

# See generated Rust code
cljrust emit examples/hello.cljr

# Compile and run directly
cljrust run examples/hello.cljr

# Create a new project
cljrust new my-app
```

## Hello World

```clojure
; hello.cljr
(defn main []
  (println! "Hello from cljrust!"))
```

## Feature Overview

### Functions & Closures

```clojure
(defn add [a : i32 b : i32] -> i32
  (+ a b))

(defn- private-helper [x : i32] -> i32
  (* x 2))

(let [square (fn [x : i32] -> i32 (* x x))]
  (println! "{}" (square 5)))
```

### Structs, Enums & Traits

```clojure
(defstruct Point #[Debug Clone]
  [pub x : f64
   pub y : f64])

(defenum Shape #[Debug]
  (Circle f64)
  (Rect f64 f64)
  Unit)

(deftrait Area
  (defn area [&self] -> f64))

(defimpl Area for Shape
  (defn area [&self] -> f64
    (match self
      (Shape::Circle r) (* 3.14159 r r)
      (Shape::Rect w h) (* w h)
      Shape::Unit 0.0)))
```

### Rust Interop

```clojure
; Method calls
(.push &mut vec 42)          ; → vec.push(42)
(.len &string)               ; → string.len()

; Static methods
(String::from "hello")       ; → String::from("hello")
(Vec::new)                   ; → Vec::new()

; Macros
(println! "x = {}" x)        ; → println!("x = {}", x)
(vec! [1 2 3])               ; → vec![1, 2, 3]

; Struct instantiation
(new Point :x 3.0 :y 4.0)   ; → Point { x: 3.0, y: 4.0 }
```

### Pattern Matching

```clojure
(match value
  (Some x) (println! "Got {}" x)
  None     (println! "Nothing"))

(match shape
  (Shape::Circle r) (* 3.14 r r)
  (Shape::Rect w h) (* w h)
  _ 0.0)
```

### Loop/Recur (Tail Recursion)

```clojure
(defn factorial [n : u64] -> u64
  (loop [i n acc 1]
    (if (= i 0)
      acc
      (recur (- i 1) (* acc i)))))
```

Compiles to an efficient Rust `loop` with mutable variables and `continue` — no stack overflow.

### Error Handling

```clojure
; Result and Option work natively
(defn parse [s : &str] -> Result < i32 String >
  (match (.parse s)
    (Ok n)  (Ok n)
    (Err e) (Err (.to_string &e))))

; ? operator
(defn read-config [path : &str] -> Result < String std::io::Error >
  (? (std::fs::read_to_string path)))
```

### Collections

```clojure
; Vec literal
[1 2 3 4 5]

; HashMap literal
{"name" "Alice" "age" 30}

; Iterators & functional ops
(let [nums (vec! [1 2 3 4 5])
      sum (.sum (.filter (.map (.iter &nums)
                  (fn [x : &i32] -> i32 (* @x @x)))
                  (fn [x : &i32] -> bool (> @x 5))))]
  (println! "Sum of squares > 5: {}" sum))
```

### References & Mutability

```clojure
; Immutable reference
&value

; Mutable reference
&mut value

; Dereference
@value               ; → *value

; Mutable bindings
(let-mut [count 0]
  (set! count (+ count 1)))
```

## Syntax Mapping

| cljrust | Rust |
|---------|------|
| `(defn f [x : i32] -> i32 body)` | `fn f(x: i32) -> i32 { body }` |
| `(let [x 1] body)` | `{ let x = 1; body }` |
| `(let-mut [x 0] ...)` | `{ let mut x = 0; ... }` |
| `(if cond a b)` | `if cond { a } else { b }` |
| `(match e pat1 body1 ...)` | `match e { pat1 => body1, ... }` |
| `(.method obj args)` | `obj.method(args)` |
| `(Type::method args)` | `Type::method(args)` |
| `(macro! args)` | `macro!(args)` |
| `(+ a b c)` | `((a + b) + c)` |
| `[1 2 3]` | `vec![1, 2, 3]` |
| `{"k" v}` | `HashMap::from([("k", v)])` |
| `(new S :f v)` | `S { f: v }` |
| `(loop [x 0] (recur (+ x 1)))` | `{ let mut x = 0; loop { ... continue; } }` |
| `(for [i (.. 0 10)] body)` | `for i in 0..10 { body }` |
| `(as x f64)` | `(x as f64)` |
| `(? expr)` | `expr?` |
| `(await expr)` | `expr.await` |
| `~"raw rust"` | `raw rust` |

## Naming Conventions

cljrust automatically converts Clojure's kebab-case to Rust's snake_case:

```
my-function  →  my_function
get-value    →  get_value
is-valid?    →  is_valid?
```

## Interactive REPL

The REPL compiles and executes each expression in real time — perfect for demos, learning, and quick experiments.

```
$ cljrust repl

cljrust REPL v0.1.0
Clojure syntax → Rust | Type expressions to evaluate

cljr> (+ 1 2 3)
  => 6

cljr> (defstruct Point #[Debug Clone]
  ...   [pub x : f64
  ...    pub y : f64])
  => defined: Point

cljr> (defimpl Point
  ...   (defn distance [&self] -> f64
  ...     (.sqrt (+ (* self.x self.x) (* self.y self.y)))))
  => defined: Point

cljr> (let [p (new Point :x 3.0 :y 4.0)]
  ...   (.distance &p))
  => 5.0

cljr> (defn factorial [n : u64] -> u64
  ...   (loop [i n acc 1]
  ...     (if (= i 0) acc (recur (- i 1) (* acc i)))))
  => defined: factorial

cljr> (factorial 20)
  => 2432902008176640000
```

### REPL Features

- **Stateful**: definitions (defn, defstruct, defenum, deftrait, defimpl) accumulate across inputs
- **Multi-line**: keep typing until parentheses are balanced — auto-detects continuation
- **Compile & execute**: expressions are compiled to Rust and run natively — not interpreted
- **Result display**: expression results are printed via `Debug` formatting
- **Pre-imported**: `HashMap` and `HashSet` available out of the box

### REPL Commands

| Command | Description |
|---------|-------------|
| `:help` | Show help and examples |
| `:rust` | Toggle showing generated Rust code |
| `:defs` | Show all accumulated definitions |
| `:clear` | Clear all accumulated state |
| `:history` | Show input history |
| `:load <file>` | Load and evaluate a `.cljr` file |
| `:last-rs` | Show the full Rust code from last evaluation |
| `:quit` | Exit the REPL |

## CLI Commands

```
cljrust new <name>           Create a new project (Cargo project structure)
cljrust emit <file.cljr>     Print generated Rust to stdout
cljrust run <file.cljr>      Compile and run immediately
cljrust compile <file.cljr>  Compile to .rs or binary
  --emit rust                  Output Rust source (default)
  --emit binary                Compile to binary via rustc
  -o <output>                  Output file path
cljrust repl                  Interactive REPL (compile & execute)
```

## Using Rust Crates

cljrust generates standard Rust source code, so you can use any crate from crates.io:

1. Create a project: `cljrust new my-app`
2. Add dependencies to `Cargo.toml`
3. Write your code in `src/main.cljr`
4. Compile to Rust: `cljrust compile src/main.cljr -o src/main.rs`
5. Build with Cargo: `cargo build`

```toml
# Cargo.toml
[dependencies]
serde = { version = "1", features = ["derive"] }
tokio = { version = "1", features = ["full"] }
```

```clojure
; src/main.cljr — use any Rust crate
(use serde::{Serialize Deserialize})

(defstruct User #[Debug Serialize Deserialize]
  [pub name : String
   pub age : u32])

(defn main []
  (let [user (new User :name (String::from "Alice") :age 30)
        json (serde_json::to_string &user)]
    (println! "{:?}" json)))
```

## Project Structure

```
cljrust/
  Cargo.toml        # Rust project config
  src/
    main.rs          # CLI entry point
    ast.rs           # AST type definitions
    lexer.rs         # Tokenizer
    parser.rs        # S-expr → typed AST
    codegen.rs       # AST → Rust source code
    repl.rs          # Interactive REPL (compile & execute)
  examples/
    hello.cljr       # Hello World
    fibonacci.cljr   # loop/recur tail recursion
    showcase.cljr    # Full feature demo
    rust_interop.cljr # Rust ecosystem interop
  SYNTAX.md          # Complete syntax reference
```

## License

MIT
