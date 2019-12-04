#!/usr/bin/env python3

from matplotlib import pyplot
import sys

def gamma(b):
    return (b / 255.0) ** 0.4

def get_channel_offset(channel):
    return channel % 4 + (channel // 4 * 512 * 512 * 4)

def load_image(path, channels):
    channels = [get_channel_offset(c) for c in channels]
    f = open(path, 'rb')
    data = f.read()
    index = 0
    image = []

    for y in range(0, 512):
        row = []
        for x in range(0, 512):
            row.append([
                gamma(data[index + channels[0]]),
                gamma(data[index + channels[1]]),
                gamma(data[index + channels[2]]),
            ])
            index += 4
        image = [row] + image
    
    return image

def show_image(image):
    pyplot.imshow(image, interpolation='nearest')
    pyplot.show()

if len(sys.argv) < 2:
    print('Usage: dat_viewer.py [path to image] [optional channel list]')
    sys.exit(1)

if len(sys.argv) != 2 and len(sys.argv) != 5:
    print('Usage: dat_viewer.py [path to image] [r channel] [g] [b]')
    sys.exit(1)

path = sys.argv[1]
image = None
if len(sys.argv) == 5:
    image = load_image(path, [int(i) for i in sys.argv[2:]])
else:
    image = load_image(path, [0, 1, 2])
show_image(image)

