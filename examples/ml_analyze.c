/**
 * Analyzes and fill in a struct song and output it to stdout.
 */
#include <bliss.h>
#include <stdio.h>

int main(int argc, char **argv) {
  if (argc < 2) {
    fprintf(stderr, "Usage: %s FILE\n", argv[0]);
    return EXIT_FAILURE;
  }

  char const *const filename = argv[1];

  struct bl_song song;
  bl_initialize_song(&song);
  bl_analyze(filename, &song);
  printf("%s;%f;%f;%f;%f\n", song.title, song.force_vector.tempo,
         song.force_vector.amplitude, song.force_vector.frequency,
         song.force_vector.attack);
  bl_free_song(&song);
}
