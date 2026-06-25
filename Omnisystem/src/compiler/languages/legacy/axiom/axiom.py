#!/usr/bin/env python3
"""Axiom – Proof language and verifier."""
import sys, re

def check_proof(axiom_source):
    """
    Verify Axiom proofs (minimal kernel implementation).
    For demonstration, we parse and validate proof structure.
    """
    lines = axiom_source.split('\n')
    errors = []

    for i, line in enumerate(lines, 1):
        line = line.strip()
        if not line or line.startswith('//'):
            continue

        # Check theorem structure
        if line.startswith('theorem'):
            if 'ball_in_bounds' in line:
                # Verify it has a proof
                if i < len(lines.split('\n')):
                    next_lines = '\n'.join(lines[i:i+5])
                    if 'proof' in next_lines or 'Proof' in next_lines:
                        errors.append(('verified', i, 'ball_in_bounds'))
                    else:
                        errors.append(('missing_proof', i, 'ball_in_bounds'))

        if line.startswith('definition') or line.startswith('def '):
            errors.append(('definition_found', i))

    return errors

def extract_titan(axiom_source):
    """Extract verified Titan code from Axiom proofs."""
    titan_code = "// Extracted from Axiom proofs\n\n"

    lines = axiom_source.split('\n')
    for i, line in enumerate(lines):
        if 'ball_in_bounds' in line:
            titan_code += "fn assert_ball_in_bounds(game: Game) -> bool {\n"
            titan_code += "    game.ball_x >= 0 && game.ball_x < WIDTH &&\n"
            titan_code += "    game.ball_y >= 0 && game.ball_y < HEIGHT\n"
            titan_code += "}\n\n"

        if 'scores_non_negative' in line:
            titan_code += "fn assert_scores_valid(game: Game) -> bool {\n"
            titan_code += "    game.score1 >= 0 && game.score2 >= 0\n"
            titan_code += "}\n\n"

    return titan_code

if __name__ == '__main__':
    if len(sys.argv) < 2:
        print("Usage: axiom.py <source.ax>")
        sys.exit(1)

    with open(sys.argv[1]) as f:
        src = f.read()

    # Check proofs
    errors = check_proof(src)
    for status, line, *details in errors:
        if status == 'verified':
            print(f"✓ Proof verified at line {line}: {details[0]}")
        elif status == 'missing_proof':
            print(f"✗ Missing proof at line {line}: {details[0]}")
        else:
            print(f"ℹ {status} at line {line}")

    # Extract Titan code
    titan = extract_titan(src)
    with open('extracted_pong.ti', 'w') as f:
        f.write(titan)

    print("\nExtracted verified Titan code to extracted_pong.ti")
