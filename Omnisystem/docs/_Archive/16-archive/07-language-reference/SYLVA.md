# Sylva Language Reference

**Sylva** is the interactive scripting and data-science language of the Omnisystem. It sits above Aether in the language stack, designed for the contexts where the feedback loop matters as much as performance: exploratory data analysis, rapid prototyping, notebook environments, and interactive shells.

Sylva is **gradually typed**: code without annotations runs immediately; annotations are added incrementally as invariants are discovered. It supports **time-travel debugging** — you can rewind execution, change a value, and replay forward to see how results diverge. It calls Titan functions at zero overhead and spawns Aether actors transparently.

---

## 1. Design Principles

1. **Immediate feedback.** Every expression is evaluable in the REPL. No compilation step required for exploration.
2. **Gradual typing.** Start untyped; add `Type` annotations as your understanding of the domain firms up. The runtime checks annotations at boundary crossings.
3. **Time travel.** The execution trace is first-class. Rewind, modify a variable, replay — divergence is shown automatically.
4. **Cross-language transparency.** Calling a Titan function looks identical to calling a Sylva function. Spawning an Aether actor looks like a local call.
5. **Data first.** DataFrames, matrices, and lists are first-class types with built-in statistical operations.

---

## 2. Lexical Structure

### 2.1 Comments

```sylva
// Single-line comment
```

### 2.2 Identifiers

Begin with a letter or underscore, contain letters, digits, and underscores. Case-sensitive.

### 2.3 Literals

```sylva
42              // integer
3.14            // float
"hello"         // string (double-quoted, UTF-8)
'x'             // character
true   false    // boolean
()              // unit
[1, 2, 3]       // list literal
```

---

## 3. Variables

### 3.1 Bindings

```sylva
let x = 42;
let name = "Alice";
let items = [1, 2, 3];
```

`let` creates an immutable binding. Attempting to reassign produces a runtime error (or compile error with type annotations).

### 3.2 Mutable Variables

```sylva
let mut counter = 0;
counter = counter + 1;
```

### 3.3 Type Annotations

Optional. When present, the runtime enforces the annotation at assignment:

```sylva
let n: i64 = 42;
let s: str = "hello";
let xs: [f64] = [1.0, 2.0, 3.0];
```

---

## 4. Functions

### 4.1 Definition

```sylva
// Standard syntax (same as Titan subset)
fn add(x: i64, y: i64) -> i64 {
    return x + y;
}

// Concise expression form
fn double(x) = x * 2;

// No-annotation form (gradual)
fn greet(name) {
    print("Hello, " + name);
}
```

### 4.2 Calling Functions

```sylva
let result = add(3, 4);
let d = double(21);
greet("World");
```

### 4.3 Return Values

```sylva
fn sign(x: i64) -> i64 {
    if x > 0 { return 1; }
    if x < 0 { return -1; }
    return 0;
}
```

### 4.4 First-Class Functions

Functions are values. They can be passed as arguments and returned:

```sylva
fn apply(f, x) {
    return f(x);
}

let result = apply(double, 5);    // 10
```

### 4.5 Closures

```sylva
let multiplier = 3;
let triple = fn(x) { return x * multiplier; };
triple(7);    // 21
```

---

## 5. Types

Sylva's type system is gradual. All expressions have a type at runtime; compile-time checking is opt-in via annotations.

### 5.1 Primitive Types

```
i64     // 64-bit integer (default integer type)
f64     // 64-bit float (default float type)
bool    // true or false
str     // UTF-8 string
()      // unit (no value)
?       // unknown/gradual (no annotation — any type accepted at runtime)
```

### 5.2 Collection Types

```sylva
[i64]           // list of i64
[str]           // list of strings
(i64, str)      // tuple
DataFrame       // tabular data (rows × columns)
```

### 5.3 Optional Types

```sylva
let maybe: i64? = some_fn();     // may return a value or null
if maybe != () {
    use_value(maybe);
}
```

### 5.4 Type Compatibility

Sylva uses structural subtyping for gradual checking: a value of type `?` is compatible with any typed binding. Type errors surface at the boundary where annotated and unannotated code meets:

```sylva
fn typed_fn(x: i64) -> i64 { return x * 2; }

let untyped_val = "oops";
typed_fn(untyped_val);    // runtime type error: expected i64, got str
```

---

## 6. Operators

### 6.1 Arithmetic

```sylva
3 + 4     // 7
10 - 3    // 7
4 * 5     // 20
10 / 3    // 3 (integer division for i64)
10 % 3    // 1
```

### 6.2 String Concatenation

```sylva
"hello" + " " + "world"    // "hello world"
"count: " + str(42)        // "count: 42"
```

### 6.3 Comparison

```sylva
x == y    x != y    x < y    x > y    x <= y    x >= y
```

### 6.4 Logical

```sylva
a && b    a || b    !a
```

### 6.5 List Operations

```sylva
let a = [1, 2, 3];
let b = [4, 5, 6];
let c = a + b;        // concatenation: [1, 2, 3, 4, 5, 6]
let head = a[0];      // index: 1
let len = len(a);     // 3
```

---

## 7. Control Flow

### 7.1 If / Else

```sylva
if x > 0 {
    print("positive");
} else if x < 0 {
    print("negative");
} else {
    print("zero");
}
```

If as an expression:

```sylva
let label = if x > 0 { "positive" } else { "non-positive" };
```

### 7.2 While Loop

```sylva
let mut i = 0;
while i < 10 {
    print(i);
    i = i + 1;
}
```

### 7.3 For Loop

Iterate over any list or range:

```sylva
for item in [1, 2, 3, 4, 5] {
    print(item);
}

for i in 0..10 {
    print(i);    // 0 through 9
}

for row in data_rows {
    let total = row[0] + row[1] + row[2];
    if total > 5 {
        print("row exceeds threshold: " + str(total));
    }
}
```

### 7.4 Pattern Matching (case / of)

```sylva
case result of
    | Ok(value) => {
        print("success: " + str(value));
    }
    | Err(msg) => {
        print("error: " + msg);
    };

// Match on values
case x of
    | 0 => print("zero")
    | 1 => print("one")
    | _ => print("other: " + str(x));
```

### 7.5 Break and Continue

```sylva
let mut i = 0;
while true {
    if i >= 10 { break; }
    if i % 2 == 0 { i = i + 1; continue; }
    print(i);
    i = i + 1;
}
```

---

## 8. Built-in Functions

### 8.1 I/O

```sylva
print("hello")                  // print to stdout with newline
print("a", "b", "c")           // multiple args, space-separated
let line = read_line()          // read one line from stdin
```

### 8.2 Type Conversion

```sylva
str(42)           // "42"
str(3.14)         // "3.14"
str(true)         // "true"
int("42")         // 42
float("3.14")     // 3.14
bool(1)           // true
```

### 8.3 Collection Operations

```sylva
len([1, 2, 3])              // 3
len("hello")                // 5

append(list, item)          // returns new list with item appended
prepend(item, list)         // returns new list with item at front
reverse(list)               // reverses a list
sort(list)                  // returns sorted copy
filter(predicate, list)     // returns elements where predicate is true
map(fn, list)               // applies fn to each element
reduce(fn, list, init)      // left fold

first(list)                 // first element or ()
last(list)                  // last element or ()
slice(list, start, end)     // sub-list
```

### 8.4 Math

```sylva
abs(x)       min(a, b)    max(a, b)
floor(x)     ceil(x)      round(x)
sqrt(x)      pow(x, n)
sin(x)       cos(x)       tan(x)
log(x)       log2(x)      log10(x)
```

### 8.5 String Operations

```sylva
len("hello")           // 5
upper("hello")         // "HELLO"
lower("HELLO")         // "hello"
trim("  hi  ")         // "hi"
split("a,b,c", ",")   // ["a", "b", "c"]
join(["a","b"], ",")   // "a,b"
contains("hello", "ell")  // true
starts_with("hello", "he")  // true
ends_with("hello", "lo")    // true
replace("hello", "l", "r")  // "herro"
```

### 8.6 DataFrame Operations

```sylva
let df = DataFrame::from_csv("data.csv")

df.shape()                          // (rows, cols)
df.columns()                        // ["col1", "col2", ...]
df.describe()                       // statistical summary
df.head(n)                          // first n rows
df.tail(n)                          // last n rows
df.select(["col1", "col2"])         // column subset
df.filter(fn(row) { row.age > 18 }) // row filter
df.sort_by("age")                   // sort by column
df.group_by("category")             // grouping
df.mean("value")                    // column mean
df.sum("value")                     // column sum
df.join(other_df, on: "id")        // join two DataFrames
```

---

## 9. Cross-Language Interoperability

### 9.1 Calling Titan Functions

Titan modules are imported and called directly. The call syntax is identical to calling Sylva functions:

```sylva
// Import a content-addressed Titan module
import titan::math                          // by module name
import 2aa62a0a1e1b1fe3 as math            // by content hash

// Call Titan functions — identical syntax to Sylva calls
let result = math::add_nums(10, 20)        // 30
let fib = math::fibonacci(20)              // 6765
```

Titan's ownership model is transparent at the boundary. Primitive types (`i64`, `f64`, `bool`, `str`) are passed by value. Structs are copied.

### 9.2 Spawning Aether Actors

```sylva
// Spawn a local actor
let counter = spawn_actor("aether://CounterActor")

// Send a message
counter.send({ type: "Increment", by: 5 })

// Request-reply
let val = counter.ask({ type: "GetValue" })
print("Current count: " + str(val))
```

### 9.3 DHT Registry

```sylva
// Publish a Sylva module globally
let hash = build::publish("my-module", module_bytes, lang: "sylva")

// Import by hash from any node
import hash as remote_module
```

---

## 10. The REPL

Start the Sylva REPL:

```bash
build run --repl
# or
build repl
```

### 10.1 Basic Usage

```
sylva> 1 + 1
2
sylva> let x = 42
sylva> x * 2
84
sylva> fn double(n) { return n * 2; }
sylva> double(21)
42
```

### 10.2 Loading Files

```
sylva> :load myfile.sy
sylva> :load titan/math.ti     // load a Titan module
```

### 10.3 Time-Travel Commands

The REPL records every expression evaluation. You can navigate the history:

| Command | Effect |
|---------|--------|
| `:trace` | Show the full execution history |
| `:rewind N` | Jump back N steps in history |
| `:step` | Advance one step forward |
| `:replay` | Re-evaluate from current point; show what changed |
| `:rewind 0` | Reset to the beginning |

**Example workflow:**

```
sylva> let data = [10, 20, 30, 40, 50]
sylva> let threshold = 25
sylva> let filtered = filter(fn(x) { x > threshold }, data)
sylva> filtered
[30, 40, 50]
sylva> :rewind 1          // go back to before `filtered` was computed
sylva> threshold = 35     // change the threshold
sylva> :replay            // re-evaluate filtered
[40, 50]                  // shows what changed
```

### 10.4 Breakpoints

```
sylva> :break fn_name         // break at entry of function
sylva> :break 42              // break at line 42
sylva> :break if x > 10       // conditional break
sylva> :continue              // resume from breakpoint
sylva> :watch x               // print value of x whenever it changes
```

---

## 11. Modules

### 11.1 Defining a Module

Any `.sy` file is a module. Functions defined at the top level are exported:

```sylva
// math_utils.sy

fn square(x: f64) -> f64 {
    return x * x;
}

fn cube(x: f64) -> f64 {
    return x * x * x;
}
```

### 11.2 Importing Modules

```sylva
import math_utils
let s = math_utils::square(5.0)

// Import specific names
import math_utils::{ square, cube }
let s = square(5.0)

// Import all (use cautiously in larger programs)
import math_utils::*
```

---

## 12. Error Handling

### 12.1 Result Type

Sylva uses the same `Result<T, E>` type as Titan for functions that may fail:

```sylva
fn safe_divide(a: i64, b: i64) -> Result<i64, str> {
    if b == 0 {
        return Err("division by zero");
    }
    return Ok(a / b);
}

let result = safe_divide(10, 2);
case result of
    | Ok(val) => print("Result: " + str(val))
    | Err(msg) => print("Error: " + msg);
```

### 12.2 Propagation

The `?` operator propagates errors upward (requires `Result` return type):

```sylva
fn compute(x: i64) -> Result<i64, str> {
    let a = safe_divide(x, 2)?;    // returns Err immediately if divide fails
    let b = safe_divide(a, 3)?;
    return Ok(b);
}
```

### 12.3 Runtime Errors

Unhandled type mismatches and bounds violations produce runtime errors that display with the full execution trace:

```
RuntimeError: type mismatch at line 14 col 8
  Expected: i64
  Got:      str ("hello")
  Trace:
    [12] let x = parse_input()      -> "hello"
    [13] let doubled = x * 2        -> ERROR
```

---

## 13. Telemetry and Observability

```sylva
// Emit a structured event
emit { event: "data_loaded", rows: len(df), source: "data.csv" }

// Observe live events (from REPL or CLI)
// $ build observe --module my_module --filter effect
```

All expressions in the REPL automatically emit telemetry to OmniCore, which is why `:trace` and `:replay` work — the trace is the telemetry log.

---

## 14. Notebook Environment

Sylva supports a notebook-style workflow. Create a `.sy` file with cell markers:

```sylva
// %% Cell 1: Load data
let df = DataFrame::from_csv("sales.csv");
print(df.describe());

// %% Cell 2: Filter
let high_value = df.filter(fn(row) { row.amount > 1000 });
print("High-value transactions: " + str(high_value.shape()[0]));

// %% Cell 3: Aggregate
let by_region = high_value.group_by("region").sum("amount");
print(by_region);
```

Run a notebook:

```bash
build run notebook.sy
build run notebook.sy --hot     # hot-reload on file change
```

---

## 15. Complete Example: Data Pipeline

```sylva
// pipeline.sy
// End-to-end data pipeline with filtering, aggregation, and cross-language compute

import titan::stats       // Titan module for heavy computation

fn load_and_validate(path: str) -> DataFrame {
    let df = DataFrame::from_csv(path);
    if df.shape()[0] == 0 {
        print("WARNING: empty dataset at " + path);
    }
    return df;
}

fn normalize_row(row: [f64]) -> [f64] {
    let total = reduce(fn(acc, x) { acc + x }, row, 0.0);
    if total == 0.0 { return row; }
    return map(fn(x) { x / total }, row);
}

fn run_pipeline(input_path: str, threshold: f64) {
    // [1] Load
    let raw = load_and_validate(input_path);
    emit { event: "loaded", rows: raw.shape()[0] };

    // [2] Filter outliers
    let clean = raw.filter(fn(row) { row.value < threshold });
    print("Kept " + str(clean.shape()[0]) + " of " + str(raw.shape()[0]) + " rows");

    // [3] Normalize using Titan for performance
    let values = clean.select(["value"]).to_list();
    let normalized = map(normalize_row, values);

    // [4] Aggregate
    let by_category = clean.group_by("category").mean("value");
    print(by_category);

    // [5] Compute statistics using Titan math
    let summary = stats::summarize(normalized);
    print("Mean: " + str(summary.mean));
    print("StdDev: " + str(summary.std));

    emit { event: "pipeline_complete", output_rows: clean.shape()[0] };
}

// Entry point
run_pipeline("data/sales.csv", threshold: 10000.0);
```

---

## 16. Quick Reference Card

```
KEYWORDS       let  let mut  fn  if  else  while  for  in  return  break  continue
               case  of  import  emit  true  false  ()
TYPES          i64  f64  bool  str  ()  [T]  (T1,T2)  DataFrame  Result<T,E>
OPERATORS      + - * /  %    & | ^ ~ << >>    == != < > <= >=    && || !
               = +=    . ::  []  +  (string concat)
BUILTINS       print  read_line  len  str  int  float  bool
               append  prepend  filter  map  reduce  sort  reverse
               first  last  slice  min  max  abs  sqrt  pow
               DataFrame::from_csv  .describe()  .filter()  .group_by()  .mean()
REPL COMMANDS  :trace  :rewind N  :step  :replay  :load file  :break  :continue  :watch
CROSS-LANG     import titan::module  spawn_actor("aether://Actor")  import hash as name
```
