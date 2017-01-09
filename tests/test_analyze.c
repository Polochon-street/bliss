#include "bliss.h"

#include <math.h>


void assert_floateq(double a, double b) {
    const float EPSILON = 0.000001;
    if(fabs(a - b) > EPSILON) {
		printf("%f %f\n", a, b);
        exit(-1);
	}
}


void assert_eq(int a, int b) {
    if(a != b) {
		printf("%d %d\n", a,b);
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
    bl_analyze("../audio/song.mp3", &song);

    assert_floateq(song.force, -1.349859);

    assert_floateq(song.force_vector.tempo, -0.110247);
    assert_floateq(song.force_vector.amplitude, 0.197553);
    assert_floateq(song.force_vector.frequency, -1.547412);
    assert_floateq(song.force_vector.attack, -1.621171);
    assert_eq(song.channels, 2);

    assert_eq(song.nSamples, 12508554);

    assert_eq(song.sample_rate, 22050);

    assert_eq(song.bitrate, 198332);
    assert_eq(song.nb_bytes_per_sample, 2);

    assert_eq(song.calm_or_loud, BL_CALM);
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
