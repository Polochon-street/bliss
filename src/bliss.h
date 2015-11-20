#include <stdio.h>
#include <libavformat/avformat.h>

size_t size;
int cli;

#define MIN(X, Y) (((X) < (Y)) ? (X) : (Y))
#define MAX(X, Y) (((X) > (Y)) ? (X) : (Y))

struct d4vector {
	float x; // Tempo rating
	float y; // Amplitude rating
	float z; // Frequency rating
	float t; // Attack rating
};

struct d3vector {
	float x;
	float y;
	float z;
};

struct d2vector {
	float x;
	float y;
};

struct bl_song {
	float force;
	struct d4vector force_vector;
	int8_t* sample_array;
	int channels;
	int nSamples;
	int sample_rate;
	int bitrate;
	int nb_bytes_per_sample;
	int resnum;
	int64_t duration;
	char *filename;
	char *artist;
	char *title;
	char *album;
	char *tracknumber;
	char *genre;
};

float bl_amp_sort(struct bl_song, int debug);
struct d2vector bl_envelope_sort(struct bl_song, int debug);
int bl_audio_decode(const char *file, struct bl_song *, int analyze);
int bl_analyze(char *filename, struct bl_song *, int debug, int analyze); 
float bl_distance(char *filename1, char *filename2, struct bl_song *, struct bl_song *, int debug);
float bl_freq_sort(struct bl_song, int debug);
void bl_free_song(struct bl_song *);
