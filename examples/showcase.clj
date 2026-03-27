; ============================================
; cljpro 完整功能展示
; Clojure 语法，编译到 Rust，使用 Rust 全生态
; ============================================

(use std::collections::HashMap)

; ── 常量与静态变量 ─────────────────────────────
(const MAX-SIZE -> i32 100)

; ── 结构体 (自动 derive) ──────────────────────
(defstruct Point #[Debug Clone]
  [pub x : f64
   pub y : f64])

; ── 枚举 ─────────────────────────────────────
(defenum Shape #[Debug]
  (Circle f64)
  (Rect f64 f64)
  Unit)

; ── Trait 定义 ────────────────────────────────
(deftrait Describable
  (defn describe [&self] -> String))

; ── impl 块 ──────────────────────────────────
(defimpl Point
  (defn distance [&self] -> f64
    (.sqrt (+ (* self.x self.x) (* self.y self.y))))

  (defn translate [&self dx : f64 dy : f64] -> Point
    (new Point :x (+ self.x dx) :y (+ self.y dy))))

; ── Trait 实现 ────────────────────────────────
(defimpl Describable for Point
  (defn describe [&self] -> String
    (format! "Point({}, {})" self.x self.y)))

; ── 主函数 ────────────────────────────────────
(defn main []
  ; let 绑定
  (let [p (new Point :x 3.0 :y 4.0)
        dist (.distance &p)]
    (println! "Point: {:?}" p)
    (println! "Distance: {}" dist)
    (println! "Description: {}" (.describe &p)))

  ; 可变绑定
  (let-mut [v (vec! [1 2 3])]
    (.push &mut v 4)
    (.push &mut v 5)
    (println! "Vector: {:?}" v))

  ; 模式匹配
  (let [shape (Shape::Circle 5.0)]
    (match shape
      (Shape::Circle r) (println! "Circle with radius {}" r)
      (Shape::Rect w h) (println! "Rect {}x{}" w h)
      Shape::Unit (println! "Unit shape")))

  ; if 表达式
  (let [x 42
        msg (if (> x 40) "big" "small")]
    (println! "{} is {}" x msg))

  ; for 循环
  (for [i (.. 0 5)]
    (println! "i = {}" i))

  ; loop/recur (→ Rust loop + continue)
  (let [result (loop [n 10 acc 0]
                 (if (= n 0)
                   acc
                   (recur (- n 1) (+ acc n))))]
    (println! "Sum 1..10 = {}" result))

  ; HashMap
  (let [m (HashMap::from [("a" 1) ("b" 2) ("c" 3)])]
    (println! "Map: {:?}" m))

  ; 闭包
  (let [nums (vec! [1 2 3 4 5])
        doubled (.collect (.map (.iter &nums) (fn [x : &i32] -> i32 (* @x 2))))]
    (println! "Doubled: {:?}" doubled))

  ; 方法链
  (let [s (String::from "hello world")]
    (println! "Upper: {}" (.to_uppercase &s))
    (println! "Contains 'world': {}" (.contains &s "world")))

  ; 类型转换
  (let [x (as 42 f64)]
    (println! "42 as f64 = {}" x))

  (println! "Done!"))
