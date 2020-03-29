Fonts in this folder are enabled by using cargo feature flags.

These are not enabled by default. Special care is taken to only offer for bundling fonts that are licensed using the Apache 2.0 License. If you enable the font features, you will need to make sure you comply with the licensing requirements of the fonts.

To bundle all fonts, enable the feature "bundled-fonts":

`kludgine = { version = ..., features=["bundled-fonts"]}`

To enable individual fonts, you can use the lowercase folder name:

`kludgine = { version = ..., features=["bundled-fonts-roboto"]}`
