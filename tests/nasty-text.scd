// Test file with nasty text for eval-selection testing
// Select each block and evaluate to test edge cases

// 1. Basic string with quotes
"Hello \"world\" it's a 'test'".postln;

// 2. Double semicolons (SC idiom)
(
var x = 10;;
x.postln;;
)

// 3. Backticks and dollar signs
"Price: $100 with `code`".postln;

// 4. Pipes and ampersands
"a | b && c || d".postln;

// 5. Backslashes
"path\\to\\file".postln;
\symbol.postln;

// 6. Parentheses and braces galore
(
{
    ({
        [(1..10), (20..30)].postln;
    }).value;
}.value;
)

// 7. Multi-line string
"Line one
Line two
Line three".postln;

// 8. Special SC syntax
SinOsc.ar(440!2) * 0.1;

// 9. Comments inside code
(
// This is a comment with "quotes" and $pecial chars
var x = 42; // inline comment;;
x.postln;
)

// 10. Unicode and emoji (if your SC supports it)
"Caf\u00e9 \u2665".postln;

// 11. Empty parens and weird spacing
(    ).(    );

// 12. Regexp-like patterns
"test.*pattern[0-9]+".postln;

// 13. Shell-dangerous characters
"$(whoami) `id` ; rm -rf /".postln;

// 14. Nested quotes hell
"He said \"she said 'they said \\\"what\\\"'\"".postln;

// 15. Just semicolons
;;;;

// 16. Curly quote variants (if pasted from docs)
"smart quotes".postln;
