# setup.py (with automatic dependency tracking)
from setuptools import setup

setup(
    name="Bliss",
    version="1.0.0",
    author="Phyks (Lucas Verney)",
    author_email="phyks@phyks.me",
    description="A wrapper around Bliss.",
    license="MIT",
    keywords="example documentation tutorial",
    url="https://github.com/Polochon-street/bliss",
    packages=['bliss'],
    long_description=open("../README.md", 'r').read(),
    # classifiers=[
    #     "Development Status :: 3 - Alpha",
    #     "Topic :: Utilities",
    #     "License :: OSI Approved :: BSD License",
    # ],
    setup_requires=["cffi>=1.0.0"],
    cffi_modules=["./build_bliss.py:ffi"],
    install_requires=["cffi>=1.0.0"],
)
