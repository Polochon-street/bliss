from bliss._bliss import ffi, lib
import collections.abc


class bl_song(collections.abc.Mapping):
    """
    Wrapper to ease manipulation of the `bl_song` C struct.

    `bl_song` class exposes a dict-like interface to update the fields of the
    underlying (private) `bl_song` C struct.
    """
    def __init__(self, filename=None, initializer=None, c_struct=None):
        """
        Initialize a new `bl_song` object.

        Params:
            - filename is a path to a file to load and analyze (optional).
            - initializer is an initializer to feed to `ffi.new` allocation
            call. Valid initializers are a list or tuple or dict of the field
            values.
            - c_struct is a preexisting `struct bl_song *` to use for
            initialization.
        """
        # Initializing private data members
        if c_struct is not None:
            self._c_struct = c_struct
        else:
            self._c_struct = ffi.new("struct bl_song *", initializer)
        self._types = {i[0]: i[1].type
                       for i in ffi.typeof("struct bl_song").fields}
        # _keepalive is useful to prevent garbage
        # collector from collecting dynamically allocated
        # data members
        self._keepalive = {}

        if filename is not None:
            self.analyze(filename)

    def __getitem__(self, key):
        """
        Implementation of dict-like access to the fields.
        """
        return self.get(key)

    def __setitem__(self, key, value):
        """
        Implementation of dict-like update of the fields.
        """
        return self.set(key, value)

    def __len__(self):
        """
        For len() method.
        """
        return len(self._types)

    def __iter__(self):
        """
        Implement dict-like iteration on data members.
        """
        return self._types.__iter__()

    def __enter__(self):
        """
        Usable in a with context.
        """
        return self

    def __exit__(self, exc_type, exc_val, exc_tb):
        """
        Free dynamically allocated memory at the end of a with statement.
        """
        self.free()

    def __repr__(self):
        """
        String representation.
        """
        value_dict = {k: self.get(k) for k in self._types}
        return value_dict.__repr__()

    def get(self, key):
        """
        Get a data member with a conversion to Python type as much as possible.

        Params:
            - key is the id of the data member to get.

        Returns the value with a Python type.
        """
        value = getattr(self._c_struct, key)
        # Ease manipulation of char* data members, convert to python str()
        if self._types[key] == ffi.typeof("char *") and value != ffi.NULL:
            return ffi.string(value).decode("utf-8")
        # Ease manipulation of force_vector_s fields
        elif self._types[key] == ffi.typeof("struct force_vector_s"):
            return {
                "tempo": value.tempo,
                "amplitude": value.amplitude,
                "frequency": value.frequency,
                "attack": value.attack
            }
        # Same for array of int fields
        elif self._types[key] == ffi.typeof("int8_t *"):
            return [value[i] for i in range(self.get("nSamples"))]
        # Else, returning the value directly should be safe
        else:
            return value

    def set(self, key, value):
        """
        Set a data member with a conversion from Python type as much as
        possible.

        Params:
            - key is the id of the data member to set.
            - value is the value to set.
        """
        # Ease manipulation of char* data members, convert python str()
        if self._types[key] == ffi.typeof("char *") and value != ffi.NULL:
            value = ffi.new("char[]", value.encode("utf-8"))
            # Keep the value in a dict in this object to keep it alive and safe
            # from the garbage collector.
            self._keepalive[key] = value
        # Same for force_vector_s fields
        elif self._types[key] == ffi.typeof("struct force_vector_s"):
            # Initialization from a valid initializer (tuple, list or dict)
            value = ffi.new("struct force_vector_s", value)
        # Same for array of int fields
        elif self._types[key] == ffi.typeof("int8_t *"):
            # TODO: Segfault
            value = ffi.new("int8_t[]", value)
            # Keep the value in a dict in this object to keep it alive and safe
            # from the garbage collector.
            self._keepalive[key] = value
        else:
            # Nothing to do
            pass
        return setattr(self._c_struct, key, value)

    def decode(self, filename):
        """
        Decode an audio file and load it into this `bl_song` object. Do not run
        any analysis on it.

        Params:
            - filename is the path to the file to load.
        """
        filename_char = ffi.new("char[]", filename.encode("utf-8"))
        lib.bl_audio_decode(filename_char, self._c_struct)

    def analyze(self, filename):
        """
        Load and analyze an audio file, putting it in a `bl_song` object.

        Params:
            - filename is the path to the file to load and analyze.
        """
        filename_char = ffi.new("char[]", filename.encode("utf-8"))
        lib.bl_analyze(filename_char, self._c_struct)

    def envelope_analysis(self):
        """
        Run an envelope analysis on a previously loaded file.

        Returns a {tempo, attack} dict, which is a direct mapping of
        `struct envelope_result_s`. Also updates the object data members.
        """
        result = ffi.new("struct envelope_result_s *")
        lib.bl_envelope_sort(self._c_struct, result)
        return {
            "tempo": result.tempo,
            "attack": result.attack
        }

    def amplitude_analysis(self):
        """
        Run an amplitude analysis on a previously loaded file.

        Returns a the score obtained. Also updates the object data members.
        """
        lib.bl_amplitude_sort(self._c_struct)

    def frequency_analysis(self):
        """
        Run a frequency analysis on a previously loaded file.

        Returns a the score obtained. Also updates the object data members.
        """
        lib.bl_frequency_sort(self._c_struct)

    def free(self):
        """
        Free dynamically allocated data in the underlying C struct (artist,
        genre, etc). Must be called at deletion to prevent memory leaks.
        """
        lib.bl_free_song(self._c_struct)


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
