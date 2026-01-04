; Minimal highlights aligned with tree-sitter-supercollider

; Comments
(line_comment) @comment
(block_comment) @comment

; Literals
(number) @number
(string) @string

; Identifiers and types
(class) @type
(identifier) @variable

; Methods
(method_name) @function
(class_method_name) @function
(instance_method_name) @function
