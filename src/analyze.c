#include "bliss.h"

void bl_free_song(struct bl_song *song) {
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
}

int bl_analyze(char *filename, struct bl_song *current_song, int debug, int analyze) { // add debug flag, and only decode flag
	float resnum;
	struct d2vector envelope_result;

	if(debug)
		printf("\nAnalyzing: %s\n\n", filename);

	if(bl_audio_decode(filename, current_song, analyze) == 0) { // Decode audio track
		if(analyze) {
			envelope_result = bl_envelope_sort(*current_song, debug); // Global envelope sort
			current_song->force_vector.x = envelope_result.x; // Tempo sort
			current_song->force_vector.y = bl_amp_sort(*current_song, debug); // Amplitude sort
			current_song->force_vector.z = bl_freq_sort(*current_song, debug); // Freq sort 
			current_song->force_vector.t = envelope_result.y; // Attack sort

			resnum = MAX(current_song->force_vector.x, 0) + current_song->force_vector.y + current_song->force_vector.z + MAX(current_song->force_vector.t, 0); 

			if(debug)
				printf("\n-> Final Result: %f\n", resnum);

			if(resnum > 0) {
				if(debug)
					printf("Loud\n");
				return 0;
			}
			if(resnum < 0) {
				if(debug)
					printf("Calm\n");
				return 1;
			}
			else {
				printf("Couldn't conclude\n");
				return 2;
			}
		}
		else {
			current_song->force_vector.x = 0;
			current_song->force_vector.y = 0;
			current_song->force_vector.z = 0;
			current_song->force_vector.t = 0;
			return 2;
		}
	}
	else {
		printf("Couldn't decode song\n");
		return 3;
	}
}

float bl_distance(char *filename1, char *filename2, struct bl_song *song1, struct bl_song *song2, int debug) {
	struct d2vector envelope_result1;
	struct d2vector envelope_result2;
	struct d4vector v1, v2;
	float distance;

	bl_analyze(filename1, song1, debug, 1);
	bl_analyze(filename2, song2, debug, 1);

	v1 = song1->force_vector;
	v2 = song2->force_vector;

	distance = sqrt((v1.x - v2.x)*(v1.x - v2.x) + (v1.y - v2.y)*(v1.y - v2.y)
		+ (v1.z - v2.z)*(v1.z - v2.z) + (v1.t - v2.t)*(v1.t - v2.t));

	if(debug) {
		printf("Computed distance: %f\n", distance);
	}

	return distance;
}
