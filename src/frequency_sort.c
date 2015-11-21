#include <math.h>
#include <libavcodec/avfft.h>
#include "bliss.h"

// Number of bits in the FFT, log2 of the length
#define WIN_BITS 9
// Length of the samples used in FFT
static const int WINDOW_SIZE = (1 << WIN_BITS);

// TODO: Why not used?
#define GRAVE_INF 2
#define GRAVE_SUP 4
#define AIGU_INF 17
#define AIGU_SUP 104

float bl_freq_sort(struct bl_song const * const song, int debug) {
    // FFT transform context
	RDFTContext* fft;
    // Hann window values
	float hann_window[WINDOW_SIZE];
    // Number of frames, that is number of juxtaposed windows in the music
    int n_frames;
    // Complex DFT of input
	FFTSample* x;
    // Hold FFT power spectrum
	FFTSample *power_spectrum;
    // Power maximum value
	float peak = 0;

    // TODO
	float tab_bandes[5];
	float tab_sum;

    // Initialize Hann window
	for(int i = 0; i < WINDOW_SIZE; ++i) {
		hann_window[i] = .5f * (1.0f - cos(2 * M_PI * i / (WINDOW_SIZE - 1)));
    }

    // TODO
	for(int i = 0; i < 5; ++i) {
		tab_bandes[i] = 0.0f;
    }

    // Get the number of frames in one channel
	n_frames = floor((song->nSamples / song->channels) / WINDOW_SIZE);

    // Allocate memory for x vector
	x = (FFTSample*)av_malloc(WINDOW_SIZE * sizeof(FFTSample));

    // Zero-initialize power spectrum
	power_spectrum = (FFTSample*) av_malloc((WINDOW_SIZE * sizeof(FFTSample)));
	for(int i = 0; i <= WINDOW_SIZE / 2; ++i) {  // Why / 2 ?
		power_spectrum[i] = 0.0f;
    }

    // Initialize fft
	fft = av_rdft_init(WIN_BITS, DFT_R2C);

	for(int i = 0; i < n_frames * WINDOW_SIZE * song->channels; i += song->channels * WINDOW_SIZE) {
		if(2 == song->nb_bytes_per_sample) {  // 16 bits sound
			if(2 == song->channels) {  // Stereo sound
				for(int d = 0; d < WINDOW_SIZE; ++d) {
					x[d] = (float)((((int16_t*)song->sample_array)[i+2*d] + ((int16_t*)song->sample_array)[i+2*d+1])/2) * hann_window[d];
                }
            } else {  // Mono sound
				for(int d = 0; d < WINDOW_SIZE; ++d) {
					x[d] = (float)(((int16_t*)song->sample_array)[i+d])*hann_window[d];
                }
            }
		} else if (4 == song->nb_bytes_per_sample) {  // 32 bits sound
			if(2 == song->channels) {  // Stereo sound
				for(int d = 0; d < WINDOW_SIZE; ++d) {
					x[d] = (float)((((int32_t*)song->sample_array)[i+2*d] + ((int32_t*)song->sample_array)[i+2*d+1])/2)*hann_window[d];
                }
            } else {  // Mono sound
                for(int d = 0; d < WINDOW_SIZE; ++d) {
                    x[d] = (float)(((int16_t*)song->sample_array)[i+d])*hann_window[d];
                }
            }
		}

        // Compute FFT
		av_rdft_calc(fft, x);

        // Fill-in power spectrum
        power_spectrum[0] = x[0] * x[0];  // TODO: Why not x[1]?
		for(int d = 1; d < WINDOW_SIZE / 2; ++d) {
			float re = x[d * 2];
			float im = x[d * 2 + 1];
			float raw = (re * re) + (im * im);
			power_spectrum[d] = raw;
		}
	}

    // Normalize it and compute real power in dB
	for(int d = 1; d <= WINDOW_SIZE / 2; ++d) {
		power_spectrum[d] = sqrt(power_spectrum[d] / WINDOW_SIZE);  // TODO: Why?

        // Get power spectrum peak
		peak = fmax(power_spectrum[d], peak);

        // Compute power spectrum in dB
        // TODO: Should not it be in a separate loop as peak may vary in the loop?
		power_spectrum[d] = 20 * log10(power_spectrum[d] / peak) - 3;  // TODO: Why -3?
	}

    // Sum power in frequency bands
    // TODO: What are magic numbers?
	tab_bandes[0] = (power_spectrum[1] + power_spectrum[2]) / 2;
	tab_bandes[1] = (power_spectrum[3] + power_spectrum[4]) / 2;
	for(int i = 5; i <= 30; ++i) {
		tab_bandes[2] += power_spectrum[i];
    }
	tab_bandes[2] /= (29 - 4);
	for(int i = 31; i <= 59; ++i) {
		tab_bandes[3] += power_spectrum[i];
    }
	tab_bandes[3] /= (58 - 30);
	for(int i = 60; i <= 117; ++i) {
		tab_bandes[4] += power_spectrum[i];
    }
	tab_bandes[4] /= (116 - 59);
	tab_sum = tab_bandes[4] + tab_bandes[3] + tab_bandes[2] - tab_bandes[0] - tab_bandes[1];

    // Clean everything
	av_free(x);
	av_free(power_spectrum);
	av_rdft_end(fft);

    // TODO: Why?
	return ((1. / 3.) * tab_sum + (68. / 3.));
}
