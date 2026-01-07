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

// Alternate boot: set options before boot
(
Server.default.options.sampleRate = 48000;
Server.default.options.memSize = 2.pow(18);
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

// Envelopes and release
(
{
    var env = Env.perc(0.01, 1.5, 0.2, -4).ar(doneAction: 2);
    SinOsc.ar(330, 0, env)
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

// Polyphonic event using Pbind (needs a server running)
(
{
    Pbind(
        \\instrument, \\default,
        \\dur, 0.25,
        \\freq, Pseq([220, 330, 440, 550], inf),
        \\amp, 0.1
    ).play;
}.fork; // run in a lightweight thread
)

// Nested blocks to check brace handling
(
{
    var a = {
        var b = { 1 + 2 };
        b.value + 3
    };
    a.value.postln;
}.value;
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
