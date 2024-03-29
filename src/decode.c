#include <libavcodec/avcodec.h>
#include <libavutil/opt.h>
#include <libswresample/swresample.h>

#include "bliss.h"

#define NB_BYTES_PER_SAMPLE 2
#define SAMPLE_RATE 22050
#define CHANNELS 2

int fill_song_properties(struct bl_song *const song, char const *const filename,
                         AVCodecParameters *codecpar, AVFormatContext *context,
                         struct SwrContext **swr_ctx);

int process_frame(struct bl_song *const song, int8_t **beginning_ptr,
                  AVFrame *decoded_frame, int *index_ptr, uint64_t *size_ptr,
                  struct SwrContext *swr_ctx);

int resample_decoded_frames(struct SwrContext *swr_ctx,
                            struct bl_song *const song, AVFrame *decoded_frame,
                            uint8_t ***out_buffer, int flush);

int append_buffer_to_song(struct bl_song *const song, int *index_ptr,
                          int nb_samples, int8_t **beginning_ptr,
                          uint64_t *size_ptr, uint8_t *decoded_samples);

int bl_audio_decode(char const *const filename, struct bl_song *const song) {
  int ret;
  // Contexts and libav variables
  AVPacket avpkt;
  AVFormatContext *context;
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

  // Pointer to beginning of music data
  int8_t *beginning;
  // Received frame holder
  int got_frame;
  // Position in the data buffer
  int index;
  context = avformat_alloc_context();

  av_log_set_level(AV_LOG_QUIET);

  // Open input file
  if (avformat_open_input(&context, filename, NULL, NULL) < 0) {
    fprintf(stderr, "Couldn't open file: %s. Error %d encountered.\n", filename,
            errno);
    return BL_UNEXPECTED;
  }

  // Search for a valid stream
  if (avformat_find_stream_info(context, NULL) < 0) {
    fprintf(stderr, "Couldn't find stream information\n");
    avformat_close_input(&context);
    return BL_UNEXPECTED;
  }

  // Find stream and corresponding codec
  audio_stream =
      av_find_best_stream(context, AVMEDIA_TYPE_AUDIO, -1, -1, &codec, 0);
  if (audio_stream < 0) {
    fprintf(stderr, "Couldn't find a suitable audio stream\n");
    avformat_close_input(&context);
    return BL_UNEXPECTED;
  }

#if LIBSWRESAMPLE_VERSION_MAJOR < 2
  codec_context = context->streams[audio_stream]->codec;
  if (!codec_context) {
    fprintf(stderr, "Codec not found!\n");
    avformat_close_input(&context);
    return BL_UNEXPECTED;
  }
#else
  // Find codec parameters
  codecpar = context->streams[audio_stream]->codecpar;

  // Find and allocate codec context
  codec_context = avcodec_alloc_context3(codec);
#endif
  codec_context->thread_count = 0;
  codec_context->thread_type = FF_THREAD_FRAME;

  if (avcodec_open2(codec_context, codec, NULL) < 0) {
    fprintf(stderr, "Could not open codec\n");
    return BL_UNEXPECTED;
    avformat_close_input(&context);
  }

  // Fill song properties
  if ((ret = fill_song_properties(song, filename, codecpar, context,
                                  &swr_ctx)) == BL_UNEXPECTED) {
    goto cleanup;
  }
  beginning = song->sample_array;
  index = 0;

  // Read the whole data and copy them into a huge buffer
  av_init_packet(&avpkt);
  decoded_frame = av_frame_alloc();
  if (!decoded_frame) {
    fprintf(stderr, "Could not allocate audio frame\n");
    ret = BL_UNEXPECTED;
    goto cleanup;
  }
  while (av_read_frame(context, &avpkt) == 0) {
    if (avpkt.stream_index == audio_stream) {
#if LIBSWRESAMPLE_VERSION_MAJOR < 2
      avcodec_decode_audio4(codec_context, decoded_frame, &got_frame, &avpkt);
#else
      avcodec_send_packet(codec_context, &avpkt);
      got_frame = !avcodec_receive_frame(codec_context, decoded_frame);
#endif

      av_packet_unref(&avpkt);

      // Copy decoded data into a huge array
      if (got_frame) {
        if ((ret = process_frame(song, &beginning, decoded_frame, &index, &size,
                                 swr_ctx)) == BL_UNEXPECTED) {
          goto cleanup;
        }
      }
    } else {
      // Dropping packets that do not belong to the audio stream
      // (such as album cover)
      av_packet_unref(&avpkt);
    }
  }
  // Free memory
  avpkt.data = NULL;
  avpkt.size = 0;

  // Read the end of audio, as precognized in
  // http://ffmpeg.org/pipermail/libav-user/2015-August/008433.html
#if LIBSWRESAMPLE_VERSION_MAJOR < 2
  do {
    avcodec_decode_audio4(codec_context, decoded_frame, &got_frame, &avpkt);
    if (got_frame) {
      if ((ret = process_frame(song, &beginning, decoded_frame, &index, &size,
                               swr_ctx)) == BL_UNEXPECTED) {
        goto cleanup;
      }
    }
  } while (got_frame);
#else
  avcodec_send_packet(codec_context, NULL);
  do {
    ret = avcodec_receive_frame(codec_context, decoded_frame);
    if (!ret) {
      if (process_frame(song, &beginning, decoded_frame, &index, &size,
                        swr_ctx) == BL_UNEXPECTED) {
        ret = BL_UNEXPECTED;
        goto cleanup;
      }
    }
  } while (!ret);
#endif
  if (song->resampled == 1) {
    uint8_t **out_buffer;
    if ((ret = resample_decoded_frames(swr_ctx, song, decoded_frame,
                                       &out_buffer, 1)) == BL_UNEXPECTED) {
      return BL_UNEXPECTED;
    }
    if (ret) {
      if (append_buffer_to_song(song, &index, ret, &beginning, &size,
                                out_buffer[0]) == BL_UNEXPECTED) {
        return BL_UNEXPECTED;
      }
    }
    if (out_buffer)
      av_freep(&out_buffer[0]);
    av_freep(&out_buffer);
  }

  // Use correct number of samples after decoding
  if ((song->nSamples = index) <= 0) {
    fprintf(stderr, "Couldn't find any valid samples while decoding\n");
    return BL_UNEXPECTED;
  }
  song->sample_array = beginning;
  song->sample_rate = SAMPLE_RATE;
  song->channels = CHANNELS;

  ret = BL_OK;
cleanup:
  // Free memory
  if (song->resampled)
    swr_free(&swr_ctx);
#if LIBSWRESAMPLE_VERSION_MAJOR < 2
  avcodec_close(codec_context);
#else
  avcodec_free_context(&codec_context);
#endif
  av_frame_unref(decoded_frame);
#if LIBAVUTIL_VERSION_MAJOR > 51
  av_frame_free(&decoded_frame);
#endif
  av_packet_unref(&avpkt);
  avformat_close_input(&context);

  return ret;
}

int fill_song_properties(struct bl_song *const song, char const *const filename,
                         AVCodecParameters *codecpar, AVFormatContext *context,
                         struct SwrContext **swr_ctx) {
  // Dictionary to fetch tags
  AVDictionaryEntry *tags_dictionary;
  uint64_t size = 0;

  song->filename = malloc(strlen(filename) + 1);
  strcpy(song->filename, filename);

#if LIBSWRESAMPLE_VERSION_MAJOR < 2
  song->sample_rate = codec_context->sample_rate;
  song->nb_bytes_per_sample =
      av_get_bytes_per_sample(codec_context->sample_fmt);
  song->channels = codec_context->channels;
#else
  song->sample_rate = codecpar->sample_rate;
  song->nb_bytes_per_sample = av_get_bytes_per_sample(codecpar->format);
  song->channels = codecpar->channels;
#endif
  song->duration = (uint64_t)(context->duration) / ((uint64_t)AV_TIME_BASE);
  song->bitrate = context->bit_rate;
  song->resampled = 0;

  // Get number of samples
  size = (((uint64_t)(context->duration) * (uint64_t)SAMPLE_RATE) /
          ((uint64_t)AV_TIME_BASE)) *
         song->channels * NB_BYTES_PER_SAMPLE;

  // Estimated number of samples
  song->nSamples = ((((uint64_t)(context->duration) * (uint64_t)SAMPLE_RATE) /
                     ((uint64_t)AV_TIME_BASE)) *
                    song->channels);

  // Allocate sample_array
  if ((song->sample_array = calloc(size, 1)) == NULL) {
    fprintf(stderr, "Could not allocate enough memory\n");
    return BL_UNEXPECTED;
  }
  // Zero initialize tags
  song->artist = NULL;
  song->title = NULL;
  song->album = NULL;
  song->tracknumber = NULL;

  // Initialize tracknumber tag
  tags_dictionary = av_dict_get(context->metadata, "track", NULL, 0);
  if (tags_dictionary != NULL) {
    song->tracknumber = malloc(strlen(tags_dictionary->value) + 1);
    strcpy(song->tracknumber, tags_dictionary->value);
    song->tracknumber[strcspn(song->tracknumber, "/")] = '\0';
  } else {
    song->tracknumber = malloc(1 * sizeof(char));
    strcpy(song->tracknumber, "");
  }

  // Initialize title tag
  tags_dictionary = av_dict_get(context->metadata, "title", NULL, 0);
  if (tags_dictionary != NULL) {
    song->title = malloc(strlen(tags_dictionary->value) + 1);
    strcpy(song->title, tags_dictionary->value);
  } else {
    song->title = malloc(12 * sizeof(char));
    strcpy(song->title, "<no title>");
  }

  // Initialize artist tag
  tags_dictionary = av_dict_get(context->metadata, "ARTIST", NULL, 0);
  if (tags_dictionary != NULL) {
    song->artist = malloc(strlen(tags_dictionary->value) + 1);
    strcpy(song->artist, tags_dictionary->value);
  } else {
    song->artist = malloc(12 * sizeof(char));
    strcpy(song->artist, "<no artist>");
  }

  // Initialize album tag
  tags_dictionary = av_dict_get(context->metadata, "ALBUM", NULL, 0);
  if (tags_dictionary != NULL) {
    song->album = malloc(strlen(tags_dictionary->value) + 1);
    strcpy(song->album, tags_dictionary->value);
  } else {
    song->album = malloc(11 * sizeof(char));
    strcpy(song->album, "<no album>");
  }

  // Initialize genre tag
  tags_dictionary = av_dict_get(context->metadata, "genre", NULL, 0);
  if (tags_dictionary != NULL) {
    song->genre = malloc(strlen(tags_dictionary->value) + 1);
    strcpy(song->genre, tags_dictionary->value);
  } else {
    song->genre = malloc(11 * sizeof(char));
    strcpy(song->genre, "<no genre>");
  }

  // If the song is in a floating-point format or int32, prepare the conversion
  // to int16
#if LIBSWRESAMPLE_VERSION_MAJOR < 2
  if ((codec_context->sample_fmt != AV_SAMPLE_FMT_S16) ||
      (codec_context->sample_rate != SAMPLE_RATE)) {
#else
  if ((codecpar->format != AV_SAMPLE_FMT_S16) ||
      (codecpar->sample_rate != SAMPLE_RATE)) {
#endif
    song->resampled = 1;
    song->nb_bytes_per_sample = 2;

    *swr_ctx = swr_alloc();

#if LIBSWRESAMPLE_VERSION_MAJOR < 2
    av_opt_set_int(*swr_ctx, "in_channel_layout", codec_context->channel_layout,
                   0);
    av_opt_set_int(*swr_ctx, "in_sample_rate", codec_context->sample_rate, 0);
    av_opt_set_sample_fmt(*swr_ctx, "in_sample_fmt", codec_context->sample_fmt,
                          0);
    av_opt_set_int(*swr_ctx, "out_channel_layout",
                   codec_context->channel_layout, 0);
    av_opt_set_int(*swr_ctx, "out_sample_rate", SAMPLE_RATE, 0);
#else
    av_opt_set_int(*swr_ctx, "in_channel_layout", codecpar->channel_layout, 0);
    av_opt_set_int(*swr_ctx, "in_sample_rate", codecpar->sample_rate, 0);
    av_opt_set_sample_fmt(*swr_ctx, "in_sample_fmt", codecpar->format, 0);
    av_opt_set_int(*swr_ctx, "out_channel_layout", AV_CH_LAYOUT_STEREO, 0);
    av_opt_set_int(*swr_ctx, "out_sample_rate", SAMPLE_RATE, 0);
#endif
    av_opt_set_sample_fmt(*swr_ctx, "out_sample_fmt", AV_SAMPLE_FMT_S16, 0);
    if (swr_init(*swr_ctx) < 0) {
      fprintf(stderr, "Could not allocate resampler context\n");
      return BL_UNEXPECTED;
    }
  }

  return BL_OK;
}

// If needed, realloc sample array and put stuff in beginning_ptr
int append_buffer_to_song(struct bl_song *const song, int *index_ptr,
                          int nb_samples, int8_t **beginning_ptr,
                          uint64_t *size_ptr, uint8_t *decoded_samples) {
  size_t data_size = av_samples_get_buffer_size(NULL, CHANNELS, nb_samples,
                                                AV_SAMPLE_FMT_S16, 1);
  if ((*index_ptr * song->nb_bytes_per_sample + data_size) > *size_ptr) {
    int8_t *ptr;
    ptr = realloc(*beginning_ptr, *size_ptr + data_size);
    if (ptr != NULL) {
      *beginning_ptr = ptr;
      *size_ptr += data_size;
      song->nSamples += data_size / song->nb_bytes_per_sample;
    } else {
      fprintf(stderr, "Error while trying to allocate memory\n");
      return BL_UNEXPECTED;
    }
  }
  memcpy(&(*beginning_ptr)[*index_ptr * song->nb_bytes_per_sample],
         decoded_samples, data_size);
  *index_ptr += data_size / song->nb_bytes_per_sample;

  return BL_OK;
}

int resample_decoded_frames(struct SwrContext *swr_ctx,
                            struct bl_song *const song, AVFrame *decoded_frame,
                            uint8_t ***out_buffer, int flush) {
  size_t dst_bufsize;
  int nb_samples;
  // Approximate the resampled buffer size
  int dst_nb_samples = av_rescale_rnd(
      swr_get_delay(swr_ctx, song->sample_rate) + decoded_frame->nb_samples,
      SAMPLE_RATE, song->sample_rate, AV_ROUND_UP);
  dst_bufsize = av_samples_alloc_array_and_samples(
      out_buffer, NULL, CHANNELS, dst_nb_samples, AV_SAMPLE_FMT_S16, 0);
  if (!flush) {
    nb_samples = swr_convert(swr_ctx, *out_buffer, dst_bufsize,
                             (const uint8_t **)decoded_frame->data,
                             decoded_frame->nb_samples);
  } else {
    nb_samples = swr_convert(swr_ctx, *out_buffer, dst_bufsize, NULL, 0);
  }
  if (nb_samples < 0) {
    fprintf(stderr, "Error while converting from floating-point to int\n");
    return BL_UNEXPECTED;
  }

  return nb_samples;
}

int process_frame(struct bl_song *const song, int8_t **beginning_ptr,
                  AVFrame *decoded_frame, int *index_ptr, uint64_t *size_ptr,
                  struct SwrContext *swr_ctx) {
  uint8_t *decoded_samples = decoded_frame->extended_data[0];
  int nb_samples = decoded_frame->nb_samples;
  uint8_t **out_buffer;
  // If the song isn't in a 16-bit format, convert it to
  if (song->resampled == 1) {
    if ((nb_samples = resample_decoded_frames(
             swr_ctx, song, decoded_frame, &out_buffer, 0)) == BL_UNEXPECTED) {
      return BL_UNEXPECTED;
    }
    decoded_samples = out_buffer[0];
  }
  if (nb_samples > 0)
    if (append_buffer_to_song(song, index_ptr, nb_samples, beginning_ptr,
                              size_ptr, decoded_samples) == BL_UNEXPECTED)
      return BL_UNEXPECTED;

  if (song->resampled == 1) {
    if (out_buffer)
      av_freep(&out_buffer[0]);
    av_freep(&out_buffer);
  }
  return BL_OK;
}
