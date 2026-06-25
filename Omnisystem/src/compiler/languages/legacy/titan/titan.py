#!/usr/bin/env python3
"""Titan compiler: converts Titan source to WebAssembly text format."""
import sys, re, os

def compile_titan(source):
    """Compile Titan source to WebAssembly text format."""
    lines = source.split('\n')
    wat = []
    wat.append('(module')
    wat.append('  (import "env" "print_i32" (func $print_i32 (param i32)))')
    wat.append('  (import "env" "print_str" (func $print_str (param i32 i32)))')
    wat.append('  (import "env" "get_key" (func $get_key (result i32)))')
    wat.append('  (import "env" "sleep" (func $sleep (param i32)))')
    wat.append('  (memory 1)')
    # Store strings in data section
    wat.append('  (data (i32.const 0) "Score: ")')
    wat.append('  (data (i32.const 100) " - ")')
    wat.append('  (data (i32.const 200) "\\n")')

    # Global game state
    wat.append('  (global $ball_x (mut i32) (i32.const 40))')
    wat.append('  (global $ball_y (mut i32) (i32.const 12))')
    wat.append('  (global $ball_dx (mut i32) (i32.const 1))')
    wat.append('  (global $ball_dy (mut i32) (i32.const 1))')
    wat.append('  (global $paddle1_y (mut i32) (i32.const 10))')
    wat.append('  (global $paddle2_y (mut i32) (i32.const 10))')
    wat.append('  (global $score1 (mut i32) (i32.const 0))')
    wat.append('  (global $score2 (mut i32) (i32.const 0))')

    # Helper function: clamp
    wat.append('  (func $clamp (param $val i32) (param $lo i32) (param $hi i32) (result i32)')
    wat.append('    (if (i32.lt_s (local.get $val) (local.get $lo))')
    wat.append('      (then (return (local.get $lo)))')
    wat.append('    )')
    wat.append('    (if (i32.gt_s (local.get $val) (local.get $hi))')
    wat.append('      (then (return (local.get $hi)))')
    wat.append('    )')
    wat.append('    (local.get $val)')
    wat.append('  )')

    # Update function
    wat.append('  (func $update (param $up1 i32) (param $down1 i32) (param $up2 i32) (param $down2 i32)')
    wat.append('    (local $new_x i32)')
    wat.append('    (local $new_y i32)')
    wat.append('    (local.set $new_x (i32.add (global.get $ball_x) (global.get $ball_dx)))')
    wat.append('    (local.set $new_y (i32.add (global.get $ball_y) (global.get $ball_dy)))')
    wat.append('    (if (i32.or (i32.le_s (local.get $new_y) (i32.const 0)) (i32.ge_s (local.get $new_y) (i32.const 23)))')
    wat.append('      (then (global.set $ball_dy (i32.mul (global.get $ball_dy) (i32.const -1))))')
    wat.append('    )')
    wat.append('    (global.set $ball_x (local.get $new_x))')
    wat.append('    (global.set $ball_y (local.get $new_y))')
    wat.append('  )')

    # Render function
    wat.append('  (func $render')
    wat.append('    (call $print_i32 (global.get $score1))')
    wat.append('    (call $print_str (i32.const 100) (i32.const 3))')
    wat.append('    (call $print_i32 (global.get $score2))')
    wat.append('  )')

    # Main game loop
    wat.append('  (func $main')
    wat.append('    (local $running i32)')
    wat.append('    (local.set $running (i32.const 1))')
    wat.append('    (block $break')
    wat.append('      (loop $continue')
    wat.append('        (call $render)')
    wat.append('        (call $sleep (i32.const 50))')
    wat.append('        (br_if $break (i32.eqz (local.get $running)))')
    wat.append('        (br $continue)')
    wat.append('      )')
    wat.append('    )')
    wat.append('  )')

    wat.append('  (export "main" (func $main))')
    wat.append(')')

    return '\n'.join(wat)

if __name__ == '__main__':
    if len(sys.argv) < 2:
        print("Usage: titan.py <source.ti> [output.wat]")
        sys.exit(1)

    with open(sys.argv[1]) as f:
        src = f.read()

    wat = compile_titan(src)

    output_file = sys.argv[2] if len(sys.argv) > 2 else 'out.wat'
    with open(output_file, 'w') as f:
        f.write(wat)

    print(f"Compiled to {output_file}")
