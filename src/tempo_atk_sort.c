// System headers
//#include <libavcodec/avfft.h>
#include <fftw3.h>
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
	// First fft window size (1014 = 23ms * 44.1kHz)
	int fft_winsize = 1014;
	// Temporary filtered band
	double temp_band[fft_winsize];
	// FIR Registry
	double registry[256];
	double y;
	// Real FFT plan 
	fftw_plan p;
	int nb_frames = ( song->nSamples - (song->nSamples % fft_winsize) ) * 2 / fft_winsize;
	double *filtered_array[36];
	for(int i = 0; i < 36; ++i)
		filtered_array[i] = calloc(nb_frames, sizeof(double));
	// Hold FFT spectrum
	double fft_array[fft_winsize/2 + 1];
	// Hold fft input
	double *in;
	in = fftw_malloc(fft_winsize * sizeof(double));
	for(int i = 0; i < fft_winsize; ++i) {
		in[i] = 0.0f;
	}
	// Hold fft output
	fftw_complex *out;
	out = (fftw_complex*) fftw_malloc(sizeof(fftw_complex) * fft_winsize);
	// Hold final FFT spectrum;
	double *final_fft;
	// Set up a real to complex FFT 
	p = fftw_plan_dft_r2c_1d(fft_winsize, in, out, FFTW_ESTIMATE);
	double *normalized_song;
	normalized_song = (double*)malloc(song->nSamples * sizeof(double));
	// Allocate spectrum array
	//fft_array = calloc(fft_winsize, sizeof(double));
	// Allocate x array


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
		for(int b = 0; b < (song->nSamples - song->nSamples % fft_winsize) - fft_winsize; b += (int)fft_winsize / 2) {
			for(int j = 0; j < 33; ++j)
				registry[j] = 0.0;
			// Applying filter
			for(int j = b; j < b + fft_winsize; ++j) {
				for(int k = 33; k > 1; --k)
					registry[k-1] = registry[k-2];

				registry[0] = normalized_song[j];
				
				y = 0;
				for(int k = 0; k < 33; ++k)
					y += coeffs[i][k] * registry[k];
				in[j - b] = y;
			}
			// End of filter
			fftw_execute(p);
			for(int k = 0; k < fft_winsize/2 + 1; ++k) {
				fft_array[k] = 0.0;
			}
			for(int k = 0; k < fft_winsize/2 + 1; ++k) {
				double re = out[k][0];
				double im = out[k][1];
				double abs = sqrt(re*re + im*im);
				fft_array[k] = abs;
			}
			float sum_fft = 0;
			for(int k = 0; k < fft_winsize/2 + 1; ++k)
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
	double *band_sum;
	double registry2[7];
	for(int i = 0; i < 36; ++i) {
		upsampled_array[i] = calloc(2*nb_frames, sizeof(double));
		lowpassed_array[i] = calloc(2*nb_frames, sizeof(double));
		dlowpassed_array[i] = calloc(2*nb_frames, sizeof(double));
		weighted_average[i] = calloc(2*nb_frames, sizeof(double));
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

	for(int i = 0; i < 36; ++i) {
		free(upsampled_array[i]);
		free(filtered_array[i]);
		free(lowpassed_array[i]);
		free(dlowpassed_array[i]);
	}

	band_sum = calloc(2*nb_frames, sizeof(double));
	fftw_free(out);
	out = (fftw_complex*) fftw_malloc(sizeof(fftw_complex) * 2*nb_frames);

	for(int j = 0; j < 2*nb_frames - 1; ++j) {
		for(int i = 0; i < 36; ++i) {
			band_sum[j] += weighted_average[i][j];
		}
	}


	double fs2 = 2*fs / fft_winsize;
	double df2 = fs2 / (double)(2 * nb_frames);
	// Between 50ms and 2s (before and after, the human ear don't perceive recurring sounds)
	// (aka between 0.5 Hz and 20 Hz)
	int interval_min = (int)floor(1 / df2);
	int interval_max = (int)floor(20 / df2);
	int peak_loc3 = 0;
	double peak_val3 = 0;
	int peak_loc2 = 0;
	double peak_val2 = 0;
	int peak_loc = 0;
	double peak_val = 0;

	printf("Interval min: %d\nInterval max: %d\n", interval_min, interval_max);
	printf("df: %f\n", df2);

	fftw_destroy_plan(p);
	p = fftw_plan_dft_r2c_1d(2*nb_frames, band_sum, out, FFTW_ESTIMATE);
	final_fft = calloc(2*nb_frames, sizeof(double));

	fftw_execute(p);
	for(int k = 0; k < (2 * nb_frames) / 2 + 1; ++k) {
		float re = out[k][0];
		float im = out[k][1];
		float abs = sqrt(re*re + im*im);
		
		final_fft[k] += abs;
	}

	for(int k = interval_min; k < interval_max; ++k) {
		if(final_fft[k] > peak_val3 && (final_fft[k] >= final_fft[k-1]) &&
			final_fft[k] >= final_fft[k+1]) {
			if(final_fft[k] > peak_val) {
				peak_val = final_fft[k];
				peak_loc = k;
			}
			else if(final_fft[k] > peak_val2) {
				peak_val2 = final_fft[k];
				peak_loc2 = k;
			}
			else {
				peak_val3 = final_fft[k];
				peak_loc3 = k;
			}
		}
	}

	fftw_free(in);
	fftw_free(out);

	printf("Peak loc: %d\nFrequency: %f\nPeriod: %f\n", peak_loc, peak_loc*df2, 1 / (peak_loc*df2));
	printf("Peak loc2: %d\nFrequency: %f\nPeriod: %f\n", peak_loc2, peak_loc2*df2, 1 / (peak_loc2*df2));
	printf("Peak loc3: %d\nFrequency: %f\nPeriod: %f\n", peak_loc3, peak_loc3*df2, 1 / (peak_loc3*df2));

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
