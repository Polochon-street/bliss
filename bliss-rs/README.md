[![Build Status](https://travis-ci.com/Polochon-street/bliss.svg?branch=master)](https://travis-ci.com/Polochon-street/bliss)

# Bliss music analyser - Rust version
Bliss-rs the Rust improvement of the first iteration of Bliss. The data it
outputs is not compatible with the ones used by Bliss, since it uses
different, more accurate features, based on actual literature this time.

Like Bliss, it ease the creation of « intelligent » playlists and/or continuous
play, à la Spotify/Grooveshark Radio.

## Usage
Currently not usable.

It's missing a binary / example to actually use the computed features.

## Acknowledgements

* This library relies heavily on [aubio](https://aubio.org/)'s
  [Rust bindings](https://crates.io/crates/aubio-rs) for the spectral /
  timbral analysis, so a big thanks to both the creators and contributors
  of librosa, and to @katyo for making aubio bindings for Rust.
* The first part of the chroma extraction is basically a rewrite of
  [librosa](https://librosa.org/doc/latest/index.html)'s
  [chroma feature extraction](https://librosa.org/doc/latest/generated/librosa.feature.chroma_stft.html?highlight=chroma#librosa.feature.chroma_stftfrom)
  from python to Rust, with just as little features as needed. Thanks
  to both creators and contributors as well.
* Finally, a big thanks to
  [Christof Weiss](https://www.audiolabs-erlangen.de/fau/assistant/weiss)
  for pointing me in the right direction for the chroma feature summarization,
  which are basically also a rewrite from Python to Rust of some of the
  awesome notebooks by AudioLabs Erlangen, that you can find
  [here](https://www.audiolabs-erlangen.de/resources/MIR/FMP/C0/C0.html).
