import unittest
from bliss import BL_LOUD, bl_song


class TestFetcher(unittest.TestCase):
    def test_loud(self):
        with bl_song("/tmp/test.mp3") as song:
            self.assertAlmostEqual(song["force"], 11.403019)

            # self.assertEqual(song.force_vector.tempo, 2.517007)
            # self.assertEqual(song.force_vector.amplitude, 0.107364)
            # self.assertEqual(song.force_vector.frequency, -1.432200)
            # self.assertEqual(song.force_vector.attack, 10.210849)

            self.assertEqual(song["channels"], 2)

            self.assertEqual(song["nSamples"], 25021440)

            self.assertEqual(song["sample_rate"], 44100)

            self.assertEqual(song["bitrate"], 198332)

            self.assertEqual(song["nb_bytes_per_sample"], 2)

            self.assertEqual(song["calm_or_loud"], BL_LOUD)

            self.assertEqual(song["duration"], 283)

            self.assertEqual(song["artist"], "David TMX")

            self.assertEqual(song["title"], "Lost in dreams")

            self.assertEqual(song["album"], "Renaissance")

            self.assertEqual(song["tracknumber"], "14")

            self.assertEqual(song["genre"], "(255)")
