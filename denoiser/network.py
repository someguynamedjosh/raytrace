'''
STATS:
Input: 64x64 noisy color image.
Output: 64x64 residual color image.
Error: mean squared.
Convergence: dunno i'll test it later.
'''
from warnings import simplefilter 
simplefilter(action='ignore', category=FutureWarning)

from matplotlib import pyplot
import numpy as np
import random
import tensorflow as tf
from tensorflow import keras

print('Libraries loaded.')

def import_file(path):
    f = open(path, 'rb')
    content = f.read()
    f.close()
    array = np.frombuffer(content, dtype=np.uint8)
    array = np.reshape(array, (512, 512, 4))
    array = np.delete(array, 3, 2)
    return array / 256.0

def import_files():
    files = []
    for i in range(174):
        base = 'training/' + str(i) + '/'
        files.append((
            import_file(base + 'sum.dat'),
            import_file(base + '00.dat'),
        ))
        print('Loaded ' + str(i + 1) + ' of 174')
    return files

files = import_files()
print('.dat files imported.')

INPUT_SIZE = 64
OUTPUT_SIZE = 64
HALF_DELTA = (INPUT_SIZE - OUTPUT_SIZE) // 2

def crop(image, dx, dy, size):
    return image[dx:dx+size,dy:dy+size]

def crop_input_to_output(image):
    return crop(image, HALF_DELTA, HALF_DELTA, OUTPUT_SIZE)

def random_crops(files):
    truths = []
    noisies = []
    residuals = []
    for _ in range(2048):
        collection = files[random.randint(0, len(files) - 1)]
        x, y = random.randint(0, 512 - INPUT_SIZE - 1), random.randint(0, 512 - INPUT_SIZE - 1)

        noisy = crop(collection[1], x, y, INPUT_SIZE)
        noisies.append(noisy)

        truth = crop_input_to_output(crop(collection[0], x, y, INPUT_SIZE))
        truths.append(truth)

        delta = (INPUT_SIZE - OUTPUT_SIZE) // 2
        residuals.append((crop_input_to_output(noisy) - truth + 1.0) * 0.5)

    return np.stack(noisies), np.stack(residuals), np.stack(truths)

model = keras.models.Sequential([
    keras.layers.Conv2D(64, (3, 3), input_shape=(64, 64, 3), activation='relu'),
    keras.layers.Conv2D(64, (3, 3), activation='relu'),
    keras.layers.Conv2D(64, (3, 3), activation='relu'),
    keras.layers.Conv2D(64, (3, 3), activation='relu'),
    keras.layers.Conv2D(64, (3, 3), activation='relu'),
    keras.layers.Conv2D(64, (3, 3), activation='relu'),
    keras.layers.Conv2D(64, (3, 3), activation='relu'),
    keras.layers.Conv2D(3, (3, 3), activation='relu'),
])

inputs = keras.layers.Input(shape=(64, 64, 3))
encode1 = keras.layers.Conv2D(16, 3, padding='same')(inputs)
pool1 = keras.layers.MaxPool2D()(encode1) # 32x32x16
encode2 = keras.layers.Conv2D(32, 3, padding='same')(pool1)
pool2 = keras.layers.MaxPool2D()(encode2) # 16x16x32
encode3 = keras.layers.Conv2D(64, 3, padding='same')(pool2)
pool3 = keras.layers.MaxPool2D()(encode3) # 8x8x64

encode4 = keras.layers.Conv2D(64, 3, padding='same')(pool3)
pool4 = keras.layers.MaxPool2D()(encode4) # 4x4x64

decode3 = keras.layers.Conv2D(64, 3, padding='same')(pool4)
upsample3 = keras.layers.UpSampling2D()(decode3) # 8x8x64
skip3 = keras.layers.add([pool3, upsample3])
decode2 = keras.layers.Conv2D(32, 3, padding='same')(skip3)
upsample2 = keras.layers.UpSampling2D()(decode2) # 16x16x32
skip2 = keras.layers.add([pool2, upsample2])
decode1 = keras.layers.Conv2D(16, 3, padding='same')(skip2)
upsample1 = keras.layers.UpSampling2D()(decode1) # 32x32x16
skip1 = keras.layers.add([pool1, upsample1])
decode0 = keras.layers.Conv2D(16, 3, padding='same')(skip1)
upsample0 = keras.layers.UpSampling2D()(decode0) # 64x64x8
skip0 = keras.layers.add([encode1, upsample0])
output = keras.layers.Conv2D(3, 3, padding='same')(skip0)

model = keras.models.Model(
    inputs=inputs,
    outputs=output,
)

model.summary()

model.compile(
    optimizer='adam',
    loss='mean_squared_error',
)

def show():
    noisies, residuals, truths = random_crops(files)
    decoded = model.predict(noisies[0:12])

    for i in range(10):
        pyplot.subplot(5, 10, i + 1)
        pyplot.xticks([])
        pyplot.yticks([])
        pyplot.imshow(truths[i])
        pyplot.subplot(5, 10, i + 11)
        pyplot.xticks([])
        pyplot.yticks([])
        pyplot.imshow(crop_input_to_output(noisies[i]))
        pyplot.subplot(5, 10, i + 21)
        pyplot.xticks([])
        pyplot.yticks([])
        pyplot.imshow(crop_input_to_output(noisies[i]) - (decoded[i] * 2.0 - 1.0))
        pyplot.subplot(5, 10, i + 31)
        pyplot.xticks([])
        pyplot.yticks([])
        pyplot.imshow(decoded[i])
        pyplot.subplot(5, 10, i + 41)
        pyplot.xticks([])
        pyplot.yticks([])
        pyplot.imshow(residuals[i])

    pyplot.show()

for i in range(2000):
    print('Epoch ' + str(i))
    noisies, residuals, _ = random_crops(files)
    model.fit(noisies, residuals, 128)
show()

'''

(image_train, label_train), (image_test, label_test) = keras.datasets.mnist.load_data()
image_train, image_test = image_train / 255.0, image_test / 255.0

model = keras.models.Sequential([
    keras.layers.Flatten(input_shape=(28, 28)),
    keras.layers.Dense(128, activation='relu'),
    keras.layers.Dropout(0.2),
    keras.layers.Dense(10, activation='softmax'),
])

model.compile(
    optimizer='adam',
    loss='sparse_categorical_crossentropy',
    metrics=['accuracy'],
)

model.fit(image_train, label_train, epochs=10)

model.evaluate(image_test, label_test)
'''
