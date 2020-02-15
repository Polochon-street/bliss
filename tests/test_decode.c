#include "bliss.h"

#include <math.h>

void assert_eq(int a, int b) {
  if (a != b) {
    printf("%d %d\n", a, b);
    exit(-1);
  }
}
// FIXME DRY this + test with 48kHz freq
void test_decode_s16(void) {
  struct bl_song song;
  uint8_t hash[16];
  // Obtained by doing `ffmpeg -i song.flac -f hash -hash md5 out.md5`
  uint8_t expected_hash[] = {0x8a, 0x1b, 0xd8, 0x24, 0x95, 0x1c, 0x04, 0x33,
                             0xcc, 0x47, 0xfe, 0xc5, 0xbf, 0x41, 0xd0, 0xa9};

  bl_analyze("../audio/song.flac", &song);
  av_md5_sum(hash, (uint8_t *)song.sample_array,
             song.nSamples * song.nb_bytes_per_sample);

  for (int i = 0; i < 16; ++i) {
    assert_eq(hash[i], expected_hash[i]);
  }
  bl_free_song(&song);
}

void test_decode_s32(void) {
  struct bl_song song;
  uint8_t hash[16];
  // Obtained by doing `ffmpeg -f s16le -ar 22050 -acodec pcm_s16le -i
  // out_s32.raw -f hash -hash md5 out.md5` `out_s32.raw` was obtained by doing
  // `ffmpeg -i song_s32.flac -ar 22050 -f s16le -acodec pcm_s16le out_s32.raw`
  uint8_t expected_hash[] = {0xeb, 0x9f, 0x31, 0xa7, 0xb9, 0xed, 0x02, 0x2d,
                             0x66, 0xff, 0x82, 0xb7, 0x6e, 0x7c, 0x3c, 0x18};

  bl_analyze("../audio/song_s32.flac", &song);
  av_md5_sum(hash, (uint8_t *)song.sample_array,
             song.nSamples * song.nb_bytes_per_sample);

  for (int i = 0; i < 16; ++i) {
    assert_eq(hash[i], expected_hash[i]);
  }
  bl_free_song(&song);
}

void test_decode_s32_mono(void) {
  struct bl_song song;
  uint8_t hash[16];
  // Obtained by doing `ffmpeg -f s16le -ar 22050 -acodec pcm_s16le -i
  // out_s32_mono.raw -f hash -hash md5 out.md5` `out_s32_mono.raw` was obtained
  // by doing `ffmpeg -i song_s32_mono.flac -ar 22050 -f s16le -acodec pcm_s16le
  // out_s32_mono.raw`
  uint8_t expected_hash[] = {0x74, 0x7d, 0xbf, 0xcd, 0x75, 0xbe, 0xbc, 0x23,
                             0xeb, 0xe2, 0x02, 0x49, 0x35, 0xae, 0xde, 0x36};

  bl_analyze("../audio/song_s32_mono.flac", &song);
  av_md5_sum(hash, (uint8_t *)song.sample_array,
             song.nSamples * song.nb_bytes_per_sample);

  for (int i = 0; i < 16; ++i) {
    assert_eq(hash[i], expected_hash[i]);
  }
  bl_free_song(&song);
}

int main(void) {
  test_decode_s16();
  test_decode_s32();
  test_decode_s32_mono();
}
