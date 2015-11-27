cmake_minimum_required(VERSION 3.0)

project(bliss C)

find_package(PkgConfig REQUIRED)

pkg_check_modules(MULTIMEDIA REQUIRED libavformat libavutil libavcodec)

include_directories(${MULTIMEDIA_INCLUDE_DIRS} include/)
link_directories(${MULTIMEDIA_LIBRARY_DIRS})
add_definitions(${MULTIMEDIA_CFLAGS_OTHER})
add_definitions(-Wall -Wno-long-long -pedantic)

#####################################
# Bliss target to build the library #
#####################################
add_library(bliss SHARED
    src/amplitude_sort.c
    src/decode.c
    src/envelope_sort.c
    src/frequency_sort.c
	src/helpers.c
    src/analyze.c)
target_link_libraries(bliss
    ${MULTIMEDIA_LIBRARIES}
    m)

set_property(TARGET bliss
    PROPERTY C_STANDARD 99)


#############################
# Examples building targets #
#############################
add_executable(analyze
    examples/analyze.c)
target_link_libraries(analyze bliss)

add_executable(distance
    examples/distance.c)
target_link_libraries(distance bliss)

add_custom_target(examples
    DEPENDS analyze distance)


##############################
# Unittests building targets #
##############################
enable_testing()
add_executable(test_analyze
    tests/test_analyze.c)
target_link_libraries(test_analyze bliss)

add_test(ctest_build_test_analyze "${CMAKE_COMMAND}" --build ${CMAKE_BINARY_DIR} --target test_analyze)
add_test(ctest_run_test_analyze test_analyze)
set_tests_properties(ctest_run_test_analyze PROPERTIES DEPENDS ctest_build_test_analyze)


###################
# Install targets #
###################
install(TARGETS bliss DESTINATION /usr/lib)
install(FILES include/bliss.h DESTINATION /usr/include)
