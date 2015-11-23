from _bliss import ffi, lib
import ctypes


class bl_song:
    """
    Wrapper to ease manipulation of the C `bl_song` struct.
    """
    def __init__(self):
        self._c_struct = ffi.new("struct bl_song *")
        self._types = {i[0]: i[1].type
                       for i in ffi.typeof("struct bl_song").fields}
        self._keepalive = {}

    def get(self, item):
        value = getattr(self._c_struct, item)
        # Ease manipulation of char* data members
        if self._types[item] == ffi.typeof("char *"):
            return ffi.string(value).decode("utf-8")
        elif self._types[item] == ffi.typeof("struct force_vector_s"):
            # TODO
            return
        else:
            return value

    def set(self, item, value):
        # Ease manipulation of char* data members
        if self._types[item] == ffi.typeof("char *"):
            value = ffi.new("char[]", value.encode("utf-8"))
            # Keep the value in a dict in this object to keep it alive and safe
            # from the garbage collector.
            self._keepalive[item] = value
            return setattr(self._c_struct, item, value)
        elif self._types[item] == ffi.typeof("struct force_vector_s"):
            # TODO
            return
        else:
            return setattr(self._c_struct, item, value)


song = bl_song()
song.set("artist", "foobar")
song.set("force", 1)
print(bl_song)
print(song.get("artist"))
print(str(song.get("force")))
