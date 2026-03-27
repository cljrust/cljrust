# cljpro 语法参考

Clojure 语法 → Rust 编译器。用 Clojure 的表达力，享受 Rust 的性能和生态。

## 基本类型

```clojure
42          ; i64 整数
3.14        ; f64 浮点
"hello"     ; 字符串
\a          ; 字符
true false  ; 布尔
nil         ; () 单元类型
:keyword    ; 编译为 "keyword" 字符串
```

## 变量绑定

```clojure
; let 绑定 (不可变)
(let [x 10
      y : i32 20]    ; 可选类型标注
  (+ x y))

; let-mut 绑定 (可变)
(let-mut [count 0]
  (set! count 10))

; 单个可变绑定
(let [mut x 0]
  (set! x 42))

; 顶层常量
(const MAX -> i32 100)

; 顶层定义
(def NAME -> &str "cljpro")
```

## 函数

```clojure
; 公开函数
(defn add [a : i32 b : i32] -> i32
  (+ a b))

; 私有函数
(defn- helper [x : i32] -> i32
  (* x 2))

; 异步函数
(defn-async fetch-data [url : &str] -> Result < String Error >
  (? (do-fetch url)))

; 闭包/匿名函数
(fn [x : i32 y : i32] -> i32 (+ x y))
(fn [x] (* x x))  ; 类型推断
```

## 控制流

```clojure
; if 表达式 (有返回值)
(if (> x 0)
  "positive"
  "non-positive")

; do 块
(do
  (println! "step 1")
  (println! "step 2")
  42)  ; 最后一个表达式是返回值

; match 模式匹配
(match value
  1 "one"
  2 "two"
  _ "other")

; match 枚举
(match option
  (Some x) (println! "Got {}" x)
  None (println! "Nothing"))

; for 循环
(for [i (.. 0 10)]
  (println! "{}" i))

; while 循环
(while (> n 0)
  (set! n (- n 1)))

; loop/recur (尾递归优化)
(loop [n 10 acc 0]
  (if (= n 0)
    acc
    (recur (- n 1) (+ acc n))))
```

## 运算符

```clojure
; 算术 (支持多参数)
(+ 1 2 3)        ; → (1 + 2) + 3
(- 10 3)         ; → 10 - 3
(* 2 3 4)        ; → (2 * 3) * 4
(/ 10 2)         ; → 10 / 2
(% 10 3)         ; → 10 % 3

; 比较
(= a b)          ; → a == b
(!= a b)         ; → a != b
(< a b) (> a b)  ; → a < b, a > b
(<= a b) (>= a b)

; 逻辑
(and a b)        ; → a && b
(or a b)         ; → a || b
(not x)          ; → !x

; 位运算
(bit-and a b)    ; → a & b
(bit-or a b)     ; → a | b
(bit-xor a b)    ; → a ^ b
(shl a b)        ; → a << b
(shr a b)        ; → a >> b
```

## 集合

```clojure
; Vec
[1 2 3 4 5]              ; → vec![1, 2, 3, 4, 5]

; HashMap
{"key" 1 "other" 2}      ; → HashMap::from([...])
{:a 1 :b 2}              ; → HashMap::from([("a", 1), ("b", 2)])

; 元组
(tuple 1 "hello" true)   ; → (1, "hello", true)
```

## Rust 互操作

```clojure
; 方法调用: (.method object args...)
(.push &mut v 42)        ; → v.push(42)
(.len &s)                ; → s.len()
(.to_uppercase &s)       ; → s.to_uppercase()

; 静态方法: (Type::method args...)
(String::from "hello")   ; → String::from("hello")
(Vec::new)               ; → Vec::new()

; 宏调用: (name! args...)
(println! "Hello {}" name)  ; → println!("Hello {}", name)
(vec! [1 2 3])              ; → vec![1, 2, 3]
(format! "{:?}" x)          ; → format!("{:?}", x)

; use 导入
(use std::collections::HashMap)
(use std::io::{self Read Write})

; 引用
&x                        ; → &x
&mut x                    ; → &mut x
@x                        ; → *x (解引用)

; 类型转换
(as 42 f64)              ; → (42 as f64)

; ? 错误传播
(? (some-fallible-call))  ; → some_fallible_call()?
(try! (some-call))        ; 同上

; .await
(await (fetch url))       ; → fetch(url).await

; 范围
(.. 0 10)                 ; → 0..10
(..= 0 10)               ; → 0..=10

; 索引
(get vec 0)               ; → vec[0]

; 原始 Rust 代码逃逸
~"unsafe { *ptr }"        ; 直接插入 Rust 代码
```

## 结构体

```clojure
; 定义 (带 derive)
(defstruct Point #[Debug Clone PartialEq]
  [pub x : f64
   pub y : f64])

; 创建实例
(new Point :x 3.0 :y 4.0)   ; → Point { x: 3.0, y: 4.0 }
```

## 枚举

```clojure
(defenum Color #[Debug]
  Red
  Green
  Blue
  (Custom u8 u8 u8))

; 使用
Color::Red
(Color::Custom 255 128 0)
```

## Trait

```clojure
; 定义 trait
(deftrait Drawable
  (defn draw [&self])
  (defn area [&self] -> f64))

; 实现 trait
(defimpl Drawable for Circle
  (defn draw [&self]
    (println! "Drawing circle"))
  (defn area [&self] -> f64
    (* 3.14159 self.radius self.radius)))
```

## Impl 块

```clojure
(defimpl Point
  (defn new [x : f64 y : f64] -> Point
    (new Point :x x :y y))

  (defn distance [&self] -> f64
    (.sqrt (+ (* self.x self.x) (* self.y self.y)))))
```

## 完整示例

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

## 命名约定

cljpro 自动将 Clojure 的 kebab-case 转为 Rust 的 snake_case:

```
my-function  →  my_function
get-value    →  get_value
is-valid?    →  is_valid?
set!         →  set!
```
