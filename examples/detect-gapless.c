/**
 * Tries to detect if two songs are linked together 
 * Returns 1 if two songs are linked, 0 otherwise
 * WORK IN PROGRESS
 */
#include <stdio.h>
#include <bliss.h>

int main (int argc, char **argv) {
	if (argc < 3) {
        fprintf(stderr, "Usage: %s FILE_1 FILE_2\n", argv[0]);
        return EXIT_FAILURE;
    }

	char const * const filename1 = argv[1];
	char const * const filename2 = argv[2];

	struct bl_song song1;
	struct bl_song song2;

	float diff_chan1 = 1;
	float diff_chan2 = 1;

	bl_audio_decode(filename1, &song1);
	bl_audio_decode(filename2, &song2);

	printf("Song 1\n");
	printf("%"PRId16"\n", ((int16_t*)song1.sample_array)[song1.nSamples]);
	printf("%"PRId16"\n", ((int16_t*)song1.sample_array)[song1.nSamples-1]);
	
	printf("Song 2\n");
	printf("%"PRId16"\n", ((int16_t*)song2.sample_array)[0]);
	printf("%"PRId16"\n", ((int16_t*)song2.sample_array)[1]);
	
	if(abs(((int16_t*)song1.sample_array)[song1.nSamples]) >= 5 && abs(((int16_t*)song2.sample_array)[0]) >= 5) {
		diff_chan1 = fabs((((float)((int16_t*)song1.sample_array)[song1.nSamples] - ((int16_t*)song2.sample_array)[0]) / (float)INT16_MAX));
	}

	if(abs(((int16_t*)song1.sample_array)[song1.nSamples-1]) >= 5 && abs(((int16_t*)song2.sample_array)[1]) >= 5) {
		diff_chan2 = fabs((((float)((int16_t*)song1.sample_array)[song1.nSamples-1] - ((int16_t*)song2.sample_array)[1]) / (float)INT16_MAX));
	}

	printf("Difference between two songs (channel 1): %f\n", diff_chan1);
	printf("Difference between two songs (channel 2): %f\n", diff_chan2);

	if(diff_chan1 < 0.01 || diff_chan2 < 0.01) {
		printf("Gapless!\n");
		return 1;
	}
	else {
		printf("Not Gapless.\n");
		return 0;
	}
}

