// System headers
#include <fftw3.h>
#include <math.h>

// Library header
#include "bliss.h"
#include "bandpass_coeffs.h"

#define MAX(x, y) (((x) > (y)) ? (x) : (y))
#define MAX_INT16 (1 << 15)

/* Beat detection achieved thanks to <TODO link> */

void bl_envelope_sort(struct bl_song const * const song,
		struct envelope_result_s * result) {
	// TODO Make sure the sampling freq is 44.1 kHz
	float fs = 44100;
	// Signal mean
	float signal_mean = 0;
	// Signal variance
	float signal_variance = 0;
	// First fft window size (1014 = 23ms * 44.1kHz)
	int fft_winsize = 1014;
	// FIR registry
	double registry[256];
	// FIR temporary output
	double y;
	// RDFT plan 
	fftw_plan p;
	// Estimate the number of frames of size fft_winsize
	int nb_frames = ( song->nSamples - (song->nSamples % fft_winsize) ) * 2 / fft_winsize;
	// Hold signal filtered by 36 different bandpass filters
	double *filtered_array[36];
	// Hold first RDFT spectrum
	double fft_array_bp[fft_winsize/2 + 1];
	// Hold first RDFT input
	double *in;
	// Hold RDFT output
	fftw_complex *out;
	// Hold final RDFT spectrum;
	double *fft_array_tempo;
	// Hold normalized signal
	double *normalized_song;

	normalized_song = (double*)malloc(song->nSamples * sizeof(double));

	for(int i = 0; i < 36; ++i)
		filtered_array[i] = calloc(nb_frames, sizeof(double));

	in = fftw_malloc(fft_winsize * sizeof(double));
	out = (fftw_complex*) fftw_malloc(sizeof(fftw_complex) * fft_winsize);

	for(int i = 0; i < fft_winsize; ++i) {
		in[i] = 0.0f;
	}

	// Set up the RDFT
	p = fftw_plan_dft_r2c_1d(fft_winsize, in, out, FFTW_ESTIMATE);

	/* End initialization */
	
	/* Part 1: Bandpass filtering over 36 frequency bands */

	for(int i = 0; i < song->nSamples; ++i)
		normalized_song[i] = (double)((int16_t*)song->sample_array)[i] / MAX_INT16; 

	// Achieve zero mean and unity variance
	signal_mean = bl_mean(normalized_song, song->nSamples);
	signal_variance = bl_variance(normalized_song, song->nSamples);
	for(int i = 0; i < song->nSamples; ++i) {
		normalized_song[i] = (normalized_song[i] - signal_mean) / signal_variance;
	}

	// Apply and store 36 bandpassed and RDFT'd signals
	for(int i = 0; i < 36; ++i) {
		int d = 0;
		for(int b = 0; b < (song->nSamples - song->nSamples % fft_winsize) - fft_winsize; b += (int)fft_winsize / 2) {
			for(int j = 0; j < 33; ++j)
				registry[j] = 0.0;
			// Apply filter
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
				double re = out[k][0];
				double im = out[k][1];
				double abs = sqrt(re*re + im*im);
				fft_array_bp[k] = abs;
			}
			float sum_fft = 0;
			for(int k = 0; k < fft_winsize/2 + 1; ++k)
				sum_fft += fft_array_bp[k] * fft_array_bp[k];
			filtered_array[i][(int)floor((double)d / (double)fft_winsize)] += sum_fft;
			d += fft_winsize;
		}
	}

	/* Part two: process the filtered signal a bit more */

	// Create two ill-named temporary arrays to avoid allocating five well-named ones
	double *temp_filtered_array1[36];
	double *temp_filtered_array2[36];
	double *weighted_average[36];
	// Hold the sum of the band's intensity
	double *band_sum;
	// Hold the low pass registry
	double registry2[7];
	// Coefficients values extracted from the paper (see above)
	float mu = 100.0;
	float lambda = 0.8;
	double atk_sum = 0;
	double c, d;

	for(int i = 0; i < 36; ++i) {
		temp_filtered_array1[i] = calloc(2*nb_frames, sizeof(double));
		temp_filtered_array2[i] = calloc(2*nb_frames, sizeof(double));
		weighted_average[i] = calloc(2*nb_frames, sizeof(double));
	}
	band_sum = calloc(2*nb_frames, sizeof(double));

	for(int i = 0; i < 36; ++i) { 
		// Upsample array by 2
		for(int j = 0; j < nb_frames; j++) {
			temp_filtered_array1[i][2*j] = log(1 + mu*filtered_array[i][j]) / log(1 +mu);
			temp_filtered_array1[i][2*j + 1] = 0;
		}
		
		// Reset registry values
		for(int r = 0; r < 7; ++r) {
			registry[r] = 0.0;
			registry2[r] = 0.0;
		}

		y = 0;

		// Apply low pass filter 
		for(int j = 0; j < nb_frames*2; ++j) {
			for(int k = 7; k > 1; --k) {
				registry[k-1] = registry[k-2];
				registry2[k-1] = registry2[k-2];
			}
			registry[0] = temp_filtered_array1[i][j];
			registry2[0] = y;
			
			y = 0;
			d = 0;
			c = 0;
			for(int k = 0; k < 7; ++k)
				d += butterb[k] * registry[k];
			for(int k = 1; k < 7; ++k)
				c += buttera[k] * registry2[k-1];
			y = (d - c) / buttera[0];
			temp_filtered_array2[i][j] = y;
		}

		// Differenciate low pass array
		temp_filtered_array1[i][0] = temp_filtered_array2[i][0];
		for(int j = 1; j < nb_frames*2; ++j) {
			temp_filtered_array1[i][j] = temp_filtered_array2[i][j] - temp_filtered_array2[i][j-1];
			temp_filtered_array1[i][j] = MAX(temp_filtered_array1[i][j], 0);
			
		}
		// Compute weighted average of low pass array / differenciated low pass array
		for(int j = 0; j < nb_frames*2; ++j) {
			weighted_average[i][j] = (1 - lambda) * temp_filtered_array2[i][j] + lambda * 172 * temp_filtered_array1[i][j] / 10;
		}
	}

	fftw_free(out);
	out = (fftw_complex*) fftw_malloc(sizeof(fftw_complex) * 2*nb_frames);

	/* Part 3: Perform the tempo estimation (finally !) */

	// New sampling frequency (after the above processing)
	double fs2 = 2*fs / fft_winsize;
	// RDFT frequency interval 
	double df2 = fs2 / (double)(2 * nb_frames);
	// Between 50ms and 2s (before and after, the human ear don't perceive recurring sounds
	// or at least, let's hope so)
	// (aka between 0.5 Hz and 20 Hz)
	int interval_min = (int)floor(0.5 / df2);
	int interval_max = (int)floor(20 / df2);
	// RDFT peak location index and corresponding values
	int peak_loc3 = 0;
	double peak_val3 = 0;
	int peak_loc2 = 0;
	double peak_val2 = 0;
	int peak_loc = 0;
	double peak_val = 0;
	// Hold final bliss scores
	double tempo1_score = 0;
	double tempo2_score = 0;
	double tempo3_score = 0;
	// Amplitude of peak n against amplitude of peak 1
	double peak1_percentage = 1;
	double peak2_percentage = 0;
	double peak3_percentage = 0;

	fft_array_tempo = calloc(2*nb_frames, sizeof(double));

	// Sum all bands' weighted average
	for(int j = 0; j < 2*nb_frames - 1; ++j) {
		for(int i = 0; i < 36; ++i) {
			band_sum[j] += weighted_average[i][j];
		}
	}

	// Update and run RDFT plan
	fftw_destroy_plan(p);
	p = fftw_plan_dft_r2c_1d(2*nb_frames, band_sum, out, FFTW_ESTIMATE);
	fftw_execute(p);

	for(int k = 0; k < (2 * nb_frames) / 2 + 1; ++k) {
		float re = out[k][0];
		float im = out[k][1];
		float abs = sqrt(re*re + im*im);
		
		fft_array_tempo[k] += abs;
	}

	// Find the 3 major peaks between 50ms and 2s
	for(int k = interval_min; k < interval_max; ++k) {
		if(fft_array_tempo[k] > peak_val3 && (fft_array_tempo[k] >= fft_array_tempo[k-1]) &&
			fft_array_tempo[k] >= fft_array_tempo[k+1]) {
			if(fft_array_tempo[k] > peak_val) {
				peak_val = fft_array_tempo[k];
				peak_loc = k;
			}
			else if(fft_array_tempo[k] > peak_val2) {
				if(fabs(k - peak_loc) > 40) {
					peak_val2 = fft_array_tempo[k];
					peak_loc2 = k;
				}
			}
			else {
				if(fabs(k - peak_loc2) > 40) {
					peak_val3 = fft_array_tempo[k];
					peak_loc3 = k;
				}
			}
		}
	}

	peak2_percentage = peak_val2 / peak_val;
	peak3_percentage = peak_val3 / peak_val;

	// Compute final score
	tempo1_score = -4.1026 / (peak_loc * df2) + 4.2052;
	tempo2_score = -4.1026 / (peak_loc2 * df2) + 4.2052;
	tempo3_score = -4.1026 / (peak_loc3 * df2) + 4.2052;

	for(int i = 0; i < 36; ++i) 
		for(int j = 0; j < nb_frames*2 - 1; ++j)
			atk_sum += weighted_average[i][j];

	printf("Peak loc: %d\nFrequency: %f\nPeriod: %f\n", peak_loc, peak_loc*df2, 1 / (peak_loc*df2));
	printf("Peak loc2: %d\nFrequency: %f\nPeriod: %f\n", peak_loc2, peak_loc2*df2, 1 / (peak_loc2*df2));
	printf("Peak loc3: %d\nFrequency: %f\nPeriod: %f\n", peak_loc3, peak_loc3*df2, 1 / (peak_loc3*df2));
	printf("Tempo score 1: %f\n", tempo1_score);
	printf("Tempo score 2: %f\n", tempo2_score);
	printf("Tempo score 3: %f\n", tempo3_score);
	printf("Atk score: %f\n", -1142 * atk_sum / song->nSamples + 56);

	// Free everything
	fftw_free(in);
	fftw_free(out);
	for(int i = 0; i < 36; ++i) {
		free(temp_filtered_array1[i]);
		free(temp_filtered_array2[i]);
	}

	// Compute final tempo and attack ratings
	result->tempo1 = tempo1_score;
	result->tempo2 = tempo2_score;
	result->tempo3 = tempo3_score;
	result->attack = -1142 * atk_sum / song->nSamples + 56;
}
