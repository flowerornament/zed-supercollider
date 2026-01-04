// Test file for runnables detection
// Each block below should show a play button in the gutter

// Simple code block
(
1 + 1
)

// Multi-line code block
(
var x = 10;
var y = 20;
x + y
)

// Function block
{
    |freq = 440|
    SinOsc.ar(freq)
}

// Nested blocks - outer should have play button
(
SynthDef(\test, {
    var sig = SinOsc.ar(440);
    Out.ar(0, sig);
}).add;
)

// Single line for comparison (no block, no play button expected)
1 + 1;
