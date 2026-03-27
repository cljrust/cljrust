; ============================================
; cljpro 示例: Rust 生态无缝互操作
; 展示如何直接使用 Rust 标准库和类型系统
; ============================================

(use std::collections::HashMap)
(use std::collections::HashSet)
(use std::io)

; ── 使用 Rust 泛型结构体 ─────────────────────
(defstruct Stack #[Debug]
  [pub items : Vec < i32 >])

(defimpl Stack
  (defn new [] -> Stack
    (new Stack :items (Vec::new)))

  (defn push [&mut self val : i32]
    (.push &mut self.items val))

  (defn pop [&mut self] -> Option < i32 >
    (.pop &mut self.items))

  (defn is-empty [&self] -> bool
    (.is_empty &self.items)))

; ── Result 和错误处理 ─────────────────────────
(defn parse-number [s : &str] -> Result < i32 String >
  (match (.parse s)
    (Ok n) (Ok n)
    (Err e) (Err (.to_string &e))))

; ── Option 处理 ──────────────────────────────
(defn safe-divide [a : f64 b : f64] -> Option < f64 >
  (if (= b 0.0)
    None
    (Some (/ a b))))

; ── 迭代器和函数式操作 ───────────────────────
(defn sum-of-squares [nums : &Vec < i32 >] -> i32
  (.sum
    (.map
      (.iter nums)
      (fn [x : &i32] -> i32 (* @x @x)))))

(defn main []
  ; Stack 使用
  (let-mut [stack (Stack::new)]
    (.push &mut stack 10)
    (.push &mut stack 20)
    (.push &mut stack 30)
    (println! "Stack: {:?}" stack)
    (println! "Pop: {:?}" (.pop &mut stack))
    (println! "Stack after pop: {:?}" stack))

  ; Result 处理
  (match (parse-number "42")
    (Ok n) (println! "Parsed: {}" n)
    (Err e) (println! "Error: {}" e))

  (match (parse-number "abc")
    (Ok n) (println! "Parsed: {}" n)
    (Err e) (println! "Error: {}" e))

  ; Option 处理
  (match (safe-divide 10.0 3.0)
    (Some result) (println! "10 / 3 = {}" result)
    None (println! "Division by zero!"))

  (match (safe-divide 10.0 0.0)
    (Some result) (println! "10 / 0 = {}" result)
    None (println! "Division by zero!"))

  ; 函数式编程
  (let [nums (vec! [1 2 3 4 5])]
    (println! "Sum of squares: {}" (sum-of-squares &nums)))

  ; HashSet
  (let-mut [set (HashSet::new)]
    (.insert &mut set "hello")
    (.insert &mut set "world")
    (.insert &mut set "hello")  ; 重复的会被忽略
    (println! "Set size: {}" (.len &set)))

  ; String 操作
  (let-mut [s (String::from "Hello")]
    (.push_str &mut s " World")
    (println! "String: {}" s)
    (println! "Length: {}" (.len &s))
    (println! "Bytes: {}" (.len (.as_bytes &s)))))
