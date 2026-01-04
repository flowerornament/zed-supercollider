; Use a single required capture `@indent` to mark lines that should indent.
; Zed requires `indent` capture; begin/end are not recognized.
; Indent after opening braces and parentheses; outdent on closing is implicit.

(("{")) @indent
(("[")) @indent
(("(")) @indent
