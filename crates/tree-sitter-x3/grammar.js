; Tree-sitter grammar for X3 language
; X3: Dual-VM DeFi DSL with AI + ZK

(program
  (module)*)

(module
  "import" (import_spec)*
  "export" (export_spec)*
  (item)*)

(import_spec
  (identifier)
  "::" (identifier))

(export_spec
  (identifier)
  (type))

(item
  (function_def)
  | (struct_def)
  | (enum_def)
  | (event_def)
  | (error_def)
  | (strategy_def))

(function_def
  (visibility)?
  "fn" (identifier)
  "(" (parameter_list)? ")"
  ("->" (type))?
  "{" (statement)* "}")

(visibility
  "pub"
  | "export")

(parameter_list
  (parameter) ("," (parameter))*)

(parameter
  (identifier) ":" (type)
  | "self"
  | "&" "self"
  | "&" "mut" "self")

(struct_def
  "struct" (identifier) "{"
  (field_def) ("," (field_def))* ","?
  "}")

(field_def
  (identifier) ":" (type))

(enum_def
  "enum" (identifier) "{"
  (variant) ("," (variant))* ","?
  "}")

(variant
  (identifier) ("(" (type) ("," (type))* ")")?))

(event_def
  "event" (identifier) "{"
  (field_def) ("," (field_def))* ","?
  "}")

(error_def
  "error" (identifier) "(" (string_literal) ")")

(strategy_def
  "strategy" (identifier) "{"
  (field_def)*
  (function_def)*
  "}")

(statement
  (let_stmt)
  | (assign_stmt)
  | (if_stmt)
  | (while_stmt)
  | (for_stmt)
  | (return_stmt)
  | (expr_stmt))

(let_stmt
  "let" "mut"? (identifier) (":" (type))? "=" (expression) ";")

(assign_stmt
  (identifier) "=" (expression) ";")

(if_stmt
  "if" (expression) "{" (statement)* "}"
  ("else" "{" (statement)* "}")?)

(while_stmt
  "while" (expression) "{" (statement)* "}")

(for_stmt
  "for" (identifier) "in" (expression) "{" (statement)* "}")

(return_stmt
  "return" (expression)? ";")

(expr_stmt
  (expression) ";")

(expression
  (binary_op)
  | (unary_op)
  | (call_expr)
  | (match_expr)
  | (primary))

(binary_op
  (expression) (binary_operator) (expression))

(binary_operator
  "+"
  | "-"
  | "*"
  | "/"
  | "%"
  | "=="
  | "!="
  | "<"
  | ">"
  | "<="
  | ">="
  | "&&"
  | "||"
  | "&"
  | "|"
  | "^"
  | "<<"
  | ">>")

(unary_op
  (unary_operator) (expression))

(unary_operator
  "-"
  | "!"
  | "&"
  | "&mut")

(call_expr
  (identifier) "(" (argument_list)? ")")

(argument_list
  (expression) ("," (expression))*)

(match_expr
  "match" (expression) "{"
  (match_arm) ("," (match_arm))* ","?
  "}")

(match_arm
  (pattern) "=>" (expression))

(pattern
  (identifier)
  | (number_literal)
  | (string_literal)
  | (constructor_pattern))

(constructor_pattern
  (identifier) "(" (pattern) ("," (pattern))* ")")

(primary
  (number_literal)
  | (string_literal)
  | (identifier)
  | (boolean_literal)
  | (array_literal)
  | (struct_literal)
  | (paren_expr)
  | (attribute))

(number_literal
  /[0-9]+/)

(string_literal
  /"[^"]*"/)

(identifier
  /[a-zA-Z_][a-zA-Z0-9_]*/)

(boolean_literal
  "true"
  | "false")

(array_literal
  "[" (expression) ("," (expression))* ","? "]")

(struct_literal
  (identifier) "{"
  (field_assign) ("," (field_assign))* ","?
  "}")

(field_assign
  (identifier) ":" (expression))

(paren_expr
  "(" (expression) ")")

(attribute
  "@" (identifier) ("(" (attribute_arg) ("," (attribute_arg))* ")")?
  (expression)?)

(attribute_arg
  (string_literal)
  | (identifier)
  | (expression))

(type
  (builtin_type)
  | (identifier)
  | (option_type)
  | (array_type)
  | (ref_type))

(builtin_type
  "u8"
  | "u16"
  | "u32"
  | "u64"
  | "u128"
  | "bool"
  | "string"
  | "bytes"
  | "bytes20"
  | "bytes32")

(option_type
  "Option" "<" (type) ">")

(array_type
  (type) "[" "]")

(ref_type
  "&" "mut"? (type))
