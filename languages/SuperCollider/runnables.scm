; SuperCollider runnables - detect evaluable code regions
;
; In SuperCollider, code is typically evaluated in parenthesized blocks.
; The @run capture tells Zed where to place the play button.
; The @code capture (without underscore) becomes $ZED_CUSTOM_CODE.

((code_block) @code @run
  (#set! tag sc-eval))
