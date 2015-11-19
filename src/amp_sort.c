#include "bliss.h"
#define SIZE 32769
#define SIZE_32 (1u << 31)
#define INT_INF 0
#define INT_SUP 2000
#define max( a, b ) ( ((a) > (b)) ? (a) : (b) )

float bl_amp_sort(struct bl_song song, int debug) {
	int i, d, e, g;
	int histogram_count;
	float histogram[SIZE];
	float histogram_smooth[SIZE];
	float histogram_temp[SIZE];
	float histogram_integ = 0;
	int passe = 300;
	int16_t* p16;
	int32_t* p32;
	FILE *file_amp;
	float quot_32to16 = (float)(SIZE-1)/(float)SIZE_32;
	float resnum_amp = 0;
	
	for(i = 0; i < SIZE; ++i) {
		histogram[i] = '\0';
		histogram_smooth[i] = '\0';
		histogram_temp[i] = '\0';
	}
	
	if(song.nb_bytes_per_sample == 2) {
		for(d = 0; ((int16_t*)song.sample_array)[d] == 0; ++d)
			;
		for(e = song.nSamples - 1; ((int16_t*)song.sample_array)[e] == 0; --e)
			;
		p16 = (int16_t*)song.sample_array + d;
		for(i = d; i <= e; ++i) {
			++histogram[abs(*(p16++))];
		}
	}
	else if(song.nb_bytes_per_sample == 4) {
		for(d = 0; ((int32_t*)song.sample_array)[d] == 0; ++d)
			;
		for(e = song.nSamples - 1; ((int32_t*)song.sample_array)[e] == 0; --e)
			;
		p32 = (int32_t*)song.sample_array + d;
		for(i = d; i <= e; ++i) {
			++histogram[(uint16_t)fabs((float)(*(p32++))*quot_32to16)];
		}
	}
	
	for(i = 0; i < SIZE; ++i)
		histogram_temp[i] = histogram[i];
	histogram_count = e - d;

	for(i = 0;i < SIZE; ++i) {
		histogram[i] /= histogram_count;
		histogram[i] *= 100.;
	}

	for(g = 0;g <= passe; ++g) {
		histogram_smooth[0] = histogram_temp[0];
		histogram_smooth[1] = (float)1/4*(histogram_temp[0] + 2*histogram_temp[1] + histogram_temp[2]);
		histogram_smooth[2] = (float)1/9*(histogram_temp[0] + 2*histogram_temp[1] + 3*histogram_temp[2] + 2*histogram_temp[3] + histogram_temp[4]);
		for(i = 3; i < SIZE - 5; ++i)
			histogram_smooth[i] = (float)1/27*(histogram_temp[i-3] + 3*histogram_temp[i-2] + 6*histogram_temp[i-1] + 7*histogram_temp[i]
			+ 6*histogram_temp[i+1] + histogram_temp[i+2] * 3+histogram_temp[i+3]);
		for(i = 3; i < SIZE - 5; ++i)
			histogram_temp[i] = histogram_smooth[i];
	}

	for(i = 0; i < SIZE; ++i) {
		histogram_smooth[i] /= histogram_count; 
		histogram_smooth[i] *= 100.;
		histogram_smooth[i] = fabs(histogram_smooth[i]);
	}

	for(i = 0; i <= INT_SUP ;++i)
		histogram_integ += histogram_smooth[i];

	resnum_amp = -0.2f * (float)histogram_integ + 6.0f;
	if (debug) {
		file_amp = fopen("file_amp.txt", "w");
		for(i = 0; i < SIZE; ++i)
			fprintf(file_amp, "%d\n", histogram_smooth[i]);
		printf("\n");
		printf("-> Debug amplitudes\n");
		printf("Criterion: loud < 25 < 30 < 35 < calm\n");
		printf("Histogram integration: %f\n", histogram_integ);
		printf("Amplitude result: %f\n", resnum_amp);	
		fclose(file_amp);
	}

	return (resnum_amp);
}
