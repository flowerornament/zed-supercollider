; SuperCollider runnables - detect evaluable code regions
;
; In SuperCollider, code is typically evaluated in parenthesized blocks.
; Tree-sitter captures:
;   @run  → tells Zed where to place the ▶ play button
;   @code → content passed to task as $ZED_CUSTOM_CODE (the code to evaluate)
;   tag   → matches task with tags: ["sc-eval"] in tasks.json
;
; When user clicks play button, Zed runs the "Evaluate" task with the
; code_block content in ZED_CUSTOM_CODE.

((code_block) @code @run
  (#set! tag sc-eval))
