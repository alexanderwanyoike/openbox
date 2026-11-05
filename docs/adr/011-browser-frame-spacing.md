# 011: Browser frames and spacing

Status: Accepted

The collection and USB browsers previously used touching rounded cards, a separate square banner and square action bars. Their search rows did not align, and footer actions overflowed the minimum window width.

Follow the shared workspace surface pattern used by Showbiz's `shot-workspace.css` and `workspace-controls.css`: one frame owns the outer corners, and internal panes meet at a straight divider. Playlist management belongs in one shared toolbar. Pane headings, searches and action bars use the same inset.

| Element | Size |
|---|---:|
| Workspace corner | 12 px |
| Control corner | 6 px |
| Pane inset | 16 px |
| Control gap | 8 px |
| Browser control height | 32 px |

Transport controls retain their compact height. Footer corners apply only at the outside edges; a delete confirmation owns the lower corners while visible. Folder views use the same workspace frame and inset.

At narrow widths, the selection summary becomes a clear-selection control, playlist choices narrow to 140 px, and reorder/removal use icon buttons with tooltips. No playlist action is removed. Rendered tests cover corner continuity, name input and reachable removal at 1000 px. Native verification covers both modes at 1000, 1100, 1440 and 1920 px, plus rename, bulk transfer, remove and Undo with generated audio.
