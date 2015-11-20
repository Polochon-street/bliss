#include <stdio.h>
#include <bliss.h>

int main (int argc, char **argv) {
	char *filename1 = argv[1];
	char *filename2 = argv[2];

	int debug = 1; /* 1 to enable some debug info, 0 otherwise */
	float distance;
	float threshold = 10; /* For example */

	struct bl_song song1;
	struct bl_song song2;
	
	distance = bl_distance(filename1, filename2, &song1, &song2, debug);

	if(distance > threshold) 
		/* Do something */
		;
	else
		/* Do something else */
		;
	return 0;
}

