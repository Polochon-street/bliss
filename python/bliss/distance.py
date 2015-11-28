from bliss._bliss import ffi, lib
from bliss.bl_song import bl_song


def distance(song1, song2):
    """
    Wrapper around `bl_distance_file` function.

    Params:
        - filename1 is the first file to use.
        - filename2 is the second file to use.

    Returns a dict {distance, song1, song2} containing the computed distance
    and the created `bl_song` objects.
    """
    if isinstance(song1, str) and isinstance(song2, str):
        filename1 = ffi.new("char[]", song1.encode("utf-8"))
        filename2 = ffi.new("char[]", song2.encode("utf-8"))
        song1 = ffi.new("struct bl_song *")
        song2 = ffi.new("struct bl_song *")
        return {
            "distance": lib.bl_distance_file(filename1, filename2,
                                             song1, song2),
            "song1": bl_song(c_struct=song1),
            "song2": bl_song(c_struct=song2)
        }
    elif isinstance(song1, bl_song) and isinstance(song2, bl_song):
        return {
            "distance": lib.bl_distance(song1["force_vector"],
                                          song2["force_vector"]),
            "song1": song1,
            "song2": song2
        }
    else:
        return {
            "distance": None,
            "song1": None,
            "song2": None
        }


def cosine_similarity(song1, song2):
    """
    Wrapper around `bl_cosine_similarity` function.

    Params:
        - filename1 is the first file to use.
        - filename2 is the second file to use.

    Returns a dict {similarity, song1, song2} containing the computed cosine
    similarity and the created `bl_song` objects.
    """
    if isinstance(song1, str) and isinstance(song2, str):
        filename1 = ffi.new("char[]", song1.encode("utf-8"))
        filename2 = ffi.new("char[]", song2.encode("utf-8"))
        song1 = ffi.new("struct bl_song *")
        song2 = ffi.new("struct bl_song *")
        return {
            "similarity": lib.bl_cosine_similarity_file(filename1, filename2,
                                                        song1, song2),
            "song1": bl_song(c_struct=song1),
            "song2": bl_song(c_struct=song2)
        }
    elif isinstance(song1, bl_song) and isinstance(song2, bl_song):
        return {
            "similarity": lib.bl_cosine_similarity(song1["force_vector"],
                                                   song2["force_vector"]),
            "song1": song1,
            "song2": song2
        }
    else:
        return {
            "similarity": None,
            "song1": None,
            "song2": None
        }
