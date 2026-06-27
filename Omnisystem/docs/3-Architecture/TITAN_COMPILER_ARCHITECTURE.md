# TITAN Compiler Architecture v1.0
## Complete Implementation Guide

---

## 1. COMPILER PIPELINE OVERVIEW

```
Source (.titan)
    ↓
[Lexer] → Tokens
    ↓
[Parser] → Abstract Syntax Tree (AST)
    ↓
[Type Checker] → Typed AST + Type Environment
    ↓
[Optimizer] → Optimized Typed AST
    ↓
[Code Generator] → LLVM IR
    ↓
[LLVM Backend] → Native Code (x86, ARM, etc.)
    ↓
[Linker] → Executable Binary
```

---

## 2. LEXER (Tokenization)

### 2.1 Token Types

```
KEYWORDS:
  fn, let, mut, const, if, else, while, for, loop, match,
  return, break, continue, struct, enum, trait, impl, use,
  mod, pub, private, as, async, await, spawn, channel,
  true, false, panic, assert, asm, inline, extern

OPERATORS:
  +, -, *, /, %, ^, &, |, !, ~
  ==, !=, <, >, <=, >=
  &&, ||
  =, +=, -=, *=, /=, %=, &=, |=, ^=
  ->, =>, ::, ..

DELIMITERS:
  (, ), [, ], {, }, <, >, ,, ;, :, ., @, #, $, \, ?

LITERALS:
  INTEGER: [0-9]+ (i8, i16, i32, i64, u8, u16, u32, u64, usize)
  FLOAT: [0-9]+\.[0-9]+ (f32, f64)
  STRING: "..." (UTF-8)
  IDENTIFIER: [a-zA-Z_][a-zA-Z0-9_]*

WHITESPACE & COMMENTS:
  Spaces, tabs, newlines (significant in some contexts)
  // Single-line comment
  /* Multi-line comment */
```

### 2.2 Lexer Implementation (Pseudocode)

```
class Lexer {
    source: string
    position: usize
    tokens: [Token]
    
    fn next_token() -> Token {
        skip_whitespace_and_comments()
        
        if at_end(source) {
            return Token::EOF
        }
        
        char = source[position]
        
        if is_digit(char) {
            return lex_number()
        }
        
        if is_alpha(char) or char == '_' {
            return lex_identifier()
        }
        
        if char == '"' {
            return lex_string()
        }
        
        return lex_operator_or_delimiter()
    }
    
    fn lex_number() -> Token {
        start = position
        
        while is_digit(current_char()) {
            advance()
        }
        
        if current_char() == '.' and is_digit(peek_next()) {
            advance()  // Skip '.'
            while is_digit(current_char()) {
                advance()
            }
            return Token::FLOAT(substring(start, position))
        }
        
        return Token::INTEGER(substring(start, position))
    }
    
    fn lex_identifier() -> Token {
        start = position
        
        while is_alnum(current_char()) or current_char() == '_' {
            advance()
        }
        
        text = substring(start, position)
        
        if is_keyword(text) {
            return Token::KEYWORD(text)
        }
        
        return Token::IDENTIFIER(text)
    }
    
    fn lex_string() -> Token {
        advance()  // Skip opening quote
        start = position
        
        while current_char() != '"' {
            if current_char() == '\\' {
                advance()  // Skip escape char
            }
            advance()
        }
        
        text = substring(start, position)
        advance()  // Skip closing quote
        
        return Token::STRING(unescape(text))
    }
}
```

---

## 3. PARSER (AST Construction)

### 3.1 AST Node Types

```
AST Node Hierarchy:

Program
  ├── Import[]
  └── Declaration[]

Declaration
  ├── FunctionDecl
  │   ├── name: string
  │   ├── type_params: TypeParam[]
  │   ├── params: Parameter[]
  │   ├── return_type: Type
  │   └── body: Block
  │
  ├── StructDecl
  │   ├── name: string
  │   ├── type_params: TypeParam[]
  │   └── fields: Field[]
  │
  ├── EnumDecl
  │   ├── name: string
  │   ├── type_params: TypeParam[]
  │   └── variants: Variant[]
  │
  ├── TraitDecl
  │   ├── name: string
  │   ├── type_params: TypeParam[]
  │   └── methods: MethodSignature[]
  │
  ├── ImplBlock
  │   ├── trait_name: string (optional)
  │   ├── type_name: string
  │   └── methods: MethodDecl[]
  │
  └── ConstDecl
      ├── name: string
      ├── type: Type
      └── value: Expression

Statement
  ├── LetStmt
  │   ├── name: string
  │   ├── is_mut: bool
  │   ├── type: Type (optional)
  │   └── init: Expression
  │
  ├── ExpressionStmt
  │   └── expression: Expression
  │
  └── ReturnStmt
      └── value: Expression (optional)

Expression
  ├── Literal
  │   ├── Integer(i64)
  │   ├── Float(f64)
  │   ├── String(string)
  │   ├── Bool(bool)
  │   └── Array(Expression[])
  │
  ├── Identifier(string)
  │
  ├── BinaryOp
  │   ├── operator: Operator
  │   ├── left: Expression
  │   └── right: Expression
  │
  ├── UnaryOp
  │   ├── operator: Operator
  │   └── operand: Expression
  │
  ├── FunctionCall
  │   ├── function: Expression
  │   ├── type_args: Type[]
  │   └── args: Expression[]
  │
  ├── IfExpression
  │   ├── condition: Expression
  │   ├── then_branch: Block
  │   ├── else_branch: Block (optional)
  │   └── branches: (condition, block)[]
  │
  ├── MatchExpression
  │   ├── scrutinee: Expression
  │   └── arms: (Pattern, Expression)[]
  │
  ├── LoopExpression
  │   ├── kind: "while" | "for" | "loop"
  │   ├── init: Expression (optional)
  │   ├── condition: Expression (optional)
  │   ├── body: Block
  │   └── increment: Expression (optional)
  │
  ├── BlockExpression
  │   └── statements: Statement[]
  │
  ├── StructLiteral
  │   ├── struct_name: string
  │   └── fields: (name, value)[]
  │
  ├── EnumVariant
  │   ├── enum_name: string
  │   ├── variant_name: string
  │   └── values: Expression[]
  │
  ├── Borrow
  │   ├── is_mut: bool
  │   └── expr: Expression
  │
  └── Dereference
      └── expr: Expression

Type
  ├── Primitive
  │   ├── bool, i32, i64, f32, f64, string, bytes
  │   └── usize, isize
  │
  ├── Named(string)
  │
  ├── Generic
  │   ├── base: Type
  │   └── type_args: Type[]
  │
  ├── Array
  │   ├── element_type: Type
  │   └── size: Expression (optional)
  │
  ├── Reference
  │   ├── is_mut: bool
  │   └── inner_type: Type
  │
  ├── Tuple
  │   └── element_types: Type[]
  │
  ├── Function
  │   ├── param_types: Type[]
  │   └── return_type: Type
  │
  └── Union
      └── variants: Type[]

Pattern
  ├── Literal(value)
  ├── Identifier(string)
  ├── Wildcard(_)
  ├── Tuple(Pattern[])
  ├── Struct(string, (name, pattern)[])
  ├── Enum(string, string, Pattern[])
  └── Or(Pattern, Pattern)
```

### 3.2 Parser Implementation (Pseudocode)

```
class Parser {
    tokens: [Token]
    position: usize
    current_token: Token
    
    fn parse() -> Program {
        imports = []
        declarations = []
        
        while not at_end() {
            if match("use") {
                imports.push(parse_import())
            } else {
                declarations.push(parse_declaration())
            }
        }
        
        return Program(imports, declarations)
    }
    
    fn parse_declaration() -> Declaration {
        if match("fn") {
            return parse_function()
        }
        
        if match("struct") {
            return parse_struct()
        }
        
        if match("enum") {
            return parse_enum()
        }
        
        if match("trait") {
            return parse_trait()
        }
        
        if match("impl") {
            return parse_impl()
        }
        
        if match("const") {
            return parse_const()
        }
        
        error("Expected declaration")
    }
    
    fn parse_function() -> FunctionDecl {
        name = expect("identifier").value
        
        type_params = []
        if match("<") {
            type_params = parse_type_params()
            expect(">")
        }
        
        expect("(")
        params = parse_parameters()
        expect(")")
        
        return_type = Type::Void
        if match("->") {
            return_type = parse_type()
        }
        
        body = parse_block()
        
        return FunctionDecl(name, type_params, params, return_type, body)
    }
    
    fn parse_expression() -> Expression {
        return parse_assignment()
    }
    
    fn parse_assignment() -> Expression {
        expr = parse_logical_or()
        
        if match("=") {
            value = parse_assignment()
            return Assignment(expr, value)
        }
        
        return expr
    }
    
    fn parse_logical_or() -> Expression {
        expr = parse_logical_and()
        
        while match("||") {
            op = previous()
            right = parse_logical_and()
            expr = BinaryOp(op, expr, right)
        }
        
        return expr
    }
    
    fn parse_logical_and() -> Expression {
        expr = parse_equality()
        
        while match("&&") {
            op = previous()
            right = parse_equality()
            expr = BinaryOp(op, expr, right)
        }
        
        return expr
    }
    
    fn parse_equality() -> Expression {
        expr = parse_comparison()
        
        while match("==", "!=") {
            op = previous()
            right = parse_comparison()
            expr = BinaryOp(op, expr, right)
        }
        
        return expr
    }
    
    fn parse_comparison() -> Expression {
        expr = parse_addition()
        
        while match("<", ">", "<=", ">=") {
            op = previous()
            right = parse_addition()
            expr = BinaryOp(op, expr, right)
        }
        
        return expr
    }
    
    fn parse_addition() -> Expression {
        expr = parse_multiplication()
        
        while match("+", "-") {
            op = previous()
            right = parse_multiplication()
            expr = BinaryOp(op, expr, right)
        }
        
        return expr
    }
    
    fn parse_multiplication() -> Expression {
        expr = parse_unary()
        
        while match("*", "/", "%") {
            op = previous()
            right = parse_unary()
            expr = BinaryOp(op, expr, right)
        }
        
        return expr
    }
    
    fn parse_unary() -> Expression {
        if match("!", "-", "&", "*") {
            op = previous()
            expr = parse_unary()
            return UnaryOp(op, expr)
        }
        
        return parse_postfix()
    }
    
    fn parse_postfix() -> Expression {
        expr = parse_primary()
        
        while true {
            if match("(") {
                // Function call
                args = parse_arguments()
                expect(")")
                expr = FunctionCall(expr, args)
            } else if match("[") {
                // Array/indexing
                index = parse_expression()
                expect("]")
                expr = ArrayAccess(expr, index)
            } else if match(".") {
                // Field access or method call
                field = expect("identifier").value
                if match("(") {
                    args = parse_arguments()
                    expect(")")
                    expr = MethodCall(expr, field, args)
                } else {
                    expr = FieldAccess(expr, field)
                }
            } else {
                break
            }
        }
        
        return expr
    }
    
    fn parse_primary() -> Expression {
        if match("true") {
            return Literal::Bool(true)
        }
        
        if match("false") {
            return Literal::Bool(false)
        }
        
        if check("integer") {
            return Literal::Integer(advance().value)
        }
        
        if check("float") {
            return Literal::Float(advance().value)
        }
        
        if check("string") {
            return Literal::String(advance().value)
        }
        
        if match("[") {
            elements = []
            while not check("]") {
                elements.push(parse_expression())
                if not match(",") { break }
            }
            expect("]")
            return Literal::Array(elements)
        }
        
        if match("(") {
            expr = parse_expression()
            expect(")")
            return expr
        }
        
        if match("if") {
            return parse_if()
        }
        
        if match("match") {
            return parse_match()
        }
        
        if match("while") {
            return parse_while()
        }
        
        if match("for") {
            return parse_for()
        }
        
        if match("loop") {
            body = parse_block()
            return Loop(body)
        }
        
        if match("{") {
            statements = []
            while not check("}") {
                statements.push(parse_statement())
            }
            expect("}")
            return BlockExpression(statements)
        }
        
        name = expect("identifier").value
        return Identifier(name)
    }
}
```

---

## 4. TYPE CHECKER

### 4.1 Type Checking Algorithm

```
fn infer_and_check_types(ast: AST, env: Environment) -> (TypedAST, Errors) {
    errors = []
    typed_ast = new TypedAST()
    
    for decl in ast.declarations {
        typed_decl = type_check_declaration(decl, env)
        if typed_decl is Error {
            errors.push(typed_decl.error)
        } else {
            typed_ast.add(typed_decl)
            env.add(typed_decl)
        }
    }
    
    return (typed_ast, errors)
}

fn type_check_declaration(decl: Declaration, env: Environment) -> Result<TypedDecl> {
    if decl is FunctionDecl {
        return type_check_function(decl, env)
    }
    
    if decl is StructDecl {
        return type_check_struct(decl, env)
    }
    
    if decl is TraitDecl {
        return type_check_trait(decl, env)
    }
    
    // ... other declaration types
}

fn type_check_expression(expr: Expression, env: Environment) -> (Type, TypedExpr) {
    if expr is Literal {
        if expr is Integer { return (i64, TypedExpr(expr)) }
        if expr is Float { return (f64, TypedExpr(expr)) }
        if expr is String { return (string, TypedExpr(expr)) }
        if expr is Bool { return (bool, TypedExpr(expr)) }
    }
    
    if expr is Identifier {
        type = env.lookup(expr.name)
        if type is null {
            error("Undefined variable: {}", expr.name)
        }
        return (type, TypedExpr(expr))
    }
    
    if expr is BinaryOp {
        (left_type, left_expr) = type_check_expression(expr.left, env)
        (right_type, right_expr) = type_check_expression(expr.right, env)
        
        result_type = infer_binary_op_type(expr.op, left_type, right_type)
        if result_type is null {
            error("Type mismatch for operator {}: {} and {}", 
                  expr.op, left_type, right_type)
        }
        
        return (result_type, TypedExpr(expr.op, left_expr, right_expr))
    }
    
    if expr is FunctionCall {
        (fn_type, fn_expr) = type_check_expression(expr.function, env)
        
        if fn_type is not FunctionType {
            error("Cannot call non-function")
        }
        
        // Check arguments match parameter types
        for (i, arg) in expr.args.enumerate() {
            (arg_type, typed_arg) = type_check_expression(arg, env)
            param_type = fn_type.params[i]
            
            if not unify(arg_type, param_type) {
                error("Argument type mismatch at position {}", i)
            }
        }
        
        return (fn_type.return_type, TypedExpr(expr))
    }
    
    // ... other expression types
}

fn unify(type1: Type, type2: Type) -> bool {
    // Check if two types are compatible
    
    if type1 == type2 { return true }
    
    if type1 is TypeVariable {
        // Occurs check
        if occurs(type1, type2) { return false }
        type1.bind(type2)
        return true
    }
    
    if type2 is TypeVariable {
        if occurs(type2, type1) { return false }
        type2.bind(type1)
        return true
    }
    
    if type1 is Generic and type2 is Generic {
        if type1.base != type2.base { return false }
        if type1.type_args.len() != type2.type_args.len() { return false }
        
        for (arg1, arg2) in zip(type1.type_args, type2.type_args) {
            if not unify(arg1, arg2) { return false }
        }
        return true
    }
    
    return false
}
```

### 4.2 Trait Resolution

```
fn resolve_trait_impl(expr: Expression, trait_name: string, env: Environment) -> TraitImpl {
    // Find a trait implementation that matches
    
    expr_type = infer_type(expr, env)
    
    for impl in env.trait_implementations {
        if impl.trait_name == trait_name and impl.type == expr_type {
            return impl
        }
    }
    
    error("No implementation of {} for type {}", trait_name, expr_type)
}

fn resolve_method_call(receiver: Expression, method_name: string, env: Environment) -> Method {
    receiver_type = infer_type(receiver, env)
    
    // Look for method in type's impl blocks
    for impl_block in env.impl_blocks {
        if impl_block.type == receiver_type {
            for method in impl_block.methods {
                if method.name == method_name {
                    return method
                }
            }
        }
    }
    
    // Look for method in traits
    for trait_impl in env.trait_implementations {
        if trait_impl.type == receiver_type {
            for method in trait_impl.methods {
                if method.name == method_name {
                    return method
                }
            }
        }
    }
    
    error("Method {} not found for type {}", method_name, receiver_type)
}
```

### 4.3 Lifetime Analysis

```
fn analyze_lifetimes(ast: TypedAST, env: Environment) -> LifetimeInfo {
    // Determine lifetimes of all borrowed references
    
    lifetime_info = new LifetimeInfo()
    
    for expr in ast.all_expressions() {
        if expr is Borrow {
            // Determine the lifetime of this reference
            lifetime = infer_lifetime(expr.expr, env)
            lifetime_info.add(expr, lifetime)
        }
        
        if expr is FunctionCall {
            // Check that lifetimes in arguments are valid for function signature
            for (arg, param_lifetime) in zip(expr.args, expr.function.param_lifetimes) {
                arg_lifetime = lifetime_info.get(arg)
                if not is_valid_for(arg_lifetime, param_lifetime) {
                    error("Lifetime mismatch in function call")
                }
            }
        }
    }
    
    return lifetime_info
}

fn infer_lifetime(expr: Expression, env: Environment) -> Lifetime {
    // Lifetime is the scope where the borrowed value is valid
    
    if expr is Variable {
        // Variable lifetime is its scope
        scope = find_containing_scope(expr)
        return scope
    }
    
    if expr is FieldAccess {
        // Lifetime is the lifetime of the struct
        parent_lifetime = infer_lifetime(expr.object, env)
        return parent_lifetime
    }
    
    // ... other cases
}
```

---

## 5. OPTIMIZER

### 5.1 Optimization Passes

```
CONSTANT FOLDING:
    5 + 3 → 8
    true && false → false
    "hello" + " " + "world" → "hello world"

DEAD CODE ELIMINATION:
    let x = 5
    let y = 10  // y is never used, eliminate
    return x

INLINING:
    fn add(a, b) { return a + b }
    x = add(2, 3) → x = 5

LOOP UNROLLING:
    for i in 0..4 { process(i) }
    →
    process(0)
    process(1)
    process(2)
    process(3)

COMMON SUBEXPRESSION ELIMINATION:
    x = a + b
    y = a + b → y = x

VECTORIZATION:
    for i in 0..4 { arr[i] = arr[i] * 2 }
    →
    vec_multiply(arr, 2)  // SIMD instruction
```

### 5.2 Optimizer Implementation (Pseudocode)

```
class Optimizer {
    fn optimize(ast: TypedAST) -> OptimizedAST {
        ast = fold_constants(ast)
        ast = eliminate_dead_code(ast)
        ast = inline_functions(ast)
        ast = unroll_loops(ast)
        ast = eliminate_common_subexpressions(ast)
        ast = vectorize(ast)
        return ast
    }
    
    fn fold_constants(ast: TypedAST) -> TypedAST {
        visitor = ConstantFolder()
        return visitor.visit(ast)
    }
    
    fn inline_functions(ast: TypedAST) -> TypedAST {
        for function in ast.functions {
            if should_inline(function) {
                ast.replace_calls(function, inline_body(function))
            }
        }
        return ast
    }
    
    fn should_inline(function: FunctionDecl) -> bool {
        // Inline small functions, functions called once, etc.
        return function.body.size() < 10 or function.call_count == 1
    }
}
```

---

## 6. CODE GENERATOR (LLVM IR)

### 6.1 LLVM IR Generation

```
TITAN Code:
    fn add(x: i64, y: i64) -> i64 {
        return x + y
    }

LLVM IR:
    define i64 @add(i64 %x, i64 %y) {
        %1 = add i64 %x, %y
        ret i64 %1
    }

TITAN Code:
    fn fibonacci(n: u32) -> u64 {
        if n <= 1 { return n }
        return fibonacci(n - 1) + fibonacci(n - 2)
    }

LLVM IR:
    define i64 @fibonacci(i32 %n) {
        %cond = icmp ule i32 %n, 1
        br i1 %cond, label %then, label %else
    then:
        ret i64 %n
    else:
        %n_minus_1 = sub i32 %n, 1
        %fib1 = call i64 @fibonacci(i32 %n_minus_1)
        %n_minus_2 = sub i32 %n, 2
        %fib2 = call i64 @fibonacci(i32 %n_minus_2)
        %result = add i64 %fib1, %fib2
        ret i64 %result
    }
```

### 6.2 Code Generator Implementation (Pseudocode)

```
class CodeGenerator {
    fn generate(ast: OptimizedAST) -> LLVMModule {
        module = new LLVMModule()
        
        // Generate declarations
        for decl in ast.declarations {
            if decl is FunctionDecl {
                generate_function(decl, module)
            } else if decl is StructDecl {
                generate_struct(decl, module)
            } else if decl is ConstDecl {
                generate_const(decl, module)
            }
        }
        
        return module
    }
    
    fn generate_function(fn_decl: FunctionDecl, module: LLVMModule) {
        fn_type = function_type(fn_decl)
        fn = module.add_function(fn_decl.name, fn_type)
        
        entry_block = fn.append_block("entry")
        builder = new IRBuilder(entry_block)
        
        // Generate function body
        environment = new Environment()
        for (param, arg) in zip(fn_decl.params, fn.args()) {
            environment.add(param.name, arg)
        }
        
        generate_block(fn_decl.body, builder, environment, module)
    }
    
    fn generate_block(block: Block, builder: IRBuilder, env: Environment, module: LLVMModule) {
        for stmt in block.statements {
            generate_statement(stmt, builder, env, module)
        }
    }
    
    fn generate_statement(stmt: Statement, builder: IRBuilder, env: Environment, module: LLVMModule) {
        if stmt is LetStmt {
            value = generate_expression(stmt.init, builder, env, module)
            env.add(stmt.name, value)
        }
        
        if stmt is ReturnStmt {
            if stmt.value {
                value = generate_expression(stmt.value, builder, env, module)
                builder.create_ret(value)
            } else {
                builder.create_ret_void()
            }
        }
        
        if stmt is ExpressionStmt {
            generate_expression(stmt.expression, builder, env, module)
        }
    }
    
    fn generate_expression(expr: Expression, builder: IRBuilder, env: Environment, module: LLVMModule) -> Value {
        if expr is Literal {
            if expr is Integer {
                return builder.create_const_int(expr.value)
            }
            if expr is Float {
                return builder.create_const_float(expr.value)
            }
            if expr is String {
                return builder.create_global_string(expr.value)
            }
        }
        
        if expr is Identifier {
            return env.lookup(expr.name)
        }
        
        if expr is BinaryOp {
            left = generate_expression(expr.left, builder, env, module)
            right = generate_expression(expr.right, builder, env, module)
            
            if expr.op == "+" {
                return builder.create_add(left, right)
            }
            if expr.op == "-" {
                return builder.create_sub(left, right)
            }
            if expr.op == "*" {
                return builder.create_mul(left, right)
            }
            if expr.op == "/" {
                return builder.create_div(left, right)
            }
            if expr.op == "==" {
                return builder.create_icmp_eq(left, right)
            }
            // ... other operators
        }
        
        if expr is FunctionCall {
            fn = module.get_function(expr.function.name)
            args = []
            for arg in expr.args {
                args.push(generate_expression(arg, builder, env, module))
            }
            return builder.create_call(fn, args)
        }
        
        if expr is IfExpression {
            then_block = builder.current_fn().append_block("then")
            else_block = builder.current_fn().append_block("else")
            merge_block = builder.current_fn().append_block("merge")
            
            cond = generate_expression(expr.condition, builder, env, module)
            builder.create_cond_br(cond, then_block, else_block)
            
            builder.set_insertion_point(then_block)
            then_value = generate_block(expr.then_branch, builder, env, module)
            builder.create_br(merge_block)
            
            builder.set_insertion_point(else_block)
            else_value = generate_block(expr.else_branch, builder, env, module)
            builder.create_br(merge_block)
            
            builder.set_insertion_point(merge_block)
            phi = builder.create_phi()
            phi.add_incoming(then_value, then_block)
            phi.add_incoming(else_value, else_block)
            return phi
        }
        
        // ... other expression types
    }
}
```

---

## 7. LINKING & CODE GENERATION

### 7.1 Native Code Generation

```
LLVM IR → Machine Code:

LLVM IR Module
    ↓
[LLVM Optimizer]  (Additional optimization passes)
    ↓
[Target Machine] (x86-64, ARM, RISC-V, etc.)
    ↓
Object Files (.o, .obj)
    ↓
[System Linker]  (ld, link.exe, etc.)
    ↓
Executable Binary
```

### 7.2 Linker

```
class Linker {
    fn link(object_files: [string], output: string) {
        // Link TITAN object files with C standard library
        
        command = build_linker_command(object_files, output)
        
        // Add standard library
        command.add_library("titan_std")
        
        // Add system libraries
        command.add_library("c")
        command.add_library("m")  // math
        
        // Link
        execute(command)
    }
    
    fn build_linker_command(object_files: [string], output: string) -> string {
        cmd = "ld -o " + output
        for obj_file in object_files {
            cmd += " " + obj_file
        }
        return cmd
    }
}
```

---

## 8. COMPLETE COMPILATION EXAMPLE

```
Source Code (hello.titan):
────────────────────────────
fn main() -> void {
    println("Hello, World!")
}

Step 1: LEXER
────────────
Tokens: [FN, IDENTIFIER("main"), LPAREN, RPAREN, ARROW, VOID,
         LBRACE, IDENTIFIER("println"), LPAREN, STRING("Hello, World!"),
         RPAREN, RBRACE, EOF]

Step 2: PARSER
──────────────
AST:
Program {
    declarations: [
        FunctionDecl {
            name: "main",
            params: [],
            return_type: void,
            body: Block {
                statements: [
                    ExpressionStmt {
                        expression: FunctionCall {
                            function: Identifier("println"),
                            args: [Literal::String("Hello, World!")]
                        }
                    }
                ]
            }
        }
    ]
}

Step 3: TYPE CHECKER
────────────────────
TypedAST:
Program {
    declarations: [
        FunctionDecl {
            name: "main",
            params: [],
            return_type: void,
            type: ([] -> void),
            body: Block {
                statements: [
                    ExpressionStmt {
                        expression: FunctionCall {
                            function: Identifier("println"),
                            type: ([string] -> void),
                            args: [Literal::String("Hello, World!", type: string)]
                        }
                    }
                ]
            }
        }
    ]
}

Step 4: OPTIMIZER
──────────────────
(No optimizations needed for this simple program)

Step 5: CODE GENERATOR (LLVM IR)
─────────────────────────────────
%string.const = private constant [13 x i8] c"Hello, World!"
declare void @println(i8* %str)

define void @main() {
    entry:
        %str = getelementptr [13 x i8]* %string.const, i32 0, i32 0
        call void @println(i8* %str)
        ret void
}

Step 6: LLVM BACKEND
──────────────────────
x86-64 Assembly:
.section __TEXT,__text
.globl _main
_main:
    push rbp
    mov rbp, rsp
    lea rax, [rel .L.str]
    mov rdi, rax
    call _println
    xor eax, eax
    pop rbp
    ret

.section __DATA,__data
.L.str:
    .asciiz "Hello, World!"

Step 7: LINK
─────────────
Final Executable: hello

(Ready to execute)
```

---

## 9. ERROR REPORTING

### 9.1 Error Types

```
SYNTAX ERRORS:
    hello.titan:5:10: error: unexpected token ';' in function declaration
        fn foo( ; ) { }
                 ^

TYPE ERRORS:
    hello.titan:12:5: error: type mismatch
        expected i64, found string
        let x: i64 = "hello"
               ^^^   ^^^^^^^

BORROW CHECKER ERRORS:
    hello.titan:15:10: error: value used after move
        let x = String::new()
                -----------
        let y = x
                - move here
        println(x)
                ^ use here (x was moved)

LIFETIME ERRORS:
    hello.titan:20:5: error: dangling reference
        &temporary_value
        ^^^^^^^^^^^^^^^^ reference does not live long enough

TRAIT RESOLUTION ERRORS:
    hello.titan:25:5: error: no implementation of Display for type CustomType
        println(obj)
        ^^^^^
```

### 9.2 Error Recovery

```
fn parse_with_recovery(tokens: [Token]) -> (AST, [Error]) {
    errors = []
    declarations = []
    
    while not at_end(tokens) {
        try {
            decl = parse_declaration()
            declarations.push(decl)
        } catch error {
            errors.push(error)
            // Skip to next declaration
            skip_until_next_declaration()
        }
    }
    
    return (AST(declarations), errors)
}
```

---

## 10. COMPILER MODES

### 10.1 Debug Mode

```
Compilation Flags:
    --debug         # Include debug symbols
    --assertions    # Keep assertion checks
    --checks        # Full runtime bounds checking
    --no-optimize   # Skip optimizations
    
Output:
    Large executable with DWARF debug info
    Slower execution
    Better error messages
```

### 10.2 Release Mode

```
Compilation Flags:
    --release       # Maximum optimization
    --strip         # Remove debug symbols
    --lto           # Link-time optimization
    --O3            # Aggressive optimization
    
Output:
    Small, fast executable
    No debug info
    All code paths verified at compile time
```

---

This comprehensive compiler architecture enables TITAN to achieve its design goals: zero-cost abstractions, memory safety, and incredible performance.

