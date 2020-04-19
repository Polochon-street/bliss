# Roadmap for bliss-rs

This is a rough roadmap for where I want to go for `bliss-rs`.

Features are freely inspired from the corresponding discussion in my
[Msc thesis](https://polochon.lelele.io/thesis.pdf).

## Features

### Timbral features

* Zero-crossing rate
* Spectral centroid
* Spectral rolloff
* Spectral flatness
* MFCC (maybe?)

### Temporal features

* BPM
* Beat loudness

### Loudness features

* Rough measure of the dB level of songs

### Tonal features

* Chromagram / HPCP

## Summarization / normalization

Find something better than the mean to summarize these features.
Maybe variance + mean would be enough, maybe media, maybe sth else?

Also, normalize everything.

## Usability

Work on making this actually usable as a library, and then make python bindings

## ???

Profit.
