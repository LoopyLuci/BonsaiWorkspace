/// Tree-sitter grammar for Omnisystem Titan language
///
/// Titan is Omnisystem's effect-tracking systems language with:
/// - Effect type annotations (Pure, IO, State, etc.)
/// - Resource management and linear types
/// - Actor-like parallelism
/// - Formal verification integration

module.exports = grammar({
  name: 'titan',

  rules: {
    source_file: $ => repeat($._statement),

    _statement: $ => choice(
      $.function_definition,
      $.type_definition,
      $.effect_declaration,
      $.import_statement,
      ';'
    ),

    function_definition: $ => seq(
      optional($.visibility),
      'fn',
      $.identifier,
      $.type_parameters,
      $.parameters,
      optional($.effect_annotation),
      '->',
      $.type,
      $.block
    ),

    type_definition: $ => seq(
      optional($.visibility),
      'type',
      $.identifier,
      '=',
      $.type
    ),

    effect_declaration: $ => seq(
      'effect',
      $.identifier,
      '{',
      repeat($.effect_operation),
      '}'
    ),

    effect_operation: $ => seq(
      $.identifier,
      ':',
      $.type,
      ','
    ),

    effect_annotation: $ => seq(
      '@',
      choice(
        'pure',
        'io',
        'state',
        'async',
        'unchecked'
      )
    ),

    visibility: $ => choice('pub', 'priv'),

    type_parameters: $ => seq('[', commaSep($.type_parameter), ']'),

    type_parameter: $ => seq(
      $.identifier,
      optional(seq(':', $.trait_bound))
    ),

    trait_bound: $ => commaSep($.identifier),

    parameters: $ => seq(
      '(',
      commaSep($.parameter),
      ')'
    ),

    parameter: $ => seq(
      $.identifier,
      ':',
      $.type
    ),

    type: $ => choice(
      $.identifier,
      $.generic_type,
      $.function_type,
      $.effect_type
    ),

    generic_type: $ => seq(
      $.identifier,
      '<',
      commaSep($.type),
      '>'
    ),

    function_type: $ => seq(
      'fn',
      $.type_parameters,
      $.parameter_types,
      '->',
      $.type
    ),

    parameter_types: $ => seq(
      '(',
      commaSep($.type),
      ')'
    ),

    effect_type: $ => seq(
      '{',
      commaSep(choice(
        'Pure',
        'IO',
        'State',
        'Async',
        'Panic',
        'Custom(' + $.identifier + ')'
      )),
      '}'
    ),

    import_statement: $ => seq(
      'import',
      $.module_path,
      optional(seq('{', commaSep($.identifier), '}')),
      ';'
    ),

    module_path: $ => sepBy('.', $.identifier),

    block: $ => seq(
      '{',
      repeat($._statement),
      '}'
    ),

    identifier: $ => /[a-zA-Z_][a-zA-Z0-9_]*/,

    comment: $ => token(choice(
      seq('//', /.*/),
      seq('/*', /[\s\S]*?/, '*/')
    )),

    _whitespace: $ => /\s+/,
  },

  extras: $ => [$.comment, $.whitespace],
  word: $ => $.identifier
});

function commaSep(rule) {
  return optional(sepBy(',', rule));
}

function sepBy(sep, rule) {
  return seq(rule, repeat(seq(sep, rule)));
}
