# Osc

*Interpolating wavetable oscillator.*

## Description

Linear interpolating wavetable lookup oscillator with frequency and phase modulation inputs.

This oscillator requires a buffer to be filled with a wavetable format signal. This preprocesses the Signal into a form which can be used efficiently by the Oscillator. The buffer size must be a power of 2.

This can be achieved by creating a Buffer object and sending it one of the \"b_gen\" messages ([Buffer.-sine1](Classes/Buffer.md#-sine1), [Buffer.-sine2](Classes/Buffer.md#-sine2), [Buffer.-sine3](Classes/Buffer.md#-sine3)) with the wavetable flag set to true.

This can also be achieved by creating a [Signal](Classes/Signal.md) object and sending it the \'asWavetable\' message, thereby creating a Wavetable object in the required format. Then, the wavetable data may be transmitted to the server using the [Buffer.sendCollection](Classes/Buffer.md#sendcollection) or [Buffer.loadCollection](Classes/Buffer.md#loadcollection) methods.

## Class Methods

#### `*ar, *kr`

**bufnum**:

Buffer index.

**freq**:

Frequency in Hertz.

**phase**:

Phase offset or modulator in radians. (Note: phase values should be within the range +-8pi. If your phase values are larger then simply use `.mod(2pi)` to wrap them.)

**mul**:

Output will be multiplied by this value.

**add**:

This value will be added to the output.

## Examples

``` supercollider

(
s = Server.local;
b = Buffer.alloc(s, 512, 1);
b.sine1(1.0/[1, 2, 3, 4, 5, 6], true, true, true);

SynthDef("help-Osc", { |out = 0, bufnum = 0|
    Out.ar(out,
        Osc.ar(bufnum, 200, 0, 0.5)
    )
}).play(s, [\out, 0, \bufnum, b.bufnum]);
)

(
s = Server.local;
b = Buffer.alloc(s, 512, 1);
b.sine1(1.0/[1, 2, 3, 4, 5, 6], true, true, true);

SynthDef("help-Osc", { |out = 0, bufnum = 0|
    Out.ar(out,
        Osc.ar(bufnum, XLine.kr(2000, 200), 0, 0.5) // modulate freq
    )
}).play(s, [\out, 0, \bufnum, b.bufnum]);
)


(
s = Server.local;
b = Buffer.alloc(s, 512, 1);
b.sine1([1.0], true, true, true);

SynthDef("help-Osc", { |out = 0, bufnum = 0|
    Out.ar(out,
        Osc.ar(bufnum,
            Osc.ar(bufnum,
                XLine.kr(1, 1000, 9),
                0,
                200,
                800),
            0,
            0.25)
    )
}).play(s, [\out, 0, \bufnum, b.bufnum]);
)


(
// modulate phase
s = Server.local;
b = Buffer.alloc(s, 512, 1);
b.sine1([1.0], true, true, true);

SynthDef("help-Osc", { |out = 0, bufnum = 0|
    Out.ar(out,
        Osc.ar(bufnum,
                800,
                Osc.ar(bufnum,
                        XLine.kr(20, 8000, 10),
                        0,
                        2pi),
                0.25)
    )
}).play(s, [\out, 0, \bufnum, b.bufnum]);
)



(
// change the buffer while its playing
s = Server.local;
b = Buffer.alloc(s, 4096, 1);
b.sine1(1.0/[1, 2, 3, 4, 5, 6], true, true, true);

SynthDef("help-Osc", { |out = 0, bufnum = 0|
    Out.ar(out,
        Osc.ar(bufnum, [80, 80.2], 0, 0.2)
    )
}).play(s, [\out, 0, \bufnum, b.bufnum]);
)

(
fork {
    var n = 32;
    50.do {
        b.sine1(Array.rand(n, 0, 1).cubed, true, true, true);
        0.25.wait;
    };
};
)
```
