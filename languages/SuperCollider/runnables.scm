; SuperCollider runnables - detect evaluable code regions
;
; In SuperCollider, code is typically evaluated in blocks:
; - Parenthesized blocks: ( ... )
; - Function blocks: { ... }
;
; The @run capture tells Zed where to place the play button.
; The @code capture (without underscore) becomes $ZED_CUSTOM_CODE.

; Parenthesized code blocks: ( expr; expr; ... )
((code_block) @code @run
  (#set! tag sc-eval))

; Function blocks: { |args| expr; expr; ... }
((function_block) @code @run
  (#set! tag sc-eval))
