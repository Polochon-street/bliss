#include <math.h>
#include <libavcodec/avfft.h>
#include "bliss.h"

// Number of bits in the FFT, log2 of the length
#define WIN_BITS 9
// Length of the samples used in FFT
static const int WINDOW_SIZE = (1 << WIN_BITS);

// Arbitrary frequency band limits
#define LOW_INF 5 
#define LOW_SUP 30
#define HIGH_INF 59
#define HIGH_SUP 117

float bl_frequency_sort(struct bl_song const * const song) {
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

	// Array containing frequency mean of different bands 
	float bands[5];
	// Weighted sum of frequency bands
	float bands_sum;

	// Initialize Hann window
	for(int i = 0; i < WINDOW_SIZE; ++i) {
		hann_window[i] = .5f * (1.0f - cos(2 * M_PI * i / (WINDOW_SIZE - 1)));
    }

	// Initialize band array
	for(int i = 0; i < 5; ++i) {
		bands[i] = 0.0f;
	}

	// Get the number of frames in one channel
	n_frames = floor((song->nSamples / song->channels) / WINDOW_SIZE);

	// Allocate memory for x vector
	x = (FFTSample*)av_malloc(WINDOW_SIZE * sizeof(FFTSample));

	// Zero-initialize power spectrum
	power_spectrum = (FFTSample*) av_malloc((WINDOW_SIZE * sizeof(FFTSample)) / 2 + 1*sizeof(FFTSample));
	for(int i = 0; i <= WINDOW_SIZE / 2; ++i) {  // 2 factor due to x's complex nature and power_spectrum's real nature.
		power_spectrum[i] = 0.0f;
	}

	// Initialize fft
	fft = av_rdft_init(WIN_BITS, DFT_R2C);

	for(int i = 0; i < n_frames * WINDOW_SIZE * song->channels; i += song->channels * WINDOW_SIZE) {
		if(2 == song->channels) {  // Stereo sound
			for(int d = 0; d < WINDOW_SIZE; ++d) {
				x[d] = (float)((((int16_t*)song->sample_array)[i+2*d] + ((int16_t*)song->sample_array)[i+2*d+1])/2) * hann_window[d];
			}
		}
		else {  // Mono sound
			for(int d = 0; d < WINDOW_SIZE; ++d) {
				x[d] = (float)(((int16_t*)song->sample_array)[i+d])*hann_window[d];
			}
		}

		// Compute FFT
		av_rdft_calc(fft, x);

		// Fill-in power spectrum
		power_spectrum[0] = x[0] * x[0];  // Ignore x[1] due to ffmpeg's fft specifity
		for(int d = 1; d < WINDOW_SIZE / 2; ++d) {
			float re = x[d * 2];
			float im = x[d * 2 + 1];
			float raw = (re * re) + (im * im);
			power_spectrum[d] += raw;
		}
	}

	// Normalize it and compute real power in dB
	for(int d = 1; d <= WINDOW_SIZE / 2; ++d) {
		power_spectrum[d] = sqrt(power_spectrum[d] / WINDOW_SIZE);
	
		// Get power spectrum peak
		peak = fmax(power_spectrum[d], peak);
	}

	// Compute power spectrum in dB with 3dB attenuation
	for(int d = 1; d <= WINDOW_SIZE / 2; ++d) {
		power_spectrum[d] = 20 * log10(power_spectrum[d] / peak) - 3;
	}
	// Sum power in frequency bands
	// Arbitrary separation in frequency bands
	bands[0] = (power_spectrum[1] + power_spectrum[2]) / 2;

	bands[1] = (power_spectrum[3] + power_spectrum[4]) / 2;

	for(int i = LOW_INF; i <= LOW_SUP; ++i) {
		bands[2] += power_spectrum[i];
	}
	bands[2] /= (LOW_SUP - LOW_INF);

	for(int i = LOW_SUP + 1; i <= HIGH_INF; ++i) {
		bands[3] += power_spectrum[i];
	}
	bands[3] /= (HIGH_INF - (LOW_SUP + 1));

	for(int i = HIGH_INF + 1; i <= HIGH_SUP; ++i) {
		bands[4] += power_spectrum[i];
	}
	bands[4] /= (HIGH_SUP - (HIGH_INF + 1));

	bands_sum = bands[4] + bands[3] + bands[2] - bands[0] - bands[1];

	// Clean everything
	av_free(x);
	av_free(power_spectrum);
	av_rdft_end(fft);

	// Return final score, weighted by coefficients in order to have -4 for a panel of calm songs,
	// and 4 for a panel of loud songs. (only useful if you want an absolute « Loud » and « Calm » result
	return ((1. / 3.) * bands_sum + 68. / 3.);
}
