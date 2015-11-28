from cffi import FFI

ffi = FFI()

ffi.set_source("bliss._bliss",
               "#include \"bliss.h\"",
               sources=["../src/amplitude_sort.c",
                        "../src/analyze.c",
                        "../src/decode.c",
                        "../src/envelope_sort.c",
                        "../src/frequency_sort.c",
                        "../src/helpers.c"],
               libraries=["avformat", "avutil", "avcodec"],
               include_dirs=["/usr/include/ffmpeg/", "../include/"])

header = '\n'.join([i for i in open("../include/bliss.h", 'r').readlines()
                    if not i.strip().startswith("#")])
ffi.cdef(header)

if __name__ == "__main__":
    ffi.compile()
