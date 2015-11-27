from bliss._bliss import lib


def version():
    """
    Wrapper around `bl_version` function which returns the current version.
    """
    return lib.bl_version()
