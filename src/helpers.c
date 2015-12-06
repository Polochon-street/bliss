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

float bl_version(void) {
	printf("Using bliss analyzer version %0.1f.\n", BL_VERSION);
	return (float)BL_VERSION;
}
