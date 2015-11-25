#!/use/bin/env python
from bliss import distance

if __name__ == "__main__":
    print(distance.distance("/tmp/test.mp3", "/tmp/test.mp3")["distance"])
    print(distance.cosine_similarity("/tmp/test.mp3",
                                     "/tmp/test.mp3")["similarity"])
