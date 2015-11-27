#!/use/bin/env python
from bliss import bl_song, distance

if __name__ == "__main__":
    print(distance.distance("/tmp/test.mp3", "/tmp/test.mp3")["distance"])
    print(distance.cosine_similarity("/tmp/test.mp3",
                                     "/tmp/test.mp3")["similarity"])

    song1 = bl_song("/tmp/test.mp3")
    print(distance.distance(song1, song1)["distance"])
    print(distance.cosine_similarity(song1,
                                     song1)["similarity"])
