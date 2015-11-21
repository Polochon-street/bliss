#include "bliss.h"

// We map 16-bits values onto the histogram
static const int HISTOGRAM_SIZE = 32769;
// Number of passes in histogram smoothing
static const int N_PASSES = 300;
// Limits of the integral on the histogram
static const int INTEGRAL_INF = 0;
static const int INTEGRAL_SUP = 2000;


float bl_amplitude_sort(struct bl_song const * const song) {
    // Start and end offsets of the data in the sample_array
	int start;
    int end;

    // Histogram array
	float histogram[HISTOGRAM_SIZE];
    // Smoothed histogram array
	float histogram_smooth[HISTOGRAM_SIZE];

    // Mapping of 32 bits values onto 16 bits of the histogram
	float quot_32to16 = (float)(HISTOGRAM_SIZE - 1) / (float)(1u << 31);

    // Integral of the histogram
	float histogram_integral = 0;

    // Zero initialize histograms
	for(int i = 0; i < HISTOGRAM_SIZE; ++i) {
		histogram[i] = 0.;
		histogram_smooth[i] = 0.;
	}

    // Fill-in histograms
	if(2 == song->nb_bytes_per_sample) {
        // Find beginning of data
		for(start = 0; ((int16_t*)song->sample_array)[start] == 0; ++start) {
		}
        // Find end of data
		for(end = song->nSamples - 1; ((int16_t*)song->sample_array)[end] == 0; --end) {
		}
        // Add values to the histogram
		int16_t* p16 = (int16_t*)song->sample_array + start;
		for(int i = start; i <= end; ++i) {
			histogram[abs(*p16)] += 1;
            ++p16;
		}
	} else if(4 == song->nb_bytes_per_sample) {
        // Find beginning of data
		for(start = 0; 0 == ((int32_t*)song->sample_array)[start]; ++start) {
        }
        // Find end of data
		for(end = song->nSamples - 1; 0 == ((int32_t*)song->sample_array)[end]; --end) {
        }
        // Add values to the histogram
		int32_t* p32 = (int32_t*)song->sample_array + start;
		for(int i = start; i <= end; ++i) {
            // Histogram has 2^16 fields, then we have to map 32 bits value
            // onto 16 bits
			histogram[(uint16_t)fabs((float)(*p32) * quot_32to16)] += 1;
            ++p32;
		}
	}

    // Compute smoothed histogram
    // TODO: Where does it come from?
	for(int g = 0; g <= N_PASSES; ++g) {
		histogram_smooth[0] = histogram[0];
		histogram_smooth[1] = 1. / 4. * (
                histogram[0] + (2 * histogram[1]) + histogram[2]);
		histogram_smooth[2] = 1. / 9. * (
                histogram[0] + (2 * histogram[1]) +
                (3 * histogram[2]) + (2 * histogram[3]) +
                histogram[4]);
		for(int i = 3; i < HISTOGRAM_SIZE - 5; ++i) {
			histogram_smooth[i] = 1. / 27. * (
                        histogram[i-3] + (3 * histogram[i-2]) +
                        (6 * histogram[i-1]) + (7 * histogram[i]) +
                        (6 * histogram[i+1]) + (3 * histogram[i+2]) +
                        histogram[i+3]);
        }
		for(int i = 3; i < HISTOGRAM_SIZE - 5; ++i) {
			histogram[i] = histogram_smooth[i];
        }
	}

    // Normalize it
    // TODO: Why?
	for(int i = 0; i < HISTOGRAM_SIZE; ++i) {
		histogram_smooth[i] /= (start - end);
		histogram_smooth[i] *= 100.;
		histogram_smooth[i] = fabs(histogram_smooth[i]);
	}

    // Compute integral of the smoothed histogram
	for(int i = INTEGRAL_INF; i <= INTEGRAL_SUP; ++i) {
		histogram_integral += histogram_smooth[i];
    }

    // TODO: Where does it come from?
	return (-0.2f * histogram_integral + 6.0f);
}
