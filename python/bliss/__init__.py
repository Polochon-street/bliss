#!/usr/bin/env python
from bliss.bl_song import bl_song as bl_song
import bliss.distance as distance
import bliss.version as version
from bliss._bliss import lib

#FIXME cffi doesn't seem to support C define, so hotfix for now
BL_LOUD = 0;
BL_CALM = 1;
BL_UNKNOWN = 2;
BL_UNEXPECTED = -2;
BL_OK = 0;

__all__ = ["bl_song", "distance", "version"]
