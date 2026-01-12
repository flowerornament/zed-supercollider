; SuperCollider runnables - detect evaluable code regions
;
; In SuperCollider, code is typically evaluated in parenthesized blocks.
; Tree-sitter captures:
;   @run  → tells Zed where to place the ▶ play button
;   @code → content passed to task as $ZED_CUSTOM_code (case-sensitive!)
;   tag   → matches task with tags: ["sc-eval"] in tasks.json
;
; When user clicks play button, Zed runs the "Evaluate" task with the
; captured code in ZED_CUSTOM_code.

; Multi-statement code blocks (with semicolons)
((code_block) @code @run
  (#set! tag sc-eval))

; Single-expression blocks at top level
((grouped_expression) @code @run
  (#set! tag sc-eval))
