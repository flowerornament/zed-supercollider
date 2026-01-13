; SuperCollider runnables - detect evaluable code regions
;
; Play buttons appear on TOP-LEVEL statements only, matching SC IDE behavior:
;   - SC IDE Shift+Return: evaluate current line
;   - SC IDE Cmd+Return: evaluate parenthesized region containing cursor
;
; We only place buttons on top-level constructs. For nested blocks, users
; should select the region and use keyboard shortcuts (like SC IDE).
;
; Tree-sitter captures:
;   @run  → tells Zed where to place the ▶ play button
;   @code → content passed to task as $ZED_CUSTOM_code (case-sensitive!)
;   tag   → matches task with tags: ["sc-eval"] in tasks.json
;
; When user clicks play button, Zed runs the "Evaluate" task with the
; captured code in ZED_CUSTOM_code.
;
; NOTE: Nested code_blocks are intentionally NOT matched. The tree-sitter
; grammar wraps them in ERROR nodes which corrupts capture boundaries.
; See bd issue for potential future improvements.

; Multi-statement code blocks at top level only
(source_file
  (code_block) @code @run
  (#set! tag sc-eval))

; Top-level expressions - match various statement types at source_file level
; This prevents buttons on nested parens while allowing standalone line eval

(source_file
  (grouped_expression) @code @run
  (#set! tag sc-eval))

(source_file
  (function_call) @code @run
  (#set! tag sc-eval))

(source_file
  (binary_expression) @code @run
  (#set! tag sc-eval))

(source_file
  (variable_definition) @code @run
  (#set! tag sc-eval))
