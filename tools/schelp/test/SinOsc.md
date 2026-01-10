# SinOsc

*Interpolating sine wavetable oscillator.*

## Description

Generates a sine wave. Uses a wavetable lookup oscillator with linear interpolation. Frequency and phase modulation are provided for audio-rate modulation. Technically, `SinOsc` uses the same implementation as  [Osc](Classes/Osc.md)  except that its table is fixed to be a sine wave made of `8192` samples.

### Other sinewaves oscillators

- [FSinOsc](Classes/FSinOsc.md) -- fast sinewave oscillator
- [SinOscFB](Classes/SinOscFB.md) -- sinewave with phase feedback
- [PMOsc](Classes/PMOsc.md) -- phase modulation sine oscillator
- [Klang](Classes/Klang.md) -- bank of sinewave oscillators
- [DynKlang](Classes/DynKlang.md) -- modulable bank of sinewave oscillators

## Class Methods

#### `*ar, *kr`

**freq**:

Frequency in Hertz. Sampled at audio-rate.

**phase**:

Phase in radians. Sampled at audio-rate. **Note: **phase values should be within the range +-8pi. If your phase values are larger then simply use `.mod(2pi)` to wrap them.

**mul**:

Output will be multiplied by this value.

**add**:

This value will be added to the output.

## Examples

``` supercollider

// create an audio-rate sine wave at 200 Hz,
// starting with phase 0 and an amplitude of 0.5
{ SinOsc.ar(200, 0, 0.5) }.play;

// modulate the frequency with an exponential ramp
{ SinOsc.ar(XLine.kr(2000, 200), 0, 0.5) }.play;

// more complex frequency modulation
{ SinOsc.ar(SinOsc.ar(XLine.kr(1, 1000, 9), 0, 200, 800), 0, 0.25) }.play;

// phase modulation (see also PMOsc)
{ SinOsc.ar(800, SinOsc.ar(XLine.kr(1, 1000, 9), 0, 2pi), 0.25) }.play;
```
