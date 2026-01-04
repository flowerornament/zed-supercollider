// SuperCollider Evaluation Test
// Click the play button next to any code block to evaluate it

// ============================================
// BASIC EVALUATION
// ============================================

// Simple expression
(
1 + 1
)

// Print to post window
(
"Hello from SuperCollider!".postln;
)

// Math with variables
(
var x = 10;
var y = 20;
x + y
)

// ============================================
// AUDIO - Boot server first!
// ============================================

// Boot the audio server (run this first)
(
s.boot;
)

// Simple sine tone at 440Hz (A4)
(
{SinOsc.ar(440, 0, 0.2)}.play;
)

// Stereo with slight detuning
(
{
    var freq = 440;
    SinOsc.ar([freq, freq * 1.01], 0, 0.1)
}.play;
)

// FM synthesis
(
{
    var mod = SinOsc.ar(200, 0, 100);
    SinOsc.ar(440 + mod, 0, 0.2)
}.play;
)

// Filtered noise
(
{
    var noise = WhiteNoise.ar(0.5);
    RLPF.ar(noise, MouseX.kr(100, 10000, 1), 0.1)
}.play;
)

// ============================================
// STOP SOUND
// ============================================

// Stop all synths (or use the "SuperCollider: Stop" task)
(
CmdPeriod.run;
)

// Quit the audio server
(
s.quit;
)
