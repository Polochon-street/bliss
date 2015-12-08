#!/use/bin/env python
from bliss import bl_song, distance

# Some examples of distance computation
if __name__ == "__main__":
    # Compute directly with filenames
    distance_out = distance.distance("/tmp/test.mp3", "/tmp/test.mp3")
    print(distance_out["distance"])
    # Always free the bl_song struct when done!
    distance_out["song1"].free()
    distance_out["song2"].free()

    distance_out = distance.cosine_similarity("/tmp/test.mp3", "/tmp/test.mp3")
    print(distance_out["similarity"])
    # Always free the bl_song struct when done!
    distance_out["song1"].free()
    distance_out["song2"].free()

    # Compute using a previously loaded bl_song
    with bl_song("/tmp/test.mp3") as song1:
        print(distance.distance(song1, song1)["distance"])
        print(distance.cosine_similarity(song1,
                                         song1)["similarity"])
