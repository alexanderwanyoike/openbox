# 006: Identify the native Linux window

Status: accepted.

A window icon alone does not identify an application to the desktop. Openbox previously supplied its bitmap but left WM_CLASS empty, allowing desktop launchers to group it under another application's icon.

The Iced platform application ID now matches the reverse-domain desktop entry, its icon name and StartupWMClass: io.github.alexanderwanyoike.openbox. This follows the standard freedesktop desktop-entry convention through Iced's existing native window settings. The bundled PNG remains the native window icon.

The isolated X11 regression first failed on the empty WM_CLASS and then passed with the Openbox ID and 256-pixel window icon. Existing windows must be reopened to receive their new identity. Verification does not change another application's launcher or interrupt the running user session.
