#include "bliss.h"

int bl_audio_decode(
        char const * const filename,
        struct bl_song * const song) {
    // Contexts and libav variables
    AVPacket avpkt;
	AVFormatContext* context;
	int audio_stream;
	AVCodecContext* codec_context = NULL;
	AVCodec *codec = NULL;
    AVFrame *decoded_frame = NULL;

    // Size of the samples
    uint64_t size = 0;

    // Dictionary to fetch tags
	AVDictionaryEntry *tags_dictionary;

    // Planar means channels are interleaved in data section
    // See MP3 vs FLAC for instance.
	int is_planar;

    // Pointer to beginning of music data
	int8_t *beginning;
    // Received frame holder
	int got_frame;
    // Position in the data buffer
	int index;

    // Initialize AV lib
	av_register_all();
	context = avformat_alloc_context();

    // Open input file
	if (avformat_open_input(&context, filename, NULL, NULL) < 0) {
		fprintf(stderr, "Couldn't open file: %s. Error %d encountered.\n", filename, errno);
		return BL_UNEXPECTED;
	}

    // Search for a valid stream
	if (avformat_find_stream_info(context, NULL) < 0) {
		fprintf(stderr, "Couldn't find stream information\n");
		return BL_UNEXPECTED;
	}

    // Get audio stream
    audio_stream = av_find_best_stream(context, AVMEDIA_TYPE_AUDIO, -1, -1, &codec, 0);
	if (audio_stream < 0) {
		fprintf(stderr, "Couldn't find a suitable audio stream\n");
		return BL_UNEXPECTED;
	}
    // Find associated codec
	codec_context = context->streams[audio_stream]->codec;
	if (!codec_context) {
		fprintf(stderr, "Codec not found!\n");
		return BL_UNEXPECTED;
	}
	if (avcodec_open2(codec_context, codec, NULL) < 0) {
		fprintf(stderr, "Could not open codec\n");
		return BL_UNEXPECTED;
	}

    // Fill song properties
    song->filename = malloc(strlen(filename) + 1);
    strcpy(song->filename, filename);

	song->sample_rate = codec_context->sample_rate;
	song->duration = context->duration / AV_TIME_BASE;
    song->bitrate = context->bit_rate;

    // Get number of samples
	size = (
            ((uint64_t)(context->duration) * (uint64_t)song->sample_rate) /
            ((uint64_t)AV_TIME_BASE)
           ) *
        codec_context->channels *
        av_get_bytes_per_sample(codec_context->sample_fmt);
	song->nSamples = (
            (
             ((uint64_t)(context->duration) * (uint64_t)song->sample_rate) /
             ((uint64_t)AV_TIME_BASE)
            ) *
        codec_context->channels);

    // Allocate sample_array
	song->sample_array = malloc(size);
	for (unsigned int i = 0; i < size; ++i) {
		song->sample_array[i] = 0;
    }

	beginning = song->sample_array;
	index = 0;

	song->nb_bytes_per_sample = av_get_bytes_per_sample(codec_context->sample_fmt);
	song->channels = codec_context->channels;

    // Zero initialize tags
	song->artist = NULL;
    song->title = NULL;
    song->album = NULL;
    song->tracknumber = NULL;

	// Initialize tracknumber tag
    tags_dictionary = av_dict_get(context->metadata, "track", NULL, 0);
	if (tags_dictionary!= NULL) {
		song->tracknumber = malloc(strlen(tags_dictionary->value) + 1);
		strcpy(song->tracknumber, tags_dictionary->value);
		song->tracknumber[strcspn(song->tracknumber, "/")] = '\0';
	} else {
		song->tracknumber = malloc(1 * sizeof(char));
		strcpy(song->tracknumber, "");
	}

    // Initialize title tag
    tags_dictionary = av_dict_get(context->metadata, "title", NULL, 0);
	if (tags_dictionary!= NULL) {
		song->title = malloc(strlen(tags_dictionary->value) + 1);
		strcpy(song->title, tags_dictionary->value);
	} else {
		song->title = malloc(12 * sizeof(char));
		strcpy(song->title, "<no title>");
	}

    // Initialize artist tag
    tags_dictionary = av_dict_get(context->metadata, "ARTIST", NULL, 0);
	if (tags_dictionary!= NULL) {
		song->artist= malloc(strlen(tags_dictionary->value) + 1);
		strcpy(song->artist, tags_dictionary->value);
	} else {
		song->artist= malloc(12 * sizeof(char));
		strcpy(song->artist, "<no artist>");
	}

    // Initialize album tag
    tags_dictionary = av_dict_get(context->metadata, "ALBUM", NULL, 0);
	if (tags_dictionary!= NULL) {
		song->album= malloc(strlen(tags_dictionary->value) + 1);
        strcpy(song->album, tags_dictionary->value);
	} else {
		song->album= malloc(11 * sizeof(char));
		strcpy(song->album, "<no album>");
	}

    // Initialize genre tag
    tags_dictionary = av_dict_get(context->metadata, "genre", NULL, 0);
	if (tags_dictionary!= NULL) {
		song->genre= malloc(strlen(tags_dictionary->value) + 1);
        strcpy(song->genre, tags_dictionary->value);
	} else {
		song->genre = malloc(11 * sizeof(char));
		strcpy(song->genre, "<no genre>");
	}

    // Planar means channels are not interleaved
    is_planar = av_sample_fmt_is_planar(codec_context->sample_fmt);

    // Read the whole data and copy them into a huge buffer
    av_init_packet(&avpkt);
    while(av_read_frame(context, &avpkt) >= 0) {
        if(avpkt.stream_index == audio_stream) {
            got_frame = 0;

            // If decoded frame has not been allocated yet
            if (!decoded_frame) {
                // Try to allocate it
                decoded_frame = av_frame_alloc();
                if(!decoded_frame) {
                    fprintf(stderr, "Could not allocate audio frame\n");
                    return BL_UNEXPECTED;
                }
            } else {
                // Else, unreference it and reset fields
                av_frame_unref(decoded_frame);
            }

            int length = avcodec_decode_audio4(codec_context,
                    decoded_frame,
                    &got_frame,
                    &avpkt);
            if (length < 0) {
                avpkt.size = 0;
            }

            av_free_packet(&avpkt);

            // Copy decoded data into a huge array
            if (got_frame) {
                size_t data_size = av_samples_get_buffer_size(
                        NULL,
                        codec_context->channels,
                        decoded_frame->nb_samples,
                        codec_context->sample_fmt,
                        1);

                if ((index * song->nb_bytes_per_sample + data_size) > size) {
                    size += data_size;
                    beginning = realloc(beginning, size);
                    song->nSamples += data_size / song->nb_bytes_per_sample;
                }
                int8_t *p = beginning + (index * song->nb_bytes_per_sample);
                if (1 == is_planar) {
                    for (int i = 0;
                         i < (decoded_frame->nb_samples * song->nb_bytes_per_sample);
                         i += song->nb_bytes_per_sample) {
                        for (int j = 0; j < codec_context->channels; ++j) {
                            for (int k = 0; k < song->nb_bytes_per_sample; ++k) {
                                *p = ((int8_t*)(decoded_frame->extended_data[j]))[i + k];
                                ++p;
                            }
                        }
                    }
                    index += data_size / song->nb_bytes_per_sample;
                } else if (0 == is_planar) {
                    memcpy((index * song->nb_bytes_per_sample) + beginning,
                            decoded_frame->extended_data[0],
                            data_size);
                    index += data_size / song->nb_bytes_per_sample;
                }
            }
        } else {
            // Dropping packets that do not belong to the audio stream
            // (such as album cover)
            av_free_packet(&avpkt);
        }
    }
	song->sample_array = beginning;

    // Free memory
	avpkt.data = NULL;
	avpkt.size = 0;

    // Read the end of audio, as precognized in http://ffmpeg.org/pipermail/libav-user/2015-August/008433.html
	do {
		avcodec_decode_audio4(codec_context, decoded_frame, &got_frame, &avpkt);
	} while(got_frame);

    // Free memory
	avcodec_close(codec_context);
	av_frame_unref(decoded_frame);
	# if LIBAVUTIL_VERSION_MAJOR > 51
	av_frame_free(&decoded_frame);
	#endif
	av_free_packet(&avpkt);
	avformat_close_input(&context);

	return BL_OK;
}
