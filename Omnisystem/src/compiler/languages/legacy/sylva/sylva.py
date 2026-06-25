#!/usr/bin/env python3
"""Sylva – Pure functional scripting language interpreter."""
import sys, os, readline, time, threading, collections

# ======================================================================
#  Lexer
# ======================================================================
class Token:
    def __init__(self, type, value, line, col):
        self.type = type; self.value = value; self.line = line; self.col = col

def lex(source):
    tokens = []
    i = 0
    line = 1
    col = 1
    while i < len(source):
        ch = source[i]
        if ch in ' \t\r':
            col += 1
            i += 1
            continue
        if ch == '\n':
            tokens.append(Token('NEWLINE', '\n', line, col))
            line += 1; col = 1; i += 1
            continue
        if ch == '#':
            while i < len(source) and source[i] != '\n':
                i += 1
            continue
        if ch.isalpha() or ch == '_':
            start = i
            while i < len(source) and (source[i].isalnum() or source[i] == '_'):
                i += 1
            word = source[start:i]
            typ = 'KEYWORD' if word in ('def', 'if', 'else', 'while', 'return', 'true', 'false', 'nil', 'and', 'or', 'not', 'let', 'in') else 'IDENT'
            tokens.append(Token(typ, word, line, col))
            col += (i - start)
            continue
        if ch.isdigit():
            start = i
            while i < len(source) and source[i].isdigit():
                i += 1
            num = int(source[start:i])
            tokens.append(Token('NUMBER', num, line, col))
            col += (i - start)
            continue
        if ch == '"':
            i += 1; start = i
            while i < len(source) and source[i] != '"':
                i += 1
            string = source[start:i]; i += 1
            tokens.append(Token('STRING', string, line, col))
            col += (len(string) + 2)
            continue
        # operators and punctuation
        op = ch
        if ch in '+-*/%=<>!&|':
            if i+1 < len(source) and source[i+1] in '=&|':
                op += source[i+1]; i += 1
        tokens.append(Token('OP', op, line, col))
        col += len(op); i += 1
    tokens.append(Token('EOF', None, line, col))
    return tokens

# ======================================================================
#  Parser
# ======================================================================
class ASTNode: pass
class Program(ASTNode):
    def __init__(self, statements): self.statements = statements
class DefStmt(ASTNode):
    def __init__(self, name, params, body): self.name = name; self.params = params; self.body = body
class ExprStmt(ASTNode):
    def __init__(self, expr): self.expr = expr
class IfStmt(ASTNode):
    def __init__(self, cond, then_stmt, else_stmt=None): self.cond = cond; self.then_stmt = then_stmt; self.else_stmt = else_stmt
class WhileStmt(ASTNode):
    def __init__(self, cond, body): self.cond = cond; self.body = body
class ReturnStmt(ASTNode):
    def __init__(self, expr): self.expr = expr
class Var(ASTNode):
    def __init__(self, name): self.name = name
class Number(ASTNode):
    def __init__(self, value): self.value = value
class String(ASTNode):
    def __init__(self, value): self.value = value
class Bool(ASTNode):
    def __init__(self, value): self.value = value
class Nil(ASTNode): pass
class BinOp(ASTNode):
    def __init__(self, left, op, right): self.left = left; self.op = op; self.right = right
class UnaryOp(ASTNode):
    def __init__(self, op, expr): self.op = op; self.expr = expr
class Call(ASTNode):
    def __init__(self, func, args): self.func = func; self.args = args
class Assign(ASTNode):
    def __init__(self, name, expr): self.name = name; self.expr = expr
class Dict(ASTNode):
    def __init__(self, items): self.items = items  # list of (key, value)
class Index(ASTNode):
    def __init__(self, obj, idx): self.obj = obj; self.idx = idx

def parse(tokens):
    pos = 0
    def peek(tok_type=None):
        nonlocal pos
        if pos >= len(tokens): return Token('EOF', None, 0, 0)
        t = tokens[pos]
        if tok_type and t.type != tok_type: return None
        return t
    def consume(tok_type=None):
        nonlocal pos
        if pos >= len(tokens): raise SyntaxError("Unexpected EOF")
        t = tokens[pos]; pos += 1
        if tok_type and t.type != tok_type:
            raise SyntaxError(f"Expected {tok_type}, got {t.type} at {t.line}:{t.col}")
        return t
    def parse_program():
        stmts = []
        skip_newlines()
        while peek().type != 'EOF':
            stmts.append(parse_stmt())
            skip_newlines()
        return Program(stmts)
    def skip_newlines():
        while peek().type == 'NEWLINE':
            consume('NEWLINE')
    def parse_stmt():
        if peek().type == 'KEYWORD' and peek().value == 'def':
            return parse_def()
        elif peek().type == 'KEYWORD' and peek().value == 'if':
            return parse_if()
        elif peek().type == 'KEYWORD' and peek().value == 'while':
            return parse_while()
        elif peek().type == 'KEYWORD' and peek().value == 'return':
            return parse_return()
        else:
            expr = parse_expr()
            if peek().type in ('NEWLINE', 'EOF'):
                consume('NEWLINE') if peek().type == 'NEWLINE' else None
            return ExprStmt(expr)
    def parse_def():
        consume('KEYWORD'); name = consume('IDENT').value
        consume('OP'); params = []
        while peek().type != 'OP' or peek().value != ')':
            if peek().type == 'IDENT':
                params.append(consume('IDENT').value)
                if peek().type == 'OP' and peek().value == ',':
                    consume('OP')
            else:
                break
        consume('OP')
        skip_newlines()
        body = []
        while peek().type != 'EOF' and not (peek().type == 'KEYWORD' and peek().value == 'def'):
            body.append(parse_stmt())
        return DefStmt(name, params, body)
    def parse_if():
        consume('KEYWORD')
        cond = parse_expr()
        skip_newlines()
        then_stmt = parse_stmt()
        else_stmt = None
        skip_newlines()
        if peek().type == 'KEYWORD' and peek().value == 'else':
            consume('KEYWORD'); skip_newlines()
            else_stmt = parse_stmt()
        return IfStmt(cond, then_stmt, else_stmt)
    def parse_while():
        consume('KEYWORD')
        cond = parse_expr()
        skip_newlines()
        body = parse_stmt()
        return WhileStmt(cond, body)
    def parse_return():
        consume('KEYWORD')
        expr = parse_expr() if peek().type not in ('NEWLINE', 'EOF') else Nil()
        return ReturnStmt(expr)
    def parse_expr(prec=0):
        return parse_binop(parse_atom(), prec)
    def parse_atom():
        t = peek()
        if t.type == 'NUMBER':
            consume(); return Number(t.value)
        if t.type == 'STRING':
            consume(); return String(t.value)
        if t.type == 'KEYWORD' and t.value in ('true','false'):
            consume(); return Bool(t.value == 'true')
        if t.type == 'KEYWORD' and t.value == 'nil':
            consume(); return Nil()
        if t.type == 'OP' and t.value == '{':
            return parse_dict()
        if t.type == 'IDENT':
            name = consume('IDENT').value
            if peek().type == 'OP' and peek().value == '(':
                consume('OP')
                args = []
                while peek().type != 'OP' or peek().value != ')':
                    args.append(parse_expr())
                    if peek().type == 'OP' and peek().value == ',':
                        consume('OP')
                consume('OP')
                return Call(Var(name), args)
            elif peek().type == 'OP' and peek().value == '[':
                consume('OP')
                idx = parse_expr()
                consume('OP')
                return Index(Var(name), idx)
            return Var(name)
        if t.type == 'OP' and t.value == '(':
            consume('OP')
            expr = parse_expr()
            consume('OP')
            return expr
        raise SyntaxError(f"Unexpected token {t.type}: {t.value}")
    def parse_dict():
        consume('OP')  # {
        items = []
        while peek().type != 'OP' or peek().value != '}':
            key = consume('IDENT').value
            consume('OP')  # :
            val = parse_expr()
            items.append((key, val))
            if peek().type == 'OP' and peek().value == ',':
                consume('OP')
        consume('OP')  # }
        return Dict(items)
    def parse_binop(left, min_prec):
        ops = {'+':10, '-':10, '*':20, '/':20, '%':20, '==':5, '!=':5, '<':5, '<=':5, '>':5, '>=':5, 'and':4, 'or':3}
        while True:
            t = peek()
            if t.type != 'OP' or t.value not in ops:
                break
            op = t.value
            prec = ops[op]
            if prec < min_prec:
                break
            consume('OP')
            right = parse_atom()
            right = parse_binop(right, prec+1)
            left = BinOp(left, op, right)
        return left
    return parse_program()

# ======================================================================
#  Interpreter
# ======================================================================
class Environment:
    def __init__(self, outer=None):
        self.store = {}
        self.outer = outer
    def get(self, name):
        if name in self.store:
            return self.store[name]
        if self.outer:
            return self.outer.get(name)
        raise NameError(f"Undefined variable: {name}")
    def set(self, name, value):
        self.store[name] = value
    def define(self, name, value):
        self.store[name] = value

class ReturnException(Exception):
    def __init__(self, value):
        self.value = value

def evaluate(node, env):
    if isinstance(node, Program):
        last = None
        for stmt in node.statements:
            last = evaluate(stmt, env)
        return last
    if isinstance(node, ExprStmt):
        return evaluate(node.expr, env)
    if isinstance(node, DefStmt):
        env.define(node.name, node)
        return None
    if isinstance(node, Call):
        func = evaluate(node.func, env)
        if isinstance(func, DefStmt):
            new_env = Environment(env)
            for param, arg in zip(func.params, node.args):
                new_env.define(param, evaluate(arg, env))
            for stmt in func.body[:-1]:
                evaluate(stmt, new_env)
            return evaluate(func.body[-1], new_env) if func.body else None
        elif callable(func):
            args = [evaluate(arg, env) for arg in node.args]
            return func(*args)
        else:
            raise TypeError(f"Not callable: {func}")
    if isinstance(node, Var):
        return env.get(node.name)
    if isinstance(node, Number):
        return node.value
    if isinstance(node, String):
        return node.value
    if isinstance(node, Bool):
        return node.value
    if isinstance(node, Nil):
        return None
    if isinstance(node, BinOp):
        left = evaluate(node.left, env)
        right = evaluate(node.right, env)
        if node.op == '+': return left + right
        if node.op == '-': return left - right
        if node.op == '*': return left * right
        if node.op == '/': return left // right if isinstance(left, int) else left / right
        if node.op == '%': return left % right
        if node.op == '==': return left == right
        if node.op == '!=': return left != right
        if node.op == '<': return left < right
        if node.op == '<=': return left <= right
        if node.op == '>': return left > right
        if node.op == '>=': return left >= right
        if node.op == 'and': return left and right
        if node.op == 'or': return left or right
    if isinstance(node, UnaryOp):
        val = evaluate(node.expr, env)
        if node.op == 'not': return not val
        if node.op == '-': return -val
    if isinstance(node, IfStmt):
        cond = evaluate(node.cond, env)
        if cond:
            return evaluate(node.then_stmt, env)
        elif node.else_stmt:
            return evaluate(node.else_stmt, env)
        return None
    if isinstance(node, WhileStmt):
        while evaluate(node.cond, env):
            evaluate(node.body, env)
        return None
    if isinstance(node, ReturnStmt):
        raise ReturnException(evaluate(node.expr, env))
    if isinstance(node, Assign):
        val = evaluate(node.expr, env)
        env.set(node.name, val)
        return val
    if isinstance(node, Dict):
        result = {}
        for key, val in node.items:
            result[key] = evaluate(val, env)
        return result
    if isinstance(node, Index):
        obj = evaluate(node.obj, env)
        idx = evaluate(node.idx, env)
        return obj[idx]
    raise RuntimeError(f"Unknown node: {type(node)}")

def run(source, env=None):
    if env is None:
        env = Environment()
        env.define('print', print)
        env.define('input', input)
        env.define('int', int)
        env.define('float', float)
        env.define('str', str)
        env.define('len', len)
        env.define('range', lambda *a: list(range(*a)))
        env.define('sleep', time.sleep)
        env.define('time', time.time)
        import random
        env.define('random', random.random)
        env.define('abs', abs)
        env.define('max', max)
        env.define('min', min)
    tokens = lex(source)
    ast = parse(tokens)
    try:
        evaluate(ast, env)
    except ReturnException as ret:
        return ret.value
    return None

def repl():
    env = Environment()
    while True:
        try:
            line = input('>>> ')
            if line.strip() == 'exit': break
            run(line, env)
        except Exception as e:
            print(f"Error: {e}")

if __name__ == '__main__':
    if len(sys.argv) > 1:
        with open(sys.argv[1]) as f:
            run(f.read())
    else:
        repl()
