cmake_minimum_required(VERSION 3.1)

project(bliss C)

find_package(PkgConfig REQUIRED)

pkg_check_modules(MULTIMEDIA REQUIRED libavformat libavutil libavcodec)

include_directories(${MULTIMEDIA_INCLUDE_DIRS})
link_directories(${MULTIMEDIA_LIBRARY_DIRS})
add_definitions(${MULTIMEDIA_CFLAGS_OTHER})

add_library(bliss SHARED src/amp_sort.c src/decode.c src/envelope.c src/freq_sort.c src/analyze.c)
target_link_libraries(bliss ${MULTIMEDIA_LIBRARIES})

set_property(TARGET bliss PROPERTY C_STANDARD 99)

install(TARGETS bliss DESTINATION /usr/lib)
install(FILES src/bliss.h DESTINATION /usr/include)
