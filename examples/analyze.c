/**
 * Analyzes and fill in a struct song and output it to stdout.
 */
#include <stdio.h>
#include <bliss.h>


int main (int argc, char **argv) {
	if (argc < 2) {
		fprintf(stderr, "Usage: %s FILE\n", argv[0]);
		return EXIT_FAILURE;
	}

	char const * const filename = argv[1];

	struct bl_song song;
	bl_initialize_song(&song);
	if(bl_analyze(filename, &song) != BL_UNEXPECTED) {
		char calm_or_loud[10] = "";
		if (BL_CALM == song.calm_or_loud) {
			strcpy(calm_or_loud, "Calm");
		}
		else if(BL_LOUD == song.calm_or_loud) {
			strcpy(calm_or_loud, "Loud");
    	}
		else {
			strcpy(calm_or_loud, "Unknown");
		}

		// Debug output
		printf("Analysis for music %s:\n", filename);
		printf("Force: %f\n", song.force);
		printf("Force vector: (%f, %f, %f, %f, %f, %f)\n",
			song.force_vector.tempo1,
			song.force_vector.tempo2,
			song.force_vector.tempo3,
			song.force_vector.amplitude,
			song.force_vector.frequency,
			song.force_vector.attack);
		printf("Channels: %d\n", song.channels);
		printf("Number of samples: %d\n", song.nSamples);
		printf("Sample rate: %d\n", song.sample_rate);
		printf("Bitrate: %d\n", song.bitrate);
		printf("Number of bytes per sample: %d\n", song.nb_bytes_per_sample);
		printf("Calm or loud: %s\n", calm_or_loud);
		printf("Duration: %" PRId64 "\n", song.duration);
		printf("Artist: %s\n", song.artist);
		printf("Title: %s\n", song.title);
		printf("Album: %s\n", song.album);
		printf("Track number: %s\n", song.tracknumber);
		printf("genre: %s\n", song.genre);

		bl_free_song(&song);
		return EXIT_SUCCESS;
	}
	else {
		fprintf(stderr, "Couldn't analyze song\n");
		bl_free_song(&song);
		return EXIT_FAILURE;
	}
}

