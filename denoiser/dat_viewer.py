#!/usr/bin/env python3

from matplotlib import pyplot
import sys

def gamma(b):
    return b ** 0.4

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
            row.append([data[index + channel] / 255 for channel in channels])
            index += 4
        image = [row] + image
    
    return image

def sample(image, x, y): # Clamps coords to image bounds.
    x = max(x, 0)
    x = min(x, len(image) - 1)
    y = max(y, 0)
    y = min(y, len(image) - 1)
    return image[y][x]

def transform(image, function):
    return [[[function(c) for c in pixel] for pixel in row] for row in image]

def convolve(image, filter):
    result = []
    for y in range(0, len(image)):
        row = []
        for x in range(0, len(image)):
            row.append(filter(image, x, y))
        result.append(row)
        if y % 10 == 0:
            print(str(y) + '/' + str(len(image)))
    return result

def add_vec(v1, v2):
    return [a + b for a, b in zip(v1, v2)]

def vec_distance(v1, v2):
    return sum([(c1 - c2) ** 2 for c1, c2 in zip(v1, v2)]) ** 0.5 / (3 ** 0.5)

def scalar_mul_vec(vec, scalar):
    return [v * scalar for v in vec]

def box_blur_5x(image, x, y):
    sum = sample(image, x, y)
    for dy in range(-2, 3):
        for dx in range(-2, 3):
            sum = add_vec(sum, sample(image, x + dx, y + dy))
    return scalar_mul_vec(sum, 1 / 26)

#     dx,  dy, weight
gaussian_taps = [ ]

E = 2.71828

def gaussian(x, y, size):
    return E ** (-3 * (x ** 2 + y ** 2) / (size ** 2))

def make_taps(size, multiplier):
    for x in range(-size, size + 1):
        for y in range(-size, size + 1):
            weight = gaussian(x, y, size)
            if weight < 1 / 256:
                continue
            gaussian_taps.append([x * multiplier, y * multiplier, weight])

make_taps(16, 2)

def color_bilinear(image, x, y):
    center_color = sample(image, x, y)
    sum = scalar_mul_vec(center_color, gaussian_taps[0][2])
    total_weight = gaussian_taps[0][2]
    for tap in gaussian_taps[1:]:
        value_at_tap = sample(image, x + tap[0], y + tap[1])
        weight = 1.0 - vec_distance(center_color, value_at_tap)
        weight *= tap[2]
        total_weight += weight
        sum = add_vec(sum, scalar_mul_vec(value_at_tap, weight))
    return scalar_mul_vec(sum, 1 / total_weight)

def unpack_depth(pixel):
    return (pixel[-1] * 256.0 + pixel[-2]) ** 0.5

def depth_bilinear(image, x, y):
    center_depth = unpack_depth(sample(image, x, y))
    center_normal = sample(image, x, y)[3:6]
    sum = scalar_mul_vec(sample(image, x, y)[:3], gaussian_taps[0][2])
    total_weight = gaussian_taps[0][2]
    for tap in gaussian_taps[1:]:
        value_at_tap = sample(image, x + tap[0], y + tap[1])
        depth_difference = 5.0 * abs(center_depth - unpack_depth(value_at_tap))
        normal_difference = 20.0 * vec_distance(value_at_tap[3:6], center_normal)
        weight = 1.0 / (depth_difference + normal_difference + 1.0)
        weight *= tap[2]
        total_weight += weight
        sum = add_vec(sum, scalar_mul_vec(value_at_tap[:3], weight))
    return scalar_mul_vec(sum, 1 / total_weight)

def show_image(image):
    pyplot.imshow(transform(image, gamma), interpolation='nearest')
    pyplot.show()

def view_command():
    if len(sys.argv) != 3 and len(sys.argv) != 4:
        print('Usage: dat_viewer.py view [path to image] [optional channel list]')
        sys.exit(1)
    path = sys.argv[2]
    image = None
    if len(sys.argv) == 4:
        image = load_image(path, [int(i.strip()) for i in sys.argv[3].split(',')])
    else:
        image = load_image(path, [0, 1, 2])
    show_image(image)

def filter_command():
    print('filtering...')
    if len(sys.argv) != 4:
        print('Usage: dat_viewer.py filter [path to image] [box|color_bilinear|depth_bilinear]')
        sys.exit(1)
    path = sys.argv[2]
    filters = {
        'box': ([0, 1, 2], box_blur_5x),
        'color_bilinear': ([0, 1, 2], color_bilinear),
        'depth_bilinear': ([0, 1, 2, 4, 5, 6, 8, 9], depth_bilinear)
    }
    if sys.argv[3] not in filters.keys():
        print(sys.argv[3] + ' is not a valid filter.')
        sys.exit(1)
    image = load_image(path, filters[sys.argv[3]][0])
    show_image(convolve(image, filters[sys.argv[3]][1]))

def main():
    if len(sys.argv) < 3:
        print('Usage: dat_viewer.py [view|filter] [path to image] [optional args]')
        sys.exit(1)

    if sys.argv[1] == 'view':
        view_command()
    elif sys.argv[1] == 'filter':
        filter_command()
    else:
        print('Usage: dat_viewer.py [view|filter] [path to image] [optional args]')
        sys.exit(1)

main()
