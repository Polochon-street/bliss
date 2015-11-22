/**
 * Compute distance between two songs and output it to stdout.
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
	float distance = bl_distance(filename1, filename2, &song1, &song2);

    printf("Distance between %s and %s is: %f\n", filename1, filename2, distance);

	return EXIT_SUCCESS;
}

