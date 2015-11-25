#!/usr/bin/env python
from bliss import bl_song

if __name__ == "__main__":
    import json

    # Some testing and debug code
    song = bl_song()
    song.set("artist", "foobar")
    song.set("force", 1)
    print(bl_song)
    print(song.get("artist"))
    print(str(song.get("force")))
    print(song["artist"])
    song["artist"] = "foo"
    print(song["artist"])

    song = bl_song("/tmp/test.mp3")
    print(song["genre"])
    song.free()

    for key in song:
        print(key)

    with bl_song("/tmp/test.mp3") as song:
        print(song["artist"])
        print(song["force_vector"])
        song["sample_array"] = []
        song["nSamples"] = 0  # Do *not* forget to update number of samples
        song["force_vector"] = {"tempo": 1., "attack": 2., "amplitude": 3.,
                                "frequency": 4.}
        print(dict(song))
        print(json.dumps(dict(song)))
