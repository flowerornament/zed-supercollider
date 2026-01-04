// SuperCollider .scd demo for Zed extension testing
//
// What to try:
// - Open this file and check the Outline for class/methods.
// - Evaluate the selection/blocks below to hear sound (no class required).
// - Class definitions require .sc files + recompile to instantiate.

/* ===== 1) Quick smoke: boot + simple SynthDef ===== */
(
s.waitForBoot {
var def = SynthDef(\beep, {
    var freq = \freq.kr(440);
    var amp  = \amp.kr(0.1);
    var sig  = SinOsc.ar(freq) * amp;
    Out.ar(0, sig.dup);
});
def.add;
"Added SynthDef(\\beep). Try: x = Synth(\\beep); x.set(\\freq, 660); x.free;".postln;
};
)

s.boot;

/* ===== 2) Example usage (evaluate line-by-line) ===== */
// x = Synth(\beep);
// x.set(\freq, 660);
// x.free;

/* ===== 3) Class definition (for LSP/outline testing) ===== */
// Note: To instantiate this class at runtime, place it in a .sc file under
// your Extensions dir and use thisProcess.recompile.

BeepSynth {
    classvar <>defaultFreq = 440;  // class var with accessor
    classvar <>defaultAmp  = 0.1;
    var <amp;                          // instance var with read accessor

    *new { arg amp=defaultAmp;        // class method (constructor)
        ^super.new.init(amp)
    }

    init { arg amp;                   // instance initializer
        this.amp = amp;
        ^this
    }

    *def { arg freq=defaultFreq, amp=defaultAmp;  // class method returning a SynthDef
        ^SynthDef(\beep2, {
            var sig = SinOsc.ar(freq) * amp;
            Out.ar(0, sig.dup)
        })
    }

    make { arg freq=defaultFreq;      // instance method: produce a SynthDef using instance amp
        ^BeepSynth.def(freq, this.amp)
    }
}

/* ===== 4) Hypothetical usage after compile (for reference) ===== */
// thisProcess.recompile;  // after moving class to an Extensions .sc file
// b = BeepSynth.new(0.2);
// d = b.make(660);
// d.add; x = Synth(\beep2); // x.free;
