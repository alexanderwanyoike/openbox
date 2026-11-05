# 005: Separate waveform animation from library redraws

Status: accepted. Extends ADR 003.

Dragging a mixer control rebuilt and filtered the library, and waveform movement depended on a 33 ms application timer. The native capture harness measured roughly 23 distinct waveform frames per second. Repeated rectangular peak bars also produced coarse edges and flattened the visible shape.

The browser uses Iced's lazy widget with an explicit content revision. Library/navigation/selection events invalidate it; audio snapshots and mixer controls do not. Its widgets own their display strings so the cached tree cannot borrow outdated library records. Metadata refresh remains tied to completed imports/reanalysis, rather than every mixer event.

The audio crate builds a 500-bin-per-second envelope and a hierarchy of coarser levels while decoding a selected clip in the background. Each point reads at most two bins, including full-track views. Stereo channels contribute independently to three crossover band energies so phase cancellation cannot erase their waveform. This is display analysis and does not change music analysis or USB exports.

The UI paints connected mirrored shapes, with blue low-frequency energy, amber mids and a pale high-frequency core. Passed audio has a warm outer layer. The overview geometry is cached per clip and dimensions. Beat ticks, bar numbers, cue and playhead remain separate overlays.

Playing waveform canvases request redraws on the next display frame and interpolate the audio cursor using its audio-buffer timestamp and playback rate. Snapshots, clocks and meters refresh at 80 ms. Position clamps at the clip end and freezes when paused. Seeking resets the interpolation anchor. Canvas redraws do not send application messages or rebuild the browser.

The application uses Iced’s GPU-first fallback renderer, an established wgpu composition path, with tiny-skia retained when GPU initialization fails. Software rendering remains selectable through Cargo features. Moving ruler labels use outlined glyph paths with bounded damage regions because software canvas text otherwise invalidates the entire viewport. The development profile optimizes application code and uses release-like settings for tiny-skia. Debug and overflow checks remain enabled for application, library and USB code.

Knob drags clear double-click tracking once they move, support Shift for finer motion, and suppress unchanged value messages. Icon tooltips wait 500 ms to avoid immediately covering adjacent controls. Native screenshot capture checks visible motion; recorded audio still verifies control behavior.

Showbiz (`../showbiz/frontend/src/globals.css` and `desktop.css`, MIT) informs the neutral Native material, compact chrome, inset rounded panels and restrained blue selections. Other Showbiz material tokens are adapted into explicit Rust palettes. Mode, material and accent compose independently; waveform colour choices remain independent, with a matching surface and contrast variant for each application mode. The settings dialog preserves the underlying widget tree and blocks input to the dimmed workspace. Preferences save atomically in a separate settings file.

The native shell also follows Showbiz's integrated 48-pixel app chrome and separate 48-pixel workspace toolbar. Iced AW 0.14.1 supplies the menu tree and pointer handling, rather than a custom popup implementation. Iced's window tasks provide caption dragging, minimize, maximize/restore, close and corner resizing. Native decorations are disabled to avoid a duplicate title bar. Manrope faces match Showbiz's typography; their OFL is bundled. Collection navigation is a separate module, uses a 212-pixel inset panel and keeps at least 260 pixels of tree viewport at a 900-pixel window height with the decks visible. Creation remains scoped to the selected folder through a compact name field and playlist/folder icon actions.


## Reported defects

The display clock follows the established reference-time approach used by [Mixxx VisualPlayPosition](https://github.com/mixxxdj/mixxx/blob/main/src/waveform/visualplayposition.cpp). No Mixxx code is copied. CPAL supplies the playback lead; each snapshot describes the end of the queued buffer at that reference instant. Interpolation can therefore be negative when the reference is in the future. UI receipt time is not an audio clock. All canvases use the window redraw timestamp and request Iced's next frame rather than maintaining another 16 ms timer.

Native Iced sliders retain keyboard modifiers, hit testing, drag handling and default reset. Shared material caps, neutral rails and scale marks distinguish faders from progress indicators. The channel caps use the same styling. Deck layout tests preserve the entire reset control and bottom padding at minimum and wide window sizes.

All ten waveform colours have light and dark variants. Cached overview geometry and settings previews include mode in their invalidation key. Tooltip surfaces are opaque, bounded and wrapped. Table cells clip their contents and expose full metadata on hover.

Preview failures use a dismissible notice. Decoder details are collapsed initially, and only output failures offer an audio-output retry. Native Xvfb fixtures exercise both modes, missing files, tooltips and controls without touching the user's desktop or music. Physical display smoothness still needs the user's next playback test.
