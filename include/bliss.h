#ifndef BL_BLISS_H_
#define BL_BLISS_H_

#include <stdio.h>
#include <libavformat/avformat.h>

static const int BL_LOUD = 0;
static const int BL_CALM = 1;
static const int BL_UNKNOWN = 2;
static const int BL_UNEXPECTED = 3;
static const int BL_OK = 0;

struct force_vector_s {
	float tempo;
	float amplitude;
	float frequency;
	float attack;
};


struct envelope_result_s {
	float tempo;
	float attack;
};


struct bl_song {
	float force;
	struct force_vector_s force_vector;
	int8_t* sample_array;
	int channels;
	int nSamples;
	int sample_rate;
	int bitrate;
	int nb_bytes_per_sample;
	int calm_or_loud;
	int64_t duration;
	char* filename;
	char* artist;
	char* title;
	char* album;
	char* tracknumber;
	char* genre;
};


/**
 * Run the analysis on the given song.
 *
 * @param[in] filename  is the filename of the song to analyze.
 * @param[out] current_song  is the resulting `bl_song` structure after
 * analysis.
 *
 * @return A value characterizing the song, whether calm, loud or
 * error-specific.
 */
int bl_analyze(char const * const filename,
        struct bl_song * current_song);


/**
 * Compute the distance between two songs stored in audio files.
 *
 * @remark Distance is computed using a standard euclidian distance between
 * force vectors.
 *
 * @param[in] filename1  is the path to the first song to compare.
 * @param[in] filename2  is the path to the second song to compare.
 * @param[out] song1  is the resulting `bl_song` structure for the first song,
 *                    after analysis.
 * @param[out] song2  is the resulting `bl_song` structure for the second song,
 *                    after analysis.
 *
 * @return The distance between the two songs stored in audio files.
 */
float bl_distance_file(
        char const * const filename1,
        char const * const filename2,
        struct bl_song * song1,
        struct bl_song * song2);

/**
 * Compute the distance between two songs.
 *
 * @remark Distance is computed using a standard euclidian distance between
 * force vectors.
 *
 * @param[in] filename1  is the path to the first song to compare.
 * @param[in] filename2  is the path to the second song to compare.
 * @param[out] song1  is the resulting `bl_song` structure for the first song,
 *                    after analysis.
 * @param[out] song2  is the resulting `bl_song` structure for the second song,
 *                    after analysis.
 *
 * @return The distance between the two songs.
 */
float bl_distance(
		struct bl_song * song1,
        struct bl_song * song2);


/**
 * Compute the cosine similarity between two songs stored in audio files.
 *
 * @param[in] filename1  is the path to the first song to compare.
 * @param[in] filename2  is the path to the second song to compare.
 * @param[out] song1  is the resulting `bl_song` structure for the first song,
 *                    after analysis.
 * @param[out] song2  is the resulting `bl_song` structure for the second song,
 *                    after analysis.
 *
 * @return The cosine similarity between the two songs stored in audio files.
 */
float bl_cosine_similarity_file(
        char const * const filename1,
        char const * const filename2,
        struct bl_song * song1,
        struct bl_song * song2);


/**
 * Compute the cosine similarity between two songs.
 *
 * @param[in] filename1  is the path to the first song to compare.
 * @param[in] filename2  is the path to the second song to compare.
 * @param[out] song1  is the resulting `bl_song` structure for the first song,
 *                    after analysis.
 * @param[out] song2  is the resulting `bl_song` structure for the second song,
 *                    after analysis.
 *
 * @return The cosine similarity between the two songs.
 */
float bl_cosine_similarity(
        struct bl_song * song1,
        struct bl_song * song2);



/**********************
 * Specific analyzers *
 **********************/

/**
 * Compute envelope-related characteristics: tempo and attack ratings.
 *
 * The tempo rating draws the envelope of the whole song, and then computes its
 * DFT, obtaining peaks at the frequency of each dominant beat. The period of
 * each dominant beat can then be deduced from the frequencies, hinting at the
 * song's tempo.
 *
 * Warning: the tempo is not equal to the force of the song. As an example , a
 * heavy metal track can have no steady beat at all, giving a very low tempo score
 * while being very loud.
 *
 * The attack rating computes the difference between each value in the envelope
 * and the next (its derivative).
 * The final value is obtained by dividing the sum of the positive derivates by
 * the number of samples, in order to avoid different results just because of
 * the songs' length.
 * As you have already guessed, a song with a lot of attacks also tends to wake
 * humans up very quickly.
 *
 * @param[in]  song  the song to analyze.
 * @param[out] result  an `envelope_result_s` structure to handle the resulting
 * ratings.
 */
void bl_envelope_sort(struct bl_song const * const song,
        struct envelope_result_s * result);


/**
 * Compute amplitude rating.
 *
 * The amplitude rating reprents the physical « force » of the song, that is,
 * how much the speaker's membrane will move in order to create the sound.
 * It is obtained by applying a magic formula with magic coefficients to a
 * histogram of the values of all the song's samples
 *
 * @param[in]  song  the song to analyze.
 *
 * @return  the amplitude rating.
 */
float bl_amplitude_sort(struct bl_song const * const song);


/**
 * Compute frequency rating.
 *
 * The frequency rating is a ratio between high and low frequencies: a song
 * with a lot of high-pitched sounds tends to wake humans up far more easily.
 * This rating is obtained by performing a DFT over the sample array, and
 * splitting the resulting array in 4 frequency bands: low, mid-low, mid,
 * mid-high, and high. Using the value in dB for each band, the final formula
 * corresponds to freq_result = high + mid-high + mid - (low + mid-low)
 *
 * @param[in] song  the song to analyze.
 *
 * @return  the frequency rating for this song.
 */
float bl_frequency_sort(struct bl_song const * const song);


/***********
 * Decoder *
 ***********/

/**
 * Decode specified audio file.
 *
 * Decode the specified audio file with libAV and fill in the song structure.
 *
 * @param[in] filename  name of the file to decode and load.
 * @param[out] song  the `bl_song` song structure to fill.
 *
 * @return `BL_OK` if everything went fine, `BL_UNEXPECTED` otherwise.
 */
int bl_audio_decode(char const * const filename,
        struct bl_song * const song);


/***********
 * Helpers *
 * *********/

/**
 * Free the dynamically allocated memory to store song data.
 *
 * @param song  a `bl_song` struct representing the song to free.
 */
void bl_free_song(struct bl_song * const song);

#endif  // BL_BLISS_H_
