#!/usr/bin/env python3

from pathlib import Path

import json5
from PIL import Image

with open(Path(__file__).parent / "run" / "pass" / "raytracer.stdout") as f:
    height = int(f.readline())
    width = int(f.readline())
    img = Image.new("RGB", [width, height])
    data = img.load()
    for y in range(height):
        for x in range(width):
            s = f.readline()
            pixel = json5.loads(s)
            data[x, y] = (pixel["red"], pixel["green"], pixel["blue"])
img.save("raytracer.png")
