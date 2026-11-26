; The structural highlights, read off the tree. The keyword, operator and
; punctuation lists above are printed from the grammar value.

(comment) @comment

(string) @string
(triple_string) @string
(template_string) @string
(escape) @string.escape
(brace_escape) @string.escape
(format_spec) @string.special
(substitution
  "{" @punctuation.special
  "}" @punctuation.special)

(number) @number
(boolean) @boolean
(none) @constant.builtin

(doc_declaration
  text: (triple_string) @comment.doc)

(function_definition
  name: (identifier) @function)
(call_expression
  function: (identifier) @function)
(call_expression
  function: (qualified_identifier
    name: (identifier) @function))

(fiber_declaration
  name: (identifier) @type)
(struct_declaration
  name: (identifier) @type)
(protocol_declaration
  name: (identifier) @type)
(fiber_type
  name: (identifier) @type)
(fiber_type
  name: (qualified_identifier
    name: (identifier) @type))
(constructor_pattern
  head: (identifier) @type)
(variant
  name: (identifier) @variant)
(match_arm
  variant: (identifier) @variant)
(component
  name: (identifier) @property)
(struct_entry
  name: (identifier) @property)
(map_entry
  key: (identifier) @property)

(port_declaration
  name: (identifier) @variable.special)
(parameter
  name: (identifier) @variable.parameter)
