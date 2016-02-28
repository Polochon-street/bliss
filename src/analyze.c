// System headers
#include <math.h>

// Library headers
#include "bliss.h"


int bl_analyze(char const * const filename,
		struct bl_song * const current_song) {
	float rating;
	struct envelope_result_s envelope_result = {0.0f, 0.0f};

	// Decode audio track
	if(0 == bl_audio_decode(filename, current_song)) {
		// Analyze global envelope
		bl_envelope_sort(current_song, &envelope_result);
		// Analyze tempo
		current_song->force_vector.tempo = envelope_result.tempo;
		// Analyze amplitude
		current_song->force_vector.amplitude = bl_amplitude_sort(current_song);
		// Analyze frequencies
		current_song->force_vector.frequency = bl_frequency_sort(current_song);
		// Analyze attack
		current_song->force_vector.attack = envelope_result.attack;

		// Compute global rating
		rating = (fmax(current_song->force_vector.tempo, 0) +
			current_song->force_vector.amplitude +
			current_song->force_vector.frequency +
			fmax(current_song->force_vector.attack, 0));
		current_song->force = rating;
		if(rating > 0) {
			current_song->calm_or_loud = BL_LOUD;
        } 
		else if(rating < 0) {
			current_song->calm_or_loud = BL_CALM;
        } 
		else {
			current_song->calm_or_loud = BL_UNKNOWN;
		}
		return current_song->calm_or_loud;
	} 
	else {
		fprintf(stderr, "Couldn't decode song\n");
		return BL_UNEXPECTED;
	}
}

float bl_distance(struct force_vector_s v_song1,
		struct force_vector_s v_song2) {
	struct force_vector_s v1, v2;
	float distance;

	// TODO: Error handling

	v1 = v_song1; 
	v2 = v_song2;

	distance = sqrt(
		(v1.tempo - v2.tempo) * (v1.tempo - v2.tempo) +
		(v1.amplitude - v2.amplitude) * (v1.amplitude - v2.amplitude) +
		(v1.frequency - v2.frequency) * (v1.frequency - v2.frequency) +
		(v1.attack - v2.attack) * (v1.attack - v2.attack)
		);

	return distance;
}

float bl_distance_file(char const * const filename1,
		char const * const filename2,
		struct bl_song * song1,
		struct bl_song * song2) {
	struct force_vector_s v1, v2;
	float distance;

	bl_analyze(filename1, song1);
	bl_analyze(filename2, song2);
	// TODO: Error handling

	v1 = song1->force_vector;
	v2 = song2->force_vector;

	distance = sqrt(
		(v1.tempo - v2.tempo) * (v1.tempo - v2.tempo) +
		(v1.amplitude - v2.amplitude) * (v1.amplitude - v2.amplitude) +
		(v1.frequency - v2.frequency) * (v1.frequency - v2.frequency) +
		(v1.attack - v2.attack) * (v1.attack - v2.attack)
	);

	return distance;
}

float bl_cosine_similarity(struct force_vector_s v_song1,
		struct force_vector_s v_song2) {
	struct force_vector_s v1, v2;
	float similarity;

	v1 = v_song1;
	v2 = v_song2;

	similarity = (v1.tempo*v2.tempo + v1.amplitude*v2.amplitude +
			v1.frequency*v2.frequency + v1.attack*v2.attack) / (
			sqrt(v1.tempo*v1.tempo + v1.amplitude*v1.amplitude +
				v1.frequency*v1.frequency + v1.attack*v1.attack) * 
			sqrt(v2.tempo*v2.tempo + v2.amplitude*v2.amplitude +
				v2.frequency*v2.frequency + v2.attack*v2.attack));

	return similarity;
}

float bl_cosine_similarity_file(char const * const filename1,
		char const * const filename2,
		struct bl_song *song1,
		struct bl_song *song2) {
	struct force_vector_s v1, v2;
	float similarity;

	bl_analyze(filename1, song2);
	bl_analyze(filename2, song2);

	v1 = song1->force_vector;
	v2 = song2->force_vector;

	similarity = (v1.tempo*v2.tempo + v1.amplitude*v2.amplitude +
			v1.frequency*v2.frequency + v1.attack*v2.attack) / (
			sqrt(v1.tempo*v1.tempo + v1.amplitude*v1.amplitude +
			v1.frequency*v1.frequency + v1.attack*v1.attack) *
			sqrt(v2.tempo*v2.tempo + v2.amplitude*v2.amplitude +
			v2.frequency*v2.frequency + v2.attack*v2.attack));

	return similarity;
}
