#!/usr/bin/env python3

from matplotlib import pyplot
import sys

def gamma(b):
    return (b / 255.0) ** 0.4

def load_image(path):
    f = open(path, 'rb')
    data = f.read()
    index = 0
    image = []

    for y in range(0, 512):
        row = []
        for x in range(0, 512):
            row.append([
                gamma(data[index]),
                gamma(data[index+1]),
                gamma(data[index+2]),
            ])
            index += 4
        image = [row] + image
    
    return image

def show_image(image):
    pyplot.imshow(image, interpolation='nearest')
    pyplot.show()

def command_single():
    if len(sys.argv) < 3:
        print('Usage: dat_viewer.py single [path to image]')
    path = sys.argv[2]
    image = load_image(path)
    show_image(image)

def command_average():
    if len(sys.argv) < 5:
        print('Usage: dat_viewer.py average [path to folder] [low index] [high index]')
    path = sys.argv[2]
    low = int(sys.argv[3])
    high = int(sys.argv[4])
    image = load_image(path + str(high).zfill(4) + '.dat')
    for index in range(low, high):
        piece = load_image(path + str(index).zfill(4) + '.dat')
        print('Loaded image ' + str(index))
        for x in range(0, len(piece)):
            for y in range(0, len(piece)):
                image[x][y][0] += piece[x][y][0]
                image[x][y][1] += piece[x][y][1]
                image[x][y][2] += piece[x][y][2]
    num_images = high - low + 1
    for x in range(0, len(image)):
        for y in range(0, len(image)):
            image[x][y][0] /= num_images
            image[x][y][1] /= num_images
            image[x][y][2] /= num_images
    show_image(image)


if len(sys.argv) < 2:
    print('Usage: dat_viewer.py [single|average]')
    sys.exit(1)

if sys.argv[1] == 'single':
    command_single()
elif sys.argv[1] == 'average':
    command_average()
else:
    print('Usage: dat_viewer.py [single|average]')
    sys.exit(1)
