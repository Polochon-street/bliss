import sys

from cffi import FFI
from buildtools import pkgconfig

ffi = FFI()

# Check which resample lib to use
if pkgconfig.exists("libswresample"):
    resample = ("../src/decode.c", "swresample")
elif pkgconfig.exists("libavresample"):
    resample = ("../src/decode_av.c", "avresample")
else:
    sys.exit("No libswresample/libavresample available.")

# Build
ffi.set_source("bliss._bliss",
               "#include \"bliss.h\"",
               sources=["../src/amplitude_sort.c",
                        "../src/analyze.c",
                        resample[0],
                        "../src/envelope_sort.c",
                        "../src/frequency_sort.c",
                        "../src/helpers.c"],
               libraries=["avformat", "avutil", "avcodec", resample[1]],
               include_dirs=["/usr/include/ffmpeg/", "../include/"])

header = '\n'.join([i for i in open("../include/bliss.h", 'r').readlines()
                    if not i.strip().startswith("#")])
ffi.cdef(header)

if __name__ == "__main__":
    ffi.compile()
