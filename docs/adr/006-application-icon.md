# Application icon

The icon is an original record crate drawing in the aluminium tile style used by Auto Digger. The reference is `../auto-digger/resources/icons/icon.svg`, MIT licensed by Alexander Wanyoike. The new artwork uses a crate and record rather than the turntable mark.

SVG is the source. The 16, 32, 64, 256 and 512 pixel PNGs are rendered with ImageMagick. Iced loads the 256 pixel image into the native window icon through its window settings. The decode test runs without a desktop session.

Regenerate a size with `convert -background none resources/icons/openbox.svg -resize 256x256 resources/icons/openbox-256.png`.
