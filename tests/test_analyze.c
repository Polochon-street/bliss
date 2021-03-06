#include "bliss.h"

#include <math.h>

void assert_floateq(double a, double b) {
  const float EPSILON = 0.00001;
  if (fabs(a - b) > EPSILON) {
    printf("%f %f\n", a, b);
    exit(-1);
  }
}

void assert_eq(int a, int b) {
  if (a != b) {
    printf("%d %d\n", a, b);
    exit(-1);
  }
}

void assert_streq(char const *const str1, char const *const str2) {
  if (strcmp(str1, str2) != 0) {
    exit(-1);
  }
}

void test_analyze_s16(void) {
  struct bl_song song;
  bl_analyze("../audio/song.flac", &song);

  assert_floateq(song.force, -20.777929);

  assert_floateq(song.force_vector.tempo, -8.945454);
  assert_floateq(song.force_vector.amplitude, -10.641844);
  assert_floateq(song.force_vector.frequency, -10.136086);
  assert_floateq(song.force_vector.attack, -15.560563);
  assert_eq(song.channels, 2);

  assert_eq(song.nSamples, 488138);

  assert_eq(song.sample_rate, 22050);

  assert_eq(song.bitrate, 233864);
  assert_eq(song.nb_bytes_per_sample, 2);

  assert_eq(song.duration, 11);

  assert_streq(song.artist, "David TMX");

  assert_streq(song.title, "Renaissance");

  assert_streq(song.album, "Renaissance");

  assert_streq(song.tracknumber, "02");

  assert_streq(song.genre, "Pop");
  bl_free_song(&song);
}

void test_analyze_s32(void) {
  struct bl_song song;
  bl_analyze("../audio/song_s32.flac", &song);

  assert_floateq(song.force, -20.821571);

  assert_floateq(song.force_vector.tempo, -8.218182);
  assert_floateq(song.force_vector.amplitude, -10.641695);
  assert_floateq(song.force_vector.frequency, -10.179875);
  assert_floateq(song.force_vector.attack, -15.561186);
  assert_eq(song.channels, 2);

  assert_eq(song.nSamples, 488140);

  assert_eq(song.sample_rate, 22050);

  assert_eq(song.bitrate, 840742);
  assert_eq(song.nb_bytes_per_sample, 2);

  assert_eq(song.duration, 11);

  assert_streq(song.artist, "David TMX");

  assert_streq(song.title, "Renaissance");

  assert_streq(song.album, "Renaissance");

  assert_streq(song.tracknumber, "02");

  assert_streq(song.genre, "Pop");
  bl_free_song(&song);
}

int main(void) {
  test_analyze_s16();
  test_analyze_s32();
}
