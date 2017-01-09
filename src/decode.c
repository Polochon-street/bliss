#include <libswresample/swresample.h>
#include <libavutil/opt.h>

#include "bliss.h"

#define NB_BYTES_PER_SAMPLE 2
#define SAMPLE_RATE 22050

int bl_audio_decode(
		char const * const filename,
		struct bl_song * const song) {
	int ret;
	// Contexts and libav variables
	AVPacket avpkt;
	AVFormatContext* context;
	int audio_stream;
	AVCodecContext *codec_context = NULL;
	#if LIBSWRESAMPLE_VERSION_MAJOR >= 2
	AVCodecParameters *codecpar = NULL;
	#endif
	AVCodec *codec = NULL;
	AVFrame *decoded_frame = NULL;
	struct SwrContext *swr_ctx;

	// Size of the samples
	uint64_t size = 0;

	// Dictionary to fetch tags
	AVDictionaryEntry *tags_dictionary;

	// Pointer to beginning of music data
	int8_t *beginning;
	// Received frame holder
	int got_frame;
	// Position in the data buffer
	int index;
	// Initialize AV lib
	av_register_all();
	context = avformat_alloc_context();

	av_log_set_level(AV_LOG_QUIET);

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

	// Find stream and corresponding codec
	audio_stream = av_find_best_stream(context, AVMEDIA_TYPE_AUDIO, -1, -1, &codec, 0);
	if (audio_stream < 0) {
		fprintf(stderr, "Couldn't find a suitable audio stream\n");
		return BL_UNEXPECTED;
	}

	#if LIBSWRESAMPLE_VERSION_MAJOR < 2
	codec_context = context->streams[audio_stream]->codec;
	if (!codec_context) {
		fprintf(stderr, "Codec not found!\n");
		return BL_UNEXPECTED;
	}
	#else
	// Find codec parameters
	codecpar = context->streams[audio_stream]->codecpar;

	// Find and allocate codec context
	codec_context = avcodec_alloc_context3(codec);
	#endif

	if (avcodec_open2(codec_context, codec, NULL) < 0) {
		fprintf(stderr, "Could not open codec\n");
		return BL_UNEXPECTED;
	}

	// Fill song properties
	song->filename = malloc(strlen(filename) + 1);
	strcpy(song->filename, filename);

	#if LIBSWRESAMPLE_VERSION_MAJOR < 2
	song->sample_rate = codec_context->sample_rate;
	#else
	song->sample_rate = codecpar->sample_rate;
	#endif
	song->duration = (uint64_t)(context->duration) / ((uint64_t)AV_TIME_BASE);
	song->bitrate = context->bit_rate;
	song->resampled = 0;
	#if LIBSWRESAMPLE_VERSION_MAJOR < 2
	song->nb_bytes_per_sample = av_get_bytes_per_sample(codec_context->sample_fmt);
	song->channels = codec_context->channels;
	#else
	song->nb_bytes_per_sample = av_get_bytes_per_sample(codecpar->format);
	song->channels = codecpar->channels;
	#endif

	// Get number of samples
	size = (
		((uint64_t)(context->duration) * (uint64_t)SAMPLE_RATE) /
		((uint64_t)AV_TIME_BASE)
		) *
		song->channels *
		NB_BYTES_PER_SAMPLE;

	// Estimated number of samples
	song->nSamples = (
		(
		((uint64_t)(context->duration) * (uint64_t)SAMPLE_RATE) /
		((uint64_t)AV_TIME_BASE)
		) *
		song->channels
	);

	// Allocate sample_array
	if((song->sample_array = calloc(size, 1)) == NULL) {
		fprintf(stderr, "Could not allocate enough memory\n");
		return BL_UNEXPECTED;
	}

	beginning = song->sample_array;
	index = 0;

	// If the song is in a floating-point format or int32, prepare the conversion to int16
	#if LIBSWRESAMPLE_VERSION_MAJOR < 2
	if( (codec_context->sample_fmt != AV_SAMPLE_FMT_S16) ||
		(codec_context->sample_rate != SAMPLE_RATE) ) {
	#else
	if( (codecpar->format != AV_SAMPLE_FMT_S16) ||
		(codecpar->sample_rate != SAMPLE_RATE)) {
	#endif
		song->resampled = 1;
		song->nb_bytes_per_sample = 2;
		song->sample_rate = SAMPLE_RATE;
	
		swr_ctx = swr_alloc();

		#if LIBSWRESAMPLE_VERSION_MAJOR < 2
		av_opt_set_int(swr_ctx, "in_channel_layout", codec_context->channel_layout, 0);
		av_opt_set_int(swr_ctx, "in_sample_rate", codec_context->sample_rate, 0);
		av_opt_set_sample_fmt(swr_ctx, "in_sample_fmt", codec_context->sample_fmt, 0);
		av_opt_set_int(swr_ctx, "out_channel_layout", codec_context->channel_layout, 0);
		av_opt_set_int(swr_ctx, "out_sample_rate", song->sample_rate, 0);
		#else
		av_opt_set_int(swr_ctx, "in_channel_layout", codecpar->channel_layout, 0);
		av_opt_set_int(swr_ctx, "in_sample_rate", codecpar->sample_rate, 0);
		av_opt_set_sample_fmt(swr_ctx, "in_sample_fmt", codecpar->format, 0);

		av_opt_set_int(swr_ctx, "out_channel_layout", codecpar->channel_layout, 0);
		av_opt_set_int(swr_ctx, "out_sample_rate", song->sample_rate, 0);
		#endif
		av_opt_set_sample_fmt(swr_ctx, "out_sample_fmt", AV_SAMPLE_FMT_S16, 0);
		if((ret = swr_init(swr_ctx)) < 0) {
			fprintf(stderr, "Could not allocate resampler context\n");
			return BL_UNEXPECTED;
		}
	}

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
	} 
	else {
		song->tracknumber = malloc(1 * sizeof(char));
		strcpy(song->tracknumber, "");
	}

    // Initialize title tag
    tags_dictionary = av_dict_get(context->metadata, "title", NULL, 0);
	if (tags_dictionary!= NULL) {
		song->title = malloc(strlen(tags_dictionary->value) + 1);
		strcpy(song->title, tags_dictionary->value);
	}
	else {
		song->title = malloc(12 * sizeof(char));
		strcpy(song->title, "<no title>");
	}

	// Initialize artist tag
	tags_dictionary = av_dict_get(context->metadata, "ARTIST", NULL, 0);
	if (tags_dictionary!= NULL) {
		song->artist= malloc(strlen(tags_dictionary->value) + 1);
		strcpy(song->artist, tags_dictionary->value);
	}
	else {
		song->artist= malloc(12 * sizeof(char));
		strcpy(song->artist, "<no artist>");
	}

	// Initialize album tag
	tags_dictionary = av_dict_get(context->metadata, "ALBUM", NULL, 0);
	if (tags_dictionary!= NULL) {
		song->album= malloc(strlen(tags_dictionary->value) + 1);
		strcpy(song->album, tags_dictionary->value);
	}
	else {
		song->album= malloc(11 * sizeof(char));
		strcpy(song->album, "<no album>");
	}

	// Initialize genre tag
	tags_dictionary = av_dict_get(context->metadata, "genre", NULL, 0);
	if (tags_dictionary!= NULL) {
		song->genre= malloc(strlen(tags_dictionary->value) + 1);
		strcpy(song->genre, tags_dictionary->value);
	}
	else {
		song->genre = malloc(11 * sizeof(char));
		strcpy(song->genre, "<no genre>");
	}

	// Read the whole data and copy them into a huge buffer
	av_init_packet(&avpkt);
	while(av_read_frame(context, &avpkt) == 0) {
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
			}
			else {
				// Else, unreference it and reset fields
				av_frame_unref(decoded_frame);
			}

			#if LIBSWRESAMPLE_VERSION_MAJOR < 2
			int length = avcodec_decode_audio4(codec_context,
			decoded_frame,
			&got_frame,
			&avpkt);
			if(length < 0) {
			#else
			ret = avcodec_send_packet(codec_context, &avpkt);
			got_frame = !avcodec_receive_frame(codec_context, decoded_frame);
			if(ret < 0) {
			#endif
				avpkt.size = 0;
			}

			av_packet_unref(&avpkt);

			// Copy decoded data into a huge array
			if(got_frame) {
				#if LIBSWRESAMPLE_VERSION_MAJOR < 2
				size_t data_size = av_samples_get_buffer_size(
					NULL,
					song->channels,
					decoded_frame->nb_samples,
					AV_SAMPLE_FMT_S16,
				1);
				#else
				size_t data_size = av_samples_get_buffer_size(
					decoded_frame->linesize,
					song->channels,
					decoded_frame->nb_samples,
					AV_SAMPLE_FMT_S16,
				1);
				#endif

				if((index * song->nb_bytes_per_sample + data_size) > size) {
					int8_t *ptr;
					ptr = realloc(beginning, size + data_size);
					if(ptr != NULL) {
						beginning = ptr;
						size += data_size;
						song->nSamples += data_size / song->nb_bytes_per_sample;
					}
					else
						break;
				
				}

				// If the song isn't in a 16-bit format, convert it to
				if(song->resampled == 1) {
					uint8_t **out_buffer;
					size_t dst_bufsize;
					// Approximate the resampled buffer size 
        			int dst_nb_samples = av_rescale_rnd(swr_get_delay(swr_ctx, codecpar->sample_rate) +
						decoded_frame->nb_samples, SAMPLE_RATE, codecpar->sample_rate, AV_ROUND_UP);
					dst_bufsize = av_samples_alloc_array_and_samples(&out_buffer, decoded_frame->linesize,
						song->channels, dst_nb_samples, AV_SAMPLE_FMT_S16, 1);
					ret = swr_convert(swr_ctx, out_buffer, dst_bufsize,
						(const uint8_t**)decoded_frame->extended_data, decoded_frame->nb_samples);
					if(ret < 0) {
						fprintf(stderr, "Error while converting from floating-point to int\n");
						return BL_UNEXPECTED;
					}
					if(ret != 0) {
						// Get the real resampled buffer size
						dst_bufsize = av_samples_get_buffer_size(NULL, song->channels,
							ret, AV_SAMPLE_FMT_S16, 1);
						memcpy((index * song->nb_bytes_per_sample) + beginning,
							out_buffer[0], dst_bufsize);
						index += dst_bufsize / (float)song->nb_bytes_per_sample;
					}
					av_freep(&out_buffer[0]);
					free(out_buffer);
				}
				else {
					memcpy((index * song->nb_bytes_per_sample) + beginning,
						decoded_frame->extended_data[0],
						data_size);
					index += data_size / song->nb_bytes_per_sample;
				}
			}
		}
		else {
			// Dropping packets that do not belong to the audio stream
			// (such as album cover)
			av_packet_unref(&avpkt);
		}
	}
	song->sample_array = beginning;

	// Free memory
	avpkt.data = NULL;
	avpkt.size = 0;

	// Use correct number of samples after decoding
	if((song->nSamples = index) <= 0) {
		fprintf(stderr, "Couldn't find any valid samples while decoding\n");
		return BL_UNEXPECTED;
	}
	
	// Read the end of audio, as precognized in http://ffmpeg.org/pipermail/libav-user/2015-August/008433.html
	do {
		#if LIBSWRESAMPLE_VERSION_MAJOR < 2
		avcodec_decode_audio4(codec_context, decoded_frame, &got_frame, &avpkt);
	} while(got_frame);
		#else
		ret = avcodec_send_packet(codec_context, &avpkt);
		avcodec_receive_frame(codec_context, decoded_frame);
	} while(ret != AVERROR_EOF);
		#endif

//	FILE *coucou = fopen("pls", "w");
//	fwrite(song->sample_array, size, 1, coucou);

	// Free memory
	if(song->resampled)
		swr_free(&swr_ctx);
	#if LIBSWRESAMPLE_VERSION_MAJOR < 2
	avcodec_close(codec_context);
	#else
	avcodec_free_context(&codec_context);
	#endif
	av_frame_unref(decoded_frame);
	# if LIBAVUTIL_VERSION_MAJOR > 51
	av_frame_free(&decoded_frame);
	#endif
	av_packet_unref(&avpkt);
	avformat_close_input(&context);

	return BL_OK;
}
