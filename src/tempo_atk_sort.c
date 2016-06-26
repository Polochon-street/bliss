// System headers
#include <libavcodec/avfft.h>
#include <math.h>

// Library header
#include "bliss.h"

#include "bandpass_coeffs.h"

// Number of bits in the FFT, log2 of the input
#define WINDOW_BITS 10
// Associated size of the input data for the FFT
const int WINDOW_SIZE = (1 << WINDOW_BITS);

#define MAX(x, y) (((x) > (y)) ? (x) : (y))


void bl_envelope_sort(struct bl_song const * const song,
		struct envelope_result_s * result) {
	// TODO Make sure the sampling freq is 44.1 kHz
	float fs = 44100;
	// Nyquist frequency 
	float fnyq = fs / 2;
	// Signal mean
	float signal_mean = 0;
	// Signal variance
	float signal_variance = 0;
	// First fft window size
	int fft_winsize = 1024;
	// Temporary filtered band
	double temp_band[fft_winsize];
	// FIR Registry
	double registry[256];
	double y;
	for(int j = 0; j < 33; ++j)
		registry[j] = 0.0;
	// Real FFT context
	RDFTContext* fft;
	int nb_frames = ( song->nSamples - (song->nSamples % fft_winsize) ) * 2 / fft_winsize;
	double *filtered_array[36];
	for(int i = 0; i < 36; ++i)
		filtered_array[i] = calloc(nb_frames, sizeof(double));
	// Hold FFT spectrum
	FFTSample *fft_array;
	// Complex DFT of input
	FFTSample* x;
	// Hold final FFT spectrum;
	FFTSample *final_fft;
	// Set up a real to complex FFT TODO
	fft = av_rdft_init(10, DFT_R2C); // log2(1024) = 10
	double *normalized_song;
	normalized_song = (double*)malloc(song->nSamples * sizeof(double));
	// Allocate spectrum array
	fft_array = av_malloc(fft_winsize * sizeof(FFTSample));
	for(int i = 0; i < fft_winsize; ++i) {
		fft_array[i] = 0.0f;
	}

	// Allocate x array
	x = av_malloc(fft_winsize * sizeof(FFTSample));
	for(int i = 0; i < fft_winsize; ++i) {
		x[i] = 0.0f;
	}

	for(int i = 0; i < song->nSamples; ++i)
		normalized_song[i] = (double)((int16_t*)song->sample_array)[i] / 32767; // TODO replace with adequate max

	// Pre-treatment: Compute mean & variance to normalize the signal to have zero mean and unity variance
	signal_mean = bl_mean(normalized_song, song->nSamples);
	signal_variance = bl_variance(normalized_song, song->nSamples);

	for(int i = 0; i < song->nSamples; ++i) {
		normalized_song[i] = ( normalized_song[i] - signal_mean ) / signal_variance;
	}

	// Bandpass filter bank
	for(int i = 0; i < 36; ++i) {
		int d = 0;
		for(int b = 0; b < (song->nSamples - song->nSamples % fft_winsize) - fft_winsize; b += (int)fft_winsize/2) {
			// Applying filter
			for(int j = b; j < b + fft_winsize; ++j) {
				for(int k = 33; k > 1; --k)
					registry[k-1] = registry[k-2];

				registry[0] = normalized_song[j];
				
				y = 0;
				for(int k = 0; k < 33; ++k)
					y += coeffs[i][k] * registry[k];
				x[j - b] = y;
			}
			// End of filter
			av_rdft_calc(fft, x);
			for(int k = 0; k < fft_winsize; ++k) {
				fft_array[k] = 0.0;
			}
			for(int k = 1; k < fft_winsize / 2; ++k) {
				float re = x[k*2];
				float im = x[k*2+1];
				float abs = sqrt(re*re + im*im);
				fft_array[k] += abs;
			}
			fft_array[0] = sqrt(x[0] * x[0]);
			float sum_fft = 0;
			for(int k = 0; k < fft_winsize/2; ++k)
				sum_fft += fft_array[k] * fft_array[k];
			filtered_array[i][(int)floor((double)d / (double)fft_winsize)] += sum_fft;
			d += fft_winsize;
		}
	}

	// Upsample filtered_array by 2
	double *upsampled_array[36];
	double *lowpassed_array[36];
	double *dlowpassed_array[36];
	double *weighted_average[36];
	FFTSample *band_sum;
	double registry2[7];
	for(int i = 0; i < 36; ++i) {
		upsampled_array[i] = calloc(2*nb_frames, sizeof(double));
		lowpassed_array[i] = calloc(2*nb_frames, sizeof(double));
		dlowpassed_array[i] = calloc(2*nb_frames, sizeof(double));
		weighted_average[i] = calloc(2*nb_frames, sizeof(double));
		band_sum = calloc(2*nb_frames, sizeof(FFTSample));
	}

	float mu = 100.0;
	float lambda = 0.8;
	double final = 0;
	double c, d;

	y = 0;

	for(int i = 0; i < 36; ++i) { // 2, or more like 36
		for(int j = 0; j < nb_frames - 1; j++) {
			upsampled_array[i][2*j] = log(1 + mu*filtered_array[i][j]) / log(1 +mu);
			upsampled_array[i][2*j + 1] = 0;
		//	printf("%f %f %f\n", filtered_array[i][j], upsampled_array[i][2*j], upsampled_array[i][2*j+1]);
		}

		for(int r = 0; r < 7; ++r) {
			registry[r] = 0.0;
			registry2[r] = 0.0;
		}

		y = 0;

		// LOWPASS_FILTER
		for(int j = 0; j < nb_frames*2 - 1; ++j) {
			for(int k = 7; k > 1; --k) {
				registry[k-1] = registry[k-2];
				registry2[k-1] = registry2[k-2];
			}
			registry[0] = upsampled_array[i][j];
			registry2[0] = y;
			
			y = 0;
			d = 0;
			c = 0;
			for(int k = 0; k < 7; ++k)
				d += butterb[k] * registry[k];
			for(int k = 1; k < 7; ++k)
				c += buttera[k] * registry2[k-1];
			y = (d - c) / buttera[0];
			lowpassed_array[i][j] = y;
		}

		dlowpassed_array[i][0] = lowpassed_array[i][0];
		for(int j = 1; j < nb_frames*2 - 1; ++j) {
			dlowpassed_array[i][j] = lowpassed_array[i][j] - lowpassed_array[i][j-1];
			dlowpassed_array[i][j] = MAX(dlowpassed_array[i][j], 0);
		}
		for(int j = 0; j < nb_frames*2 - 1; ++j) {
			weighted_average[i][j] = (1 - lambda) * lowpassed_array[i][j] + lambda * 172 * dlowpassed_array[i][j] / 10;
		}
	}

	for(int j = 0; j < 2*nb_frames - 1; ++j) {
		for(int i = 0; i < 36; ++i) {
			band_sum[j] += weighted_average[i][j];
		}
	}

	double log_truncated_nb_frames = floor(log2(2*nb_frames));
	int truncated_nb_frames = (int)floor(exp(log(2)*log_truncated_nb_frames));

	fft = av_rdft_init(log_truncated_nb_frames, DFT_R2C);
	final_fft = calloc(truncated_nb_frames, sizeof(FFTSample));

	av_rdft_calc(fft, band_sum);
	for(int k = 0; k < truncated_nb_frames / 2; ++k) {
		float re = band_sum[k*2];
		float im = band_sum[k*2+1];
		float abs = sqrt(re*re + im*im);
		
		final_fft[k] += abs;
	}
	fft_array[0] = sqrt(band_sum[0] * band_sum[0]);

	for(int k = 0; k < truncated_nb_frames / 2; ++k) 
		printf("%f\n", final_fft[k]);

/*	for(int i = 0; i < 36; ++i) 
		for(int j = 0; j < nb_frames*2 - 1; ++j)
			final += weighted_average[i][j];*/

	// "Normalize" the result
	/* result->attack = -1142 * final / song->nSamples + 56;

	printf("Final atk result: %f\n", result->attack);*/
	// On-the-fly envelope computation and derivation






/*	for(int i = 0; i < song->nSamples; ++i) {
		envelope = fmax(
			envelope_prev - (decr_speed * envelope_prev),
			(float)(abs(((int16_t*)song->sample_array)[i])));
	
		if((i >= precision) && (i % precision == 0)) {
			if((i / precision) % WINDOW_SIZE != 0) {
				x[(i / precision) % WINDOW_SIZE - 1] = envelope;
			}
			else {
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
		}
		else if(i % precision == 0) {
			if((i / precision) % WINDOW_SIZE != 0) {
				x[(i / precision) % WINDOW_SIZE - 1] = envelope;
			}
		}

		d_envelope = (double)(envelope - envelope_prev)/(fabs((double)sample_max));
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
		}
		else if(spectrum[indices_max[1]] < spectrum[i]) {
				indices_max[2] = indices_max[1];
				indices_max[1] = i;
		}
		else if(spectrum[indices_max[2]] < spectrum[i]) {
			indices_max[2] = i;
        }
	}
	
	// Compute corresponding frequencies
	for(int i = 0; i < 3; ++i) {
		frequencies_max[i] = 1 / ((indices_max[i] + 1) * frequency_step);
	}

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

	return; */
	return;
}
