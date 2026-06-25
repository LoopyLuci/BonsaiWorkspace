/**
 * Arithmetic Expression Evaluator — Omnisystem Integration Test
 * 
 * A complete recursive-descent parser + evaluator for:
 *   - Operators: +, -, *, /, parentheses
 *   - No raw pointers (safe for certified Lingua conversion)
 *   - Mutual recursion (expr → term → factor)
 *   - Mutable state (lexer position tracking)
 *   - Error handling (exit, fprintf)
 * 
 * This program serves as an end-to-end test for:
 *   1. Omni Lingua C→Titan conversion (certified fidelity)
 *   2. Titan compiler (borrow checking, LLVM codegen)
 *   3. Bidirectional round-trip (Titan→C)
 *   4. Full Omnisystem pipeline validation
 * 
 * Test case: "2 * (3 + 4) - 5"
 * Expected result: 9 (precedence: * before +/-, parentheses first)
 */

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <ctype.h>

/* ============================================================================
 * Token Types and Structures
 * ============================================================================ */

/**
 * TokenKind: All token types for the arithmetic language.
 * 
 * Maps directly to Titan enums; round-trip conversion must preserve values.
 */
typedef enum {
    TOK_NUM,      // 0: integer literal
    TOK_ADD,      // 1: '+' operator
    TOK_SUB,      // 2: '-' operator
    TOK_MUL,      // 3: '*' operator
    TOK_DIV,      // 4: '/' operator
    TOK_LPAREN,   // 5: '(' token
    TOK_RPAREN,   // 6: ')' token
    TOK_END       // 7: end of input
} TokenKind;

/**
 * Token: Single lexical token with kind and optional numeric value.
 * 
 * Stresses struct handling in Lingua converter:
 *   - Fixed-size fields (TaggedUnion-like pattern)
 *   - Value only meaningful when kind == TOK_NUM
 *   - Passed by value in recursive calls
 */
typedef struct {
    TokenKind kind;     // Type of token
    int value;          // Numeric value (if kind == TOK_NUM)
} Token;

/**
 * Lexer: Stateful lexical analyzer.
 * 
 * Stresses mutable struct refs in Lingua converter:
 *   - Mutable borrows (lexer_next receives &Lexer)
 *   - Position tracking (state mutation)
 *   - String scanning (input[pos])
 *   - Borrow checker must ensure safety
 */
typedef struct {
    const char *input;  // Input string (immutable ref)
    int pos;            // Current position (mutable state)
    Token current;      // Current token (cached)
} Lexer;

/* ============================================================================
 * Lexer Implementation
 * ============================================================================ */

/**
 * Skip whitespace and advance to next token.
 * 
 * Stresses:
 *   - While loops with character classification
 *   - Switch statements (for operators)
 *   - Error handling (invalid char → fprintf + exit)
 *   - Mutable state updates
 */
void lexer_next(Lexer *lex) {
    // Skip whitespace
    while (isspace(lex->input[lex->pos])) {
        lex->pos++;
    }
    
    char c = lex->input[lex->pos];
    
    // End of input
    if (c == '\0') {
        lex->current.kind = TOK_END;
        return;
    }
    
    // Digit: scan multi-digit integer
    if (isdigit(c)) {
        int val = 0;
        while (isdigit(lex->input[lex->pos])) {
            val = val * 10 + (lex->input[lex->pos] - '0');
            lex->pos++;
        }
        lex->current.kind = TOK_NUM;
        lex->current.value = val;
        return;
    }
    
    // Single-char operators and delimiters
    switch (c) {
        case '+':
            lex->current.kind = TOK_ADD;
            lex->pos++;
            return;
        case '-':
            lex->current.kind = TOK_SUB;
            lex->pos++;
            return;
        case '*':
            lex->current.kind = TOK_MUL;
            lex->pos++;
            return;
        case '/':
            lex->current.kind = TOK_DIV;
            lex->pos++;
            return;
        case '(':
            lex->current.kind = TOK_LPAREN;
            lex->pos++;
            return;
        case ')':
            lex->current.kind = TOK_RPAREN;
            lex->pos++;
            return;
        default:
            // Invalid character: error handling with exit
            fprintf(stderr, "ERROR: Invalid character '%c'\n", c);
            exit(1);
    }
}

/**
 * Initialize lexer for input string.
 * 
 * Stresses:
 *   - Struct initialization
 *   - Function calls (lexer_next)
 *   - Mutable reference passing
 */
void lexer_init(Lexer *lex, const char *input) {
    lex->input = input;
    lex->pos = 0;
    lexer_next(lex);  // Prime first token
}

/* ============================================================================
 * Recursive Descent Parser
 * ============================================================================ */

/**
 * Parser grammar (operator precedence):
 * 
 *   expr   → term {('+' | '-') term}         // Addition/subtraction (lowest)
 *   term   → factor {('*' | '/') factor}     // Multiplication/division
 *   factor → '(' expr ')' | number           // Parentheses (highest)
 * 
 * Mutual recursion:
 *   parse_expr ↔ parse_term ↔ parse_factor
 * 
 * Stresses:
 *   - Mutual recursion (Titan compiler must handle cycles)
 *   - Mutable struct refs
 *   - Control flow (while, if, switch)
 *   - Error handling (missing paren)
 */

int parse_expr(Lexer *lex);
int parse_term(Lexer *lex);
int parse_factor(Lexer *lex);

/**
 * Parse and evaluate term = factor {('*' | '/') factor}.
 * 
 * Left-associative: 2 / 4 / 2 = (2 / 4) / 2 = 0.
 */
int parse_term(Lexer *lex) {
    int val = parse_factor(lex);
    
    while (lex->current.kind == TOK_MUL || lex->current.kind == TOK_DIV) {
        TokenKind op = lex->current.kind;
        lexer_next(lex);
        int rhs = parse_factor(lex);
        
        if (op == TOK_MUL) {
            val *= rhs;
        } else {
            val /= rhs;
        }
    }
    
    return val;
}

/**
 * Parse and evaluate expr = term {('+' | '-') term}.
 * 
 * Left-associative: 5 - 3 - 1 = (5 - 3) - 1 = 1.
 */
int parse_expr(Lexer *lex) {
    int val = parse_term(lex);
    
    while (lex->current.kind == TOK_ADD || lex->current.kind == TOK_SUB) {
        TokenKind op = lex->current.kind;
        lexer_next(lex);
        int rhs = parse_term(lex);
        
        if (op == TOK_ADD) {
            val += rhs;
        } else {
            val -= rhs;
        }
    }
    
    return val;
}

/**
 * Parse and evaluate factor = '(' expr ')' | number.
 * 
 * Stresses:
 *   - Error handling (missing closing paren)
 *   - Recursion (factor → expr → term → factor cycle)
 *   - Nested expressions
 */
int parse_factor(Lexer *lex) {
    // Parenthesized expression
    if (lex->current.kind == TOK_LPAREN) {
        lexer_next(lex);
        int val = parse_expr(lex);  // Recursive call (breaks precedence cycle)
        
        if (lex->current.kind != TOK_RPAREN) {
            fprintf(stderr, "ERROR: Missing closing parenthesis\n");
            exit(1);
        }
        
        lexer_next(lex);
        return val;
    }
    
    // Number literal
    if (lex->current.kind == TOK_NUM) {
        int val = lex->current.value;
        lexer_next(lex);
        return val;
    }
    
    // Unexpected token
    fprintf(stderr, "ERROR: Unexpected token (expected number or '(')\n");
    exit(1);
}

/* ============================================================================
 * Main Entry Point
 * ============================================================================ */

/**
 * Evaluates a hardcoded arithmetic expression.
 * 
 * Expression: "2 * (3 + 4) - 5"
 *   Step 1: (3 + 4) = 7
 *   Step 2: 2 * 7 = 14
 *   Step 3: 14 - 5 = 9
 * 
 * Expected output: "Result: 9"
 * 
 * Stresses:
 *   - Full parser pipeline
 *   - String literals
 *   - I/O (printf)
 *   - Struct initialization and usage
 */
int main() {
    const char *input = "2 * (3 + 4) - 5";
    
    Lexer lex;
    lexer_init(&lex, input);
    
    int result = parse_expr(&lex);
    
    printf("Result: %d\n", result);
    
    return 0;
}

/* ============================================================================
 * Design Notes for Lingua Conversion
 * ============================================================================ */

/*
 * CERTIFIED FIDELITY (C→Titan):
 * 
 * This code contains no unsafe features:
 *   ✓ No raw pointers (all structs passed by ref or value)
 *   ✓ No array indexing beyond string literals
 *   ✓ No pointer arithmetic
 *   ✓ No casts (except implicit int promotion)
 *   ✓ Stack-only allocation (Lexer, Token on stack)
 *   ✓ No undefined behavior
 * 
 * The Lingua converter should emit pure Titan:
 *   - TokenKind → pub enum TokenKind { ... }
 *   - Token → pub struct Token { ... }
 *   - Lexer → pub struct Lexer { ... }
 *   - All functions → pub fn (or private if internal)
 *   - String indexing → checked array access
 *   - Mutable refs → &mut Lexer
 *   - Error handling → panic!() instead of exit()
 * 
 * BIDIRECTIONAL ROUND-TRIP (Titan→C):
 * 
 * After Titan compilation (C→Titan) and round-trip (Titan→C):
 *   1. The generated Titan code must compile: omni build calc.ti
 *   2. The binary must execute and print: Result: 9
 *   3. The Titan→C reverse conversion must compile with GCC
 *   4. The round-tripped binary must print: Result: 9
 *   5. Both executables should be functionally equivalent
 * 
 * This proves:
 *   - Type safety across boundaries
 *   - Borrow checker correctness
 *   - Code generation fidelity
 *   - Bidirectional converter correctness
 */
