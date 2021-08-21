#!/bin/env python3
import pathlib
import os
from datetime import datetime
from shutil import copyfile

folder = "/run/media/nuke/6538-3831/DCIM/105_PANA"

all_files = os.listdir(folder)
for file in all_files:
    file_path = os.path.join(folder, file)
    fname = pathlib.Path(file_path)
    assert fname.exists(), f"No such file: {fname}"

    date = datetime.fromtimestamp(fname.stat().st_ctime_ns / 1000000000)
    name = date.strftime("VID_%Y%m%d_%H%M%S")

    if file.endswith(".JPG"):
        name = date.strftime("IMG_%Y%m%d_%H%M%S.jpg")
    else:
        name = date.strftime("VID_%Y%m%d_%H%M%S.mp4")

    if not os.path.exists("/home/nuke/temp/unsorted/"):
        os.mkdir("/home/nuke/temp/unsorted/")
    copyfile(file_path, os.path.join("/home/nuke/temp/unsorted/", name))

    print(f"Oldname: {file}, New name: {name}")
