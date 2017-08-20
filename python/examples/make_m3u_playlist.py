#!/usr/bin/env python3

"""
This is a short script to analyze songs from a directory (non-recursive)
using bliss, and then build a simple .m3u playlist out of it using a song
picked by the user as the first song (or the first listed by os.path.expanduser).

The playlist is simply computed by increasing distance order from the first song.
"""
import bliss
import numpy as np

import os
import mimetypes
import sys
import argparse

parser = argparse.ArgumentParser(
    description='Scan for audio files in a directory and make a smart playlist out of them.',
)
parser.add_argument(
    'directory',
    type=str,
    help='directory location',
    default=os.getcwd(),
    nargs='?',
)
parser.add_argument(
    'seed',
    type=str,
    help='first song for the generated playlist',
    default='',
    nargs='?'
)
args = parser.parse_args()

url = args.directory
first_song_playlist = args.seed

file_list = [os.path.join(dp, f) for dp, dn, fn in os.walk(os.path.expanduser(url)) for f in fn]
audio_files = []

for file_n in file_list:
    guess = mimetypes.guess_type(file_n)[0]
    if guess is not None and "audio" in guess:
        audio_files.append(file_n)

playlist = []
force_vectors = []

for file_n in audio_files:
    with bliss.bl_song(file_n) as song:
        if song['duration'] > 0:
            force_vector = np.array((
                song['force_vector']['attack'],
                song['force_vector']['amplitude'],
                song['force_vector']['frequency'],
                song['force_vector']['tempo'],
            ))
            playlist.append(os.path.basename(file_n))
            force_vectors.append(force_vector)
    print('Analyzing %s...' % file_n)

index = 0
if first_song_playlist in playlist:
    index = playlist.index(first_song_playlist)

force_vectors = (force_vectors[index] - force_vectors)**2
force_vectors = np.sum(force_vectors, axis=1)
force_vectors = np.sqrt(force_vectors)
playlist = np.array(playlist)
playlist = playlist[force_vectors.argsort()]

m3u = open('playlist.m3u', 'w')
for item in playlist:
    m3u.write('%s\n' % item)
