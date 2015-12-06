import os
import sys

from cffi import FFI
from buildtools import pkgconfig

abspath = os.path.abspath(__file__)
dname = os.path.dirname(abspath)

ffi = FFI()

# Check which resample lib to use
if pkgconfig.exists("libswresample"):
    resample = (dname + "/../src/decode.c", "swresample")
elif pkgconfig.exists("libavresample"):
    resample = (dname + "/../src/decode_av.c", "avresample")
else:
    sys.exit("No libswresample/libavresample available.")

# Build
ffi.set_source("bliss._bliss",
               "#include \"bliss.h\"",
               sources=[os.path.normpath(dname + "/../src/amplitude_sort.c"),
                        os.path.normpath(dname + "/../src/analyze.c"),
                        os.path.normpath(resample[0]),
                        os.path.normpath(dname + "/../src/envelope_sort.c"),
                        os.path.normpath(dname + "/../src/frequency_sort.c"),
                        os.path.normpath(dname + "/../src/helpers.c")],
               libraries=["avformat", "avutil", "avcodec", resample[1]],
               include_dirs=["/usr/include/ffmpeg/", dname + "/../include"])

header = ''.join([i for i in open(dname + "/../include/bliss.h",
                                  'r').readlines()
                  if not i.strip().startswith("#")])
ffi.cdef(header)

if __name__ == "__main__":
    ffi.compile(tmpdir="/tmp/bliss/")
