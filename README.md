![C build](https://github.com/Polochon-street/bliss/workflows/C/badge.svg)

# Bliss music analyzer v1.2.0
Bliss music library is a multithreaded C library used to compute distance between songs. It is especially usable through MPD with [Blissify](https://github.com/Phyks/Blissify).
It is can be useful for creating « intelligent » playlists and/or continuous play, à la Spotify/Grooveshark Radio. <br />
Bliss is really useful when used as a plug-in for audio players, so feel free to use the python bindings to develop one for your favorite player! <br />
See ANALYSIS.md for a technical description of the project.

NOTE: Currently rewriting and enhancing it in Rust, after prototyping something better than the current Bliss for my Msc thesis. Stay tuned!<br />
See ROADMAP.md for a (very incomplete) list of what's to come.

## Current projects using Bliss 
* MPD thanks to [Blissify](https://github.com/Phyks/Blissify)
* [leleleplayer](https://github.com/Polochon-street/leleleplayer)

## Usage
* The main purpose of the library is to extract features from songs in the form of coordinates (tempo, amplitude, frequency, attack).
* Use `bl_analyze()` to compute these coordinates for a given song.
* Use `bl_distance_file()` to compute the euclidian distance between two songs. The closer the songs are, the more similar they are. Two same songs have a distance that tends towards 0. (This function is merely a wrapper for calling `bl_analyze()` for each song and computing their euclidian distance)
* Python bindings are also available for these functions. See [the wiki](https://github.com/Polochon-street/bliss/wiki/Python-Bindings) to learn how to use them. <br /> <br />
These two functions are just examples of what can be done with coordinates in an euclidian space; machine-learning/big data algorithms could also be used to make cool things, such as clustering. See this [article](https://linuxfr.org/news/sortie-de-la-bibliotheque-d-analyse-musicale-bliss-1-0#performances) (in French)<br /><br />
The most immediate thing one that can be done to test this library could be to install it and then run python/examples/make\_m3u\_playlist.py on a folder with random songs in it: it will try to build the best playlist out of the files in the directory.
## Dependencies

* libavformat
* libavutil
* libavcodec
* libswresample (or libavresample, if libswresample isn't present)
* libfftw3

If you are running Ubuntu (e.g. 14.04), you should `apt-get install libavutil-dev libavformat-dev libavcodec-dev libavresample-dev libfftw3-dev`.

If you are running Arch Linux, `pacman -S ffmpeg` should be enough.

For the Python bindings

* python-cffi
* python-setuptools

## Installation

### Linux users

* clone repository on github
```bash
$ git clone https://github.com/Polochon-street/bliss.git
```
* go to bliss root directory
```bash
$ cd bliss
```
* Create and enter the build directory
```bash
$ mkdir build && cd build
```
* Generate the Makefile
```bash
$ cmake .. -DCMAKE_BUILD_TYPE=Release
```
* Compile the library
```bash
$ make
```
* Install the library
```bash
(root) make install
```
* (optional) Install the python bindings
```bash
(root) cd python && python setup.py install
```

## Usage examples
* See examples/analyze.c and examples/distance.c
* Compile any project using bliss with
```bash
$ gcc -o example example.c -lbliss $(pkg-config --cflags libavutil libavformat libavcodec)
```
* Examples for python bindings are in python/examples

## Unittests
This library comes with some unittests. To build them, just run
```
$ make test
```
in the `build/` folder. Unittests source files can be found in the `tests/` folder.
