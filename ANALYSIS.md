# How does the analysis process work?

For every song analyzed, libbliss returns a struct song which contains, among other things,
four floats, each rating an aspect of the song:

* The [tempo](https://en.wikipedia.org/wiki/Tempo "link to wikipedia") rating follows [this paper](http://www.cs.tut.fi/sgn/arg/klap/sapmeter.pdf "link to the paper") until part II. A), in order to obtain a downsampled envelope of the whole song. The song's [BPM](https://en.wikipedia.org/wiki/Tempo#Beats_per_minute "link to wikipedia BPM's article") are then estimated by counting the number of peaks and dividing by the length of the song.<br />
The period of each dominant beat can then be deduced from the frequencies, hinting at the song's tempo.
Warning: the tempo is not equal to the force of the song. As an example , a heavy metal track can have no steady beat at all, giving a very low tempo score while being very loud.

* The amplitude rating reprents the physical « force » of the song, that is, how much the speaker's membrane will move in order to create the sound.<br />
It is obtained by finding the right curvature pattern of distribution of raw amplitudes.

* The frequency rating is a ratio between high and low frequencies: a song with a lot of high-pitched sounds tends to wake humans up far more easily.<br />
This rating is obtained by performing a DFT over the sample array, and splitting the resulting array in 4 frequency bands: low, mid-low, mid, mid-high, and high.
Using the value in dB for each band, the final formula corresponds to freq\_result = high + mid-high + mid - (low + mid-low)

* The attack rating is just a sum of the intensity of all the attacks divided by the song's length.<br />
As you have already guessed, a song with a lot of attacks also tends to wake humans up very quickly. <br /> <br />

These ratings are supposed to be as disjoint as possible, to avoid any redundant feature.
However, there still seem to be some correlation between the amplitude / attack rating, as can be seen in this 2D-plot for ~4000 songs: <br />
![Scatter plot of every feature against each other](https://lelele.io/correlation_graph.png)
