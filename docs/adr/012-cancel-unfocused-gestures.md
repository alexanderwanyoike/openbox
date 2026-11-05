# 012: Cancel custom gestures when the window loses focus

Status: accepted. Extends ADR 005.

Waveform scrubbing and mixer knob drags retained their gesture state when the window lost focus. Returning the pointer could issue an unexpected seek or change a mixer value. An overview seek outside its bounds clamps to the track end, so a stale drag can look like a track racing forward.

Follow Iced 0.14.2's existing text-input focus handling: a window focus event ends transient interaction state. No upstream source is copied. Waveforms clear their active drag on `Window::Unfocused`; knobs also clear double-click and modifier state. Focus loss neither publishes a seek nor changes a control value. A new press starts a new gesture.

The regressions exercise the actual canvas event interfaces for both overview and detailed waveforms and the mixer knob. They failed before the implementation. A native Iced check on a private Xvfb display changed the trim, moved focus to another private window while holding the mouse button, then moved the pointer again. The displayed value remained unchanged after focus loss. This uses a private output sink and generated temporary library, without touching the user's desktop, music or USB.

This fixes a separate gesture lifecycle defect. It does not replace the position/reference-time correction in ADR 009 or establish a new audio playback fault.
