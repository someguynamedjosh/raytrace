function setup() {
    createCanvas(400, 400);
    noLoop();
}

function intHash(a)
{
   a = (a+0x7ed55d16) + (a<<12);
   a = (a^0xc761c23c) ^ (a>>19);
   a = (a+0x165667b1) + (a<<5);
   a = (a+0xd3a2646c) ^ (a<<9);
   a = (a+0xfd7046c5) + (a<<3);
   a = (a^0xb55a4f09) ^ (a>>16);
   return a;
}

function randomValueAtPoint(x, y, seed) {
    if (!seed) seed = 0;
    [x, y] = [Math.round(x), Math.round(y)];
    randomSeed(intHash(x) ^ intHash(~y) ^ intHash(seed));
    return random();
}

function coherentNoise(x, y, seed) {
    const [tl, tr, bl, br] = [
        randomValueAtPoint(Math.floor(x), Math.floor(y), seed),
        randomValueAtPoint(Math.ceil(x), Math.floor(y), seed),
        randomValueAtPoint(Math.floor(x), Math.ceil(y), seed),
        randomValueAtPoint(Math.ceil(x), Math.ceil(y), seed),
    ];
    const ltr = x % 1.0;
    const ttb = y % 1.0;
    const [top, bottom] = [lerp(tl, tr, ltr), lerp(bl, br, ltr)];
    return lerp(top, bottom, ttb);
}

function perlin(x, y, layers, seed) {
    let size = 1.0, power = 0.5;
    let value = coherentNoise(x, y, seed) * power;
    let offset = 500;
    for ([lsize, lpower] of layers) {
        size *= lsize;
        power *= lpower;
        value += coherentNoise((x + offset) * size, y * size, seed) * power;
        offset += 100;
    }
    return value;
}

function func(x, y) {
    const perlinLayers = [
        [2, 0.5],
        [2, 0.5],
        [2, 0.5],
        [2, 0.5],
    ];
    const intensity = perlin(x, y, perlinLayers);
    return [intensity * 255, intensity * 255, intensity * 255];
}

function draw() {
    background(0);
    loadPixels();
    for (let x = 0; x < 200; x++) {
        for (let y = 0; y < 200; y++) {
            const pindex = (y * 400 + x) * 4;
            const scale = 30;
            const value = func(x / scale, y / scale);
            pixels[pindex + 0] = value[0];
            pixels[pindex + 1] = value[1];
            pixels[pindex + 2] = value[2];
        }
    }
    updatePixels();
}