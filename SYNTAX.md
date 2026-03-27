# cljrust Syntax Reference

Clojure syntax → Rust compiler. The expressiveness of Clojure with the performance and ecosystem of Rust.

## Primitive Types

```clojure
42          ; i64 integer
3.14        ; f64 float
"hello"     ; string
\a          ; char
true false  ; bool
nil         ; () unit type
:keyword    ; compiles to "keyword" string
```

## Variable Bindings

```clojure
; let binding (immutable)
(let [x 10
      y : i32 20]    ; optional type annotation
  (+ x y))

; let-mut binding (mutable)
(let-mut [count 0]
  (set! count 10))

; single mutable binding inside let
(let [mut x 0]
  (set! x 42))

; top-level constant
(const MAX -> i32 100)

; top-level static
(def NAME -> &str "cljrust")
```

## Functions

```clojure
; public function
(defn add [a : i32 b : i32] -> i32
  (+ a b))

; private function
(defn- helper [x : i32] -> i32
  (* x 2))

; async function
(defn-async fetch-data [url : &str] -> Result < String Error >
  (? (do-fetch url)))

; closure / anonymous function
(fn [x : i32 y : i32] -> i32 (+ x y))
(fn [x] (* x x))  ; type inference
```

## Control Flow

```clojure
; if expression (returns a value)
(if (> x 0)
  "positive"
  "non-positive")

; do block
(do
  (println! "step 1")
  (println! "step 2")
  42)  ; last expression is return value

; match pattern matching
(match value
  1 "one"
  2 "two"
  _ "other")

; match enum
(match option
  (Some x) (println! "Got {}" x)
  None (println! "Nothing"))

; for loop
(for [i (.. 0 10)]
  (println! "{}" i))

; while loop
(while (> n 0)
  (set! n (- n 1)))

; loop/recur (tail recursion optimization)
(loop [n 10 acc 0]
  (if (= n 0)
    acc
    (recur (- n 1) (+ acc n))))
```

## Operators

```clojure
; arithmetic (variadic)
(+ 1 2 3)        ; → (1 + 2) + 3
(- 10 3)         ; → 10 - 3
(* 2 3 4)        ; → (2 * 3) * 4
(/ 10 2)         ; → 10 / 2
(% 10 3)         ; → 10 % 3

; comparison
(= a b)          ; → a == b
(!= a b)         ; → a != b
(< a b) (> a b)  ; → a < b, a > b
(<= a b) (>= a b)

; logical
(and a b)        ; → a && b
(or a b)         ; → a || b
(not x)          ; → !x

; bitwise
(bit-and a b)    ; → a & b
(bit-or a b)     ; → a | b
(bit-xor a b)    ; → a ^ b
(shl a b)        ; → a << b
(shr a b)        ; → a >> b
```

## Collections

```clojure
; Vec
[1 2 3 4 5]              ; → vec![1, 2, 3, 4, 5]

; HashMap
{"key" 1 "other" 2}      ; → HashMap::from([...])
{:a 1 :b 2}              ; → HashMap::from([("a", 1), ("b", 2)])

; Tuple
(tuple 1 "hello" true)   ; → (1, "hello", true)
```

## Rust Interop

```clojure
; method call: (.method object args...)
(.push &mut v 42)        ; → v.push(42)
(.len &s)                ; → s.len()
(.to_uppercase &s)       ; → s.to_uppercase()

; static method: (Type::method args...)
(String::from "hello")   ; → String::from("hello")
(Vec::new)               ; → Vec::new()

; macro invocation: (name! args...)
(println! "Hello {}" name)  ; → println!("Hello {}", name)
(vec! [1 2 3])              ; → vec![1, 2, 3]
(format! "{:?}" x)          ; → format!("{:?}", x)

; use imports
(use std::collections::HashMap)
(use std::io::{self Read Write})

; references
&x                        ; → &x
&mut x                    ; → &mut x
@x                        ; → *x (deref)

; type cast
(as 42 f64)              ; → (42 as f64)

; ? error propagation
(? (some-fallible-call))  ; → some_fallible_call()?
(try! (some-call))        ; same

; .await
(await (fetch url))       ; → fetch(url).await

; range
(.. 0 10)                 ; → 0..10
(..= 0 10)               ; → 0..=10

; index
(get vec 0)               ; → vec[0]

; raw Rust escape hatch
~"unsafe { *ptr }"        ; inserts raw Rust code
```

## Structs

```clojure
; definition (with derive)
(defstruct Point #[Debug Clone PartialEq]
  [pub x : f64
   pub y : f64])

; instantiation
(new Point :x 3.0 :y 4.0)   ; → Point { x: 3.0, y: 4.0 }
```

## Enums

```clojure
(defenum Color #[Debug]
  Red
  Green
  Blue
  (Custom u8 u8 u8))

; usage
Color::Red
(Color::Custom 255 128 0)
```

## Traits

```clojure
; define trait
(deftrait Drawable
  (defn draw [&self])
  (defn area [&self] -> f64))

; implement trait
(defimpl Drawable for Circle
  (defn draw [&self]
    (println! "Drawing circle"))
  (defn area [&self] -> f64
    (* 3.14159 self.radius self.radius)))
```

## Impl Blocks

```clojure
(defimpl Point
  (defn new [x : f64 y : f64] -> Point
    (new Point :x x :y y))

  (defn distance [&self] -> f64
    (.sqrt (+ (* self.x self.x) (* self.y self.y)))))
```

## Full Example

```clojure
(use std::collections::HashMap)

(defstruct Config #[Debug Clone]
  [pub name : String
   pub values : HashMap < String i32 >])

(defimpl Config
  (defn new [name : String] -> Config
    (new Config :name name :values (HashMap::new)))

  (defn set [&mut self key : String value : i32]
    (.insert &mut self.values key value))

  (defn get [&self key : &str] -> Option < &i32 >
    (.get &self.values key)))

(defn main []
  (let-mut [cfg (Config::new (String::from "myapp"))]
    (.set &mut cfg (String::from "timeout") 30)
    (.set &mut cfg (String::from "retries") 3)
    (println! "Config: {:?}" cfg)
    (match (.get &cfg "timeout")
      (Some v) (println! "timeout = {}" v)
      None (println! "not found"))))
```

## Naming Conventions

cljrust automatically converts Clojure's kebab-case to Rust's snake_case:

```
my-function  →  my_function
get-value    →  get_value
is-valid?    →  is_valid?
set!         →  set!
```
