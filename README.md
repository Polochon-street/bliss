# Bliss music analyzer v1.0.0
Bliss music library is a C library used to compute distance between songs. It is especially used in [leleleplayer](https://github.com/Polochon-street/leleleplayer).
It is can be useful for creating « intelligent » playlists, for instance.
See below for a technical description of the project.

## Usage
* Use `bl_cosine_similarity_file()` to compute the [cosine similarity](https://en.wikipedia.org/wiki/Cosine_similarity) of two songs:
![Graph from -1 to 1, 1 = close songs, -1 = opposite songs](https://cloud.githubusercontent.com/assets/9823290/11535215/31b59a18-9913-11e5-84c9-6d9ac22d4778.png)
* Use `bl_distance_file()` to compute the euclidian distance between two songs. If the distance is < 5, the songs are really similar; between 5 and 10, they are quite similar, between 10 and 15 means quite opposite, and > 15 « total opposites ».
* Combine both functions to obtain a better result - for example, a good condition to find similar songs would be « if cosine_distance >= 0.90 AND distance <= 5 then... » .
* Python bindings are also available. See [the wiki](https://github.com/Polochon-street/bliss/wiki/Python-Bindings) to use learn how to use them.

## Dependencies

* libavformat
* libavutil
* libavcodec
* libswresample (or libavresample, if libswresample isn't present)

If you are running Ubuntu (e.g. 14.04), you should `apt-get install libavutil-dev libavformat-dev libavcodec-dev libavresample-dev`.

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
* Examples for python bindings are in python/test

## Unittests
This library comes with some unittests. To build them, just run
```
$ make test
```
in the `build/` folder. Unittests source files can be found in the `tests/` folder.


## How does the analysis process work?

For every song analyzed, libbliss returns a struct song which contains, among other things,
four floats, each rating an aspect of the song:

* The [tempo](https://en.wikipedia.org/wiki/Tempo "link to wikipedia") rating draws the envelope of the whole song, and then computes its DFT, obtaining peaks at the frequency of each dominant beat.
The period of each dominant beat can then be deduced from the frequencies, hinting at the song's tempo.<br />
Warning: the tempo is not equal to the force of the song. As an example , a heavy metal track can have no steady beat at all, giving a very low tempo score while being very loud.

* The amplitude rating reprents the physical « force » of the song, that is, how much the speaker's membrane will move in order to create the sound.<br />
It is obtained by applying a magic formula with magic coefficients to a histogram of the values of all the song's samples

* The frequency rating is a ratio between high and low frequencies: a song with a lot of high-pitched sounds tends to wake humans up far more easily.<br />
This rating is obtained by performing a DFT over the sample array, and splitting the resulting array in 4 frequency bands: low, mid-low, mid, mid-high, and high.
Using the value in dB for each band, the final formula corresponds to freq\_result = high + mid-high + mid - (low + mid-low)

* The [attack](https://en.wikipedia.org/wiki/Synthesizer#ADSR_envelope "link to wikipedia") rating computes the difference between each value in the envelope and the next (its derivative).<br />
The final value is obtained by dividing the sum of the positive derivates by the number of samples, in order to avoid different results just because of the songs' length.<br />
As you have already guessed, a song with a lot of attacks also tends to wake humans up very quickly.


## Python bindings

Please refer to the `README.md` file in `python/` folder.
