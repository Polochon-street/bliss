from bliss._bliss import ffi, lib
from bliss.bl_song import bl_song


def distance(filename1, filename2):
    """
    Wrapper around `bl_distance` function.

    Params:
        - filename1 is the first file to use.
        - filename2 is the second file to use.

    Returns a dict {distance, song1, song2} containing the computed distance
    and the created `bl_song` objects.
    """
    song1 = ffi.new("struct bl_song *")
    song2 = ffi.new("struct bl_song *")
    filename1 = ffi.new("char[]", filename1.encode("utf-8"))
    filename2 = ffi.new("char[]", filename2.encode("utf-8"))
    return {
        "distance": lib.bl_distance(filename1, filename2, song1, song2),
        "song1": bl_song(c_struct=song1),
        "song2": bl_song(c_struct=song2)
    }


def cosine_similarity(filename1, filename2):
    """
    Wrapper around `bl_cosine_similarity` function.

    Params:
        - filename1 is the first file to use.
        - filename2 is the second file to use.

    Returns a dict {similarity, song1, song2} containing the computed cosine
    similarity and the created `bl_song` objects.
    """
    song1 = ffi.new("struct bl_song *")
    song2 = ffi.new("struct bl_song *")
    filename1 = ffi.new("char[]", filename1.encode("utf-8"))
    filename2 = ffi.new("char[]", filename2.encode("utf-8"))
    return {
        "similarity": lib.bl_cosine_similarity(filename1, filename2,
                                               song1, song2),
        "song1": bl_song(c_struct=song1),
        "song2": bl_song(c_struct=song2)
    }
