#include "bliss.h"

#include <math.h>


void assert_floateq(double a, double b) {
    const float EPSILON = 0.000001;
    if(fabs(a - b) > EPSILON) {
        exit(-1);
    }
}


void assert_eq(int a, int b) {
    if(a != b) {
        exit(-1);
    }
}


void assert_streq(char const * const str1, char const * const str2) {
    if(strcmp(str1, str2) != 0) {
        exit(-1);
    }
}


void test_loud(void) {
    struct bl_song song;
    bl_analyze("../audio/loud.mp3", &song);

    assert_floateq(song.force, 11.405784);


	// TODO Correct values
    assert_floateq(song.force_vector.tempo1, 0);
    assert_floateq(song.force_vector.tempo2, 0);
    assert_floateq(song.force_vector.tempo3, 0);
    assert_floateq(song.force_vector.amplitude, 0.107364);
    assert_floateq(song.force_vector.frequency, -1.432200);
    assert_floateq(song.force_vector.attack, 10.213614);

    assert_eq(song.channels, 2);

    assert_eq(song.nSamples, 25017174);

    assert_eq(song.sample_rate, 44100);

    assert_eq(song.bitrate, 198332);

    assert_eq(song.nb_bytes_per_sample, 2);

    assert_eq(song.calm_or_loud, BL_LOUD);

    assert_eq(song.duration, 283);

    assert_streq(song.artist, "David TMX");

    assert_streq(song.title, "Lost in dreams");

    assert_streq(song.album, "Renaissance");

    assert_streq(song.tracknumber, "14");

    assert_streq(song.genre, "(255)");
	bl_free_song(&song);
}


int main(void) {
    test_loud();
}
