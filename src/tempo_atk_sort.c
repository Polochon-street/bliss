// System headers
#include <fftw3.h>
#include <math.h>

// Library header
#include "bliss.h"
#include "bandpass_coeffs.h"

#define MAX(x, y) (((x) > (y)) ? (x) : (y))
#define MAX_INT16 (1 << 15)
#define NB_COEFFS 17
// Hold number of the filterbank's bands. Currently set to one for CPU-consumption reasons.
#define NB_BANDS 1

/* Beat detection achieved thanks to Anssi Klapuri http://www.cs.tut.fi/sgn/arg/klap/sapmeter.pdf */

void bl_rectangular_filter(double *sample_array_out, double *sample_array_in, int nSamples, int smooth_width) {
	int half_smooth_w = (int)round(smooth_width/2.);
	double tempsum = 0;

	for(int k = 0; k < smooth_width; ++k)
		tempsum += sample_array_in[k];
	
	for(int k = 0; k < nSamples - smooth_width; ++k) {
		sample_array_out[k + half_smooth_w - 1] = tempsum;
		tempsum -= sample_array_in[k];
		tempsum += sample_array_in[k + smooth_width];
	}

	for(int k = nSamples - smooth_width; k < nSamples; ++k)
		sample_array_out[nSamples - half_smooth_w] += sample_array_in[k];

	for(int k = 0; k < nSamples; ++k) {
		sample_array_out[k] /= smooth_width;
	}
}

void bl_envelope_sort(struct bl_song const * const song,
		struct envelope_result_s * result) {
	int signal_mean = 0;
	int signal_variance = 0;
	double signal_mean_d = 0.0;
	double signal_variance_d = 0.0;
	// First RDFT window size (1014 = 23ms * 44.1kHz)
	//int fft_winsize = 1014;
	int fft_winsize = 508;
	// First RDFT window size (double version, to avoid a costly cast)
	//double double_fft_winsize = 1014.0;
	double double_fft_winsize = 508.0;
	// Half fft_winsize;
	int half_fft_winsize = fft_winsize / 2;
	// FIR registry
	double registry[NB_COEFFS];
	// FIR temporary output
	
	// RDFT plan 
	fftw_plan p;
	// Estimate the number of frames of size fft_winsize
	int nb_frames = ( song->nSamples - (song->nSamples % fft_winsize) ) * 2 / fft_winsize;
	// Hold bandpass iteration number
	int iteration_number = (song->nSamples - song->nSamples % fft_winsize) - fft_winsize;
	// Hold signal filtered by 5 different bandpass filters
	double *filtered_array[1];
	// Hold first RDFT spectrum
	double fft_array_bp[fft_winsize/2 + 1];
	// Hold first RDFT input
	double *in;
	// Hold RDFT output
	fftw_complex *out;
	// Hold normalized signal
	double *normalized_song;
	// Count indices; heavily used so put on a register
	register int k;

	normalized_song = malloc(song->nSamples * sizeof(double));

	for(int i = 0; i < NB_BANDS; ++i)
		filtered_array[i] = calloc(nb_frames, sizeof(double));

	in = fftw_malloc(fft_winsize * sizeof(double));
	out = (fftw_complex*)fftw_malloc(sizeof(fftw_complex) * fft_winsize);

	for(int i = 0; i < fft_winsize; ++i) {
		in[i] = 0.0f;
	}

	// Set up the RDFT
	p = fftw_plan_dft_r2c_1d(fft_winsize, in, out, FFTW_ESTIMATE);

	/* End initialization */
	
	/* Part 1: Bandpass filtering over 5 frequency bands */

	// Achieve zero mean and unity variance
	signal_mean = bl_mean(((int16_t*)song->sample_array), song->nSamples);
	signal_variance = bl_variance(((int16_t*)song->sample_array), song->nSamples, signal_mean);

	signal_mean_d = (double)signal_mean / MAX_INT16;
	signal_variance_d = (double)signal_variance / MAX_INT16;
	signal_variance_d /= MAX_INT16;

	for(int i = 0; i < song->nSamples; ++i)
		normalized_song[i] = (double)((int16_t*)song->sample_array)[i] / MAX_INT16; 

	for(int i = 0; i < song->nSamples; ++i) {
		normalized_song[i] = (normalized_song[i] - signal_mean_d) / signal_variance_d;
	}

	/* Apply and store NB_BANDS bandpassed and RDFT'd signals */
	for(int i = 0; i < NB_BANDS; ++i) {
		double d = 0;
		double y;
		for(int b = 0; b < iteration_number; b += half_fft_winsize) {
			memset(registry, 0, NB_COEFFS*sizeof(double));
			/* Apply filter */
			for(int j = b; j < b + fft_winsize; ++j) {
				y = 0;
 				for(k = NB_COEFFS - 1; k > 7; --k) {
 					registry[k] = registry[k-1];
 				}
				for(k = 7; k > 0; --k) {
					registry[k] = registry[k-1];
					y += coeffs[i][k] * (registry[k] + registry[NB_COEFFS - 1 - k]);
				}

				y += registry[8] * coeffs[i][8];
				registry[0] = normalized_song[j];
 				y += coeffs[i][0] * (registry[0] + registry[NB_COEFFS - 1]);

 				in[j - b] = y;
			}
			/* End of filter */
			/* Compute RDFT of the filtered signal and store it in filtered_array */
			fftw_execute(p);
			float sum_fft = 0;
			for(k = 0; k < fft_winsize/2 + 1; ++k) {
				double re = out[k][0];
				double im = out[k][1];
				double abs = re*re + im*im;
				fft_array_bp[k] = abs;
				sum_fft += fft_array_bp[k];
			}
			filtered_array[i][(int)floor(d / double_fft_winsize)] += sum_fft;
			d += double_fft_winsize;
			/* End of RDFT */
		}
	}
	/* End of filterbank */
	free(normalized_song);

	/* Part two: process the filtered signal a bit more */

	// Create two ill-named temporary arrays to avoid allocating five well-named ones
	double *temp_filtered_array1[NB_BANDS];
	double *temp_filtered_array2[NB_BANDS];
	double *weighted_average[NB_BANDS];
	// Hold the sum of the band's intensity
	double *smoothed_sum;
	// Hold the low pass registry
	double registry2[7];
	// Coefficients values extracted from the paper (see above)
	float mu = 100.0;
	float lambda = 0.8;
	double atk_sum = 0;
	double c, d;

	for(int i = 0; i < NB_BANDS; ++i) {
		temp_filtered_array1[i] = calloc(2*nb_frames, sizeof(double));
		temp_filtered_array2[i] = calloc(2*nb_frames, sizeof(double));
		weighted_average[i] = calloc(2*nb_frames, sizeof(double));
	}
	smoothed_sum = calloc(2*nb_frames, sizeof(double));

	double y = 0;

	for(int i = 0; i < NB_BANDS; ++i) { 
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
			for(k = 7; k > 1; --k) {
				registry[k-1] = registry[k-2];
				registry2[k-1] = registry2[k-2];
			}
			registry[0] = temp_filtered_array1[i][j];
			registry2[0] = y;
			
			y = 0;
			d = 0;
			c = 0;
			for(k = 0; k < 7; ++k)
				d += butterb[k] * registry[k];
			for(k = 1; k < 7; ++k)
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

	// Free some arrays to spare the RAM
	for(int i = 0; i < 1; ++i) {
		free(temp_filtered_array1[i]);
		free(temp_filtered_array2[i]);
		free(filtered_array[i]);
	
	}

	fftw_free(out);
	out = (fftw_complex*) fftw_malloc(sizeof(fftw_complex) * 2*nb_frames);

	// Compute final attack rating
	for(int i = 0; i < NB_BANDS; ++i) 
		for(int j = 0; j < nb_frames*2 - 1; ++j)
			atk_sum += weighted_average[i][j];

	/* Part 3: Perform the BPM estimation (finally !) */

	// Hold final bliss scores
	double tempo_score = 0;
	double atk_score = 0;
	// Hold the number of beats detected
	int beat = 0;

	// Sum all bands' weighted average to get the final signal
	for(int j = 0; j < 2*nb_frames - 1; ++j) {
		for(int i = 0; i < NB_BANDS; ++i) {
			smoothed_sum[j] += weighted_average[i][j];
		}
	}

	// Apply two rectangular smoothing filters to prepare the peak detection
	// Use weighted_average[0] as a temporary array 
	bl_rectangular_filter(weighted_average[0], smoothed_sum, 2*nb_frames, 19);
	for(int k = 0; k < 2*nb_frames; ++k)
		smoothed_sum[k] = 0;
	bl_rectangular_filter(smoothed_sum, weighted_average[0], 2*nb_frames, 19);

	for(int i = 0; i < 1; ++i)
		free(weighted_average[i]);

	float epsilon = 0.000001;

	for(int j = 1; j < 2*nb_frames - 1; ++j)
		if(((smoothed_sum[j] - smoothed_sum[j-1]) > epsilon) && ((smoothed_sum[j] - smoothed_sum[j+1]) > epsilon))
			beat++;
	
	// Compute final attack and tempo ratings
	tempo_score = 4 * (float) beat / (float) song->duration - 30.4;
	atk_score = -1.74 * atk_sum * 10000 / song->nSamples + 58.3;

	result->tempo = tempo_score;
	result->attack = atk_score;

	// Free everything
	fftw_free(in);
	fftw_free(out);

	free(smoothed_sum);
	fftw_destroy_plan(p);
	fftw_cleanup();
}
