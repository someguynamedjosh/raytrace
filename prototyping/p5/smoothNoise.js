function setup() {
    createCanvas(400, 400);
    noiseSeed(0);
    noLoop();
}

const d = 0.2;
function noisedx(x, y) {
    noiseDetail(Math.floor(-Math.log2(d)) + 2);
    return (noise(x + d, y) - noise(x - d, y)) / (d * 2);
}

function noisedy(x, y) {
    noiseDetail(Math.floor(-Math.log2(d)) + 2);
    return (noise(x, y + d) - noise(x, y - d)) / (d * 2);
}


function func(x, y) {
    const height = noise(x, y);
    const dx = noisedx(x, y);
    const dy = noisedy(x, y);
    const slope = Math.hypot(dx, dy);

    const eroded = height - slope * 0.5 + 0.5;
    const final = Math.pow(eroded * 0.8, 3.0);

    function signed(value) { return (value + 1) * 128; }
    function unsigned(value) { return value * 255; }
    const r = unsigned(final);
    return [r, r, r];
}

function draw() {
    background(0);
    loadPixels();
    for (let x = 0; x < 400; x++) {
        for (let y = 0; y < 400; y++) {
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