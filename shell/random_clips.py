#!/bin/env python3

import subprocess
import random
import re
import os
import shutil
import json
import datetime


def convert_duration_to_seconds(duration):
    """Convert duration in HH:MM:SS.xx format to seconds."""
    hours, minutes, seconds, _ = map(float, re.split("[:.]", duration))
    total_seconds = hours * 3600 + minutes * 60 + seconds
    return total_seconds


def get_aspect_ratio(filename):
    cmd = [
        "ffprobe",
        "-v",
        "error",
        "-select_streams",
        "v:0",
        "-show_entries",
        "stream=width,height",
        "-of",
        "json",
        filename,
    ]
    result = subprocess.run(
        cmd, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True
    )
    info = json.loads(result.stdout)

    if "streams" in info and len(info["streams"]) > 0:
        width = info["streams"][0]["width"]
        height = info["streams"][0]["height"]
        print(f"{width}/{height}")
        aspect_ratio = width / height
        return aspect_ratio
    return None


def extract_random_clip(input_file, index):
    # Get the total duration of the video
    result = subprocess.run(
        ["ffmpeg", "-i", input_file], stderr=subprocess.PIPE, stdout=subprocess.PIPE
    )
    output = result.stderr.decode("utf-8")

    # Extract the duration using regex
    duration_match = re.search(r"Duration: (\d{2}:\d{2}:\d{2}.\d{2})", output)
    if not duration_match:
        print("Could not extract duration from the video file.")
        return
    duration = duration_match.group(1)

    # Convert the duration to total seconds
    duration_seconds = convert_duration_to_seconds(duration)

    if duration_seconds < 420:
        print(f"Duration is shorter than 10min: {duration_seconds}\n:    {input_file}")
        return

    # Calculate a random start time (excluding intro and outro)
    start_time = random.randint(60, int(duration_seconds) - 360)

    # Use ffmpeg to extract the clip
    file_name = f"{index}.mp4"
    output_file = f"random_clips_output/{file_name}"

    command = ["ffmpeg"]
    # Force reset of timestamps
    command += ["-fflags", "+genpts"]
    # Input file
    command += ["-i", input_file]
    # Duration of clip
    command += ["-ss", str(start_time)]
    # Start time of clip
    command += ["-t", "180"]
    # Video Codec
    command += ["-c:v", "libx265"]

    scale = "640:480"
    # - yadif is an adaptive deinterlacer
    # - Video Crop to 4:3
    # - Set resolution
    # aspect_ratio = get_aspect_ratio(input_file)
    # if aspect_ratio and aspect_ratio < (4 / 3):
    #    # Only add cropping if the clip is wider than 4:3
    #    command += ["-filter:v", f"yadif,crop=floor(ih/3)*4:ih,scale={scale}"]
    # else:
    command += ["-filter:v", f"yadif,scale={scale}"]  # No cropping needed

    # Avoid pixel format issues
    command += ["-pix_fmt", "yuv420p"]
    # Video Framerate + ensure constant framerate
    command += ["-r", "24", "-fps_mode", "cfr"]
    # Audio Codec
    command += ["-c:a", "aac"]
    # Audio bitrate
    command += ["-b:a", "192k"]
    # Audio Sample rate
    command += ["-ar", "48000"]
    # Audio channels to Stereo
    command += ["-ac", "2"]
    # Output file
    command += [output_file]

    subprocess.run(command)

    print(
        f"Random 5-minute clip from {input_file}"
        + f"Extracted from {start_time} seconds in."
        + "Output saved as {output_file}."
    )

    # Write the file name to the concat file so ffmpeg knows what to concat lateron.
    with open("random_clips_output/concat.txt", "a") as file:
        file.write(f"file {file_name}\n")


def main_fn():
    # Get the current directory
    current_directory = os.getcwd()

    if os.path.exists("random_clips_output"):
        shutil.rmtree("random_clips_output")
    if os.path.exists("output.mp4"):
        os.remove("output.mp4")
    os.makedirs("random_clips_output", exist_ok=True)

    # Walk through all directories and subdirectories and get all clips.
    mkv_files = []
    for root, _, files in os.walk(f"{current_directory}/input"):
        for file in files:
            if file.endswith(".mkv") or file.endswith(".mp4"):
                # Get the full path of the .mkv file
                full_path = os.path.join(root, file)
                mkv_files.append(full_path)

    # Randomize the list
    random.shuffle(mkv_files)

    # Print the list of .mkv files with their full paths
    for index, file in enumerate(mkv_files):
        now = datetime.datetime.now()
        print(f"\n\n{now}: Encoding clip {index}")
        extract_random_clip(file, index)

    # Concat the files.
    subprocess.run(
        [
            "ffmpeg",
            "-f",
            "concat",
            "-safe",
            "0",
            "-i",
            "random_clips_output/concat.txt",
            "-c",
            "copy",
            "output.mp4",
        ]
    )


if __name__ == "__main__":
    main_fn()
