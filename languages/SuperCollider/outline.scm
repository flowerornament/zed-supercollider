; Outline items must use the `@item` capture in Zed.
; Keep names in `@name` for display text.

; Class definitions
(class_def
  (class) @name) @item

; Instance methods within a class
(class_def
  (_)*
  (instance_method_name) @name
  (_)* ) @item

; Class (static) methods within a class
(class_def
  (_)*
  (class_method_name) @name
  (_)* ) @item

; Fallback: standalone method names (outside class)
(instance_method_name) @name @item
(class_method_name) @name @item
