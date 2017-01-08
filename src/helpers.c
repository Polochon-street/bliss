#include "bliss.h"

void bl_free_song(struct bl_song * const song) {
	if(song->artist) {
		free(song->artist);
		song->artist = NULL;
	}
	if(song->title) {
		free(song->title);
		song->title = NULL;
	}
	if(song->album) {
		free(song->album);
		song->album = NULL;
	}
	if(song->tracknumber) {
		free(song->tracknumber);
		song->tracknumber = NULL;
	}
	if(song->sample_array) {
		free(song->sample_array);
		song->sample_array = NULL;
	}
	if(song->filename) {
		free(song->filename);
		song->filename = NULL;
	}
	if(song->genre) {
		free(song->genre);
		song->genre = NULL;
	}
}

void bl_initialize_song(struct bl_song *song) {
	song->artist = NULL;
	song->title = NULL;
	song->album = NULL;
	song->tracknumber = NULL;
	song->sample_array = NULL;
	song->filename = NULL;
	song->genre = NULL;
}

float bl_version(void) {
	printf("Using bliss analyzer version %0.1f.\n", BL_VERSION);
	return (float)BL_VERSION;
}

int bl_mean(int16_t *sample_array, int nSamples) {
	int mean = 0;

	for(int i = 0; i < nSamples; ++i)
		mean += sample_array[i];

	return mean / nSamples;
}

int bl_variance(int16_t *sample_array, int nSamples, int mean) {
	int64_t variance = 0;

	for(int i = 0; i < nSamples; i++) {
		int32_t v;
		v = sample_array[i] - mean;
		variance += v*v;
	}

	return variance / nSamples;
} 
