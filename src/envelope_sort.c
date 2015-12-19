// System headers
#include <libavcodec/avfft.h>
#include <math.h>

// Library header
#include "bliss.h"

// Number of bits in the FFT, log2 of the input
#define WINDOW_BITS 10
// Associated size of the input data for the FFT
const int WINDOW_SIZE = (1 << WINDOW_BITS);

void bl_envelope_sort(struct bl_song const * const song,
        struct envelope_result_s * result) {
    // Real FFT context
	RDFTContext* fft;
    // Hold FFT spectrum
	FFTSample* spectrum;
	// Complex DFT of input
	FFTSample* x;
	// Arbitrary sample drop interval
	int precision = 350;
	int freq_size = WINDOW_SIZE / 2;
    // Make the envelope converge to zero in 0.45s
	float decr_speed = 1 / ((float)song->sample_rate * 0.45);
    // Distance between frequencies in the DFT space
	float frequency_step = (float)song->sample_rate / ((float)precision * freq_size);
    // Derivative of the envelope
	double d_envelope = 0;
    // Max value for a sample
	int64_t sample_max = ((1 << (8 * song->nb_bytes_per_sample - 1)));
    // Auxiliary variables to compute the envelope
	double envelope = 0;
    double envelope_prev = 0;
    // Attack rating
	double attack = 0;
    // Peaks in the DFT
	int indices_max[3] = {0, 0, 0};
    // Peaks in the DFT as frequencies
    float frequencies_max[3] = {0., 0., 0.};

    // Set up a real to complex FFT
	fft = av_rdft_init(WINDOW_BITS, DFT_R2C);

    // Allocate spectrum array
	spectrum = av_malloc(freq_size * sizeof(FFTSample));
	for(int i = 0; i < freq_size; ++i) {
		spectrum[i] = 0.0f;
    }

    // Allocate x array
	x = av_malloc(WINDOW_SIZE * sizeof(FFTSample));
	for(int i = 0; i < WINDOW_SIZE; ++i) {
		x[i] = 0.0f;
    }

    // On-the-fly envelope computation and derivation
	for(int i = 0; i < song->nSamples; ++i) {
		envelope = fmax(
			envelope_prev - (decr_speed * envelope_prev),
			(float)(abs(((int16_t*)song->sample_array)[i])));
	
		if((i >= precision) && (i % precision == 0)) {
			if((i / precision) % WINDOW_SIZE != 0) {
				x[(i / precision) % WINDOW_SIZE - 1] = envelope;
			} else {
				x[WINDOW_SIZE - 1] = envelope;
				av_rdft_calc(fft, x);
				for(int d = 1; d < freq_size - 1; ++d) {
					float re = x[d*2];
					float im = x[d*2+1];
					float raw = re*re + im*im;
					spectrum[d] += raw;
				}
				spectrum[0] = 0;
			}
		} else if(i % precision == 0) {
			if((i / precision) % WINDOW_SIZE != 0) {
				x[(i / precision) % WINDOW_SIZE - 1] = envelope;
            }
        }

		d_envelope = (double)(envelope - envelope_prev)/(fabs((double)sample_max));
		d_envelope = d_envelope > 0 ? d_envelope : 0;
		attack += d_envelope * d_envelope;
		envelope_prev = envelope;
	}

    // Find three major peaks in the DFT
    // (up to freq_size / 2 as spectrum is symmetric)
	for(int i = 1; i < freq_size / 2; ++i) {
		if(spectrum[indices_max[0]] < spectrum[i]) {
			indices_max[2] = indices_max[1];
			indices_max[1] = indices_max[0];
			indices_max[0] = i;
		} else if(spectrum[indices_max[1]] < spectrum[i]) {
				indices_max[2] = indices_max[1];
				indices_max[1] = i;
		} else if(spectrum[indices_max[2]] < spectrum[i]) {
			indices_max[2] = i;
        }
	}
	
    // Compute corresponding frequencies
    for(int i = 0; i < 3; ++i) {
        frequencies_max[i] = 1 / ((indices_max[i] + 1) * frequency_step);
    }

	/* TODO Highly experimental (read: almost nonfuctionnal) attack part; to be completed as soon as possible */

    // Compute final tempo and attack ratings
    result->tempo = ( -6 * fmin(
                fmin(frequencies_max[0], frequencies_max[1]),
                fmax(frequencies_max[1], frequencies_max[2]))
            ) + 6;  // TODO ???
	result->attack = attack / song->nSamples * pow(10, 7) - 6;  // TODO ???

    // Free everything
	av_rdft_end(fft);
	av_free(spectrum);
	av_free(x);

	return;
}
