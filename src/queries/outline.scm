(function_definition
  name: (identifier) @name) @item

(binding
  name: (identifier) @name) @item

(port_declaration
  kind: _ @context
  name: (identifier) @name) @item

(fiber_declaration
  "fiber" @context
  name: (identifier) @name) @item

(struct_declaration
  "struct" @context
  name: (identifier) @name) @item

(namespace_definition
  "namespace" @context
  name: (identifier) @name) @item

(step_block
  "step" @name) @item
