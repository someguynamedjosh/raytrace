function setup() {
    createCanvas(400, 400);
    noLoop();
}

function fractal(x, y, dx, dy) {
    const d = Math.abs(dx) + Math.abs(dy);
    strokeWeight(d / 4);
    line(x, y, x + dx, y + dy);
    const newd = d * 0.7;
    if (newd < 2) return;
    const probability = 0.8;
    if (dx == 0) {
        if (Math.random() < probability) fractal(x + dx, y + dy, newd, 0);
        if (Math.random() < probability) fractal(x + dx, y + dy, -newd, 0);
    } else {
        if (Math.random() < probability) fractal(x + dx, y + dy, 0, newd);
        if (Math.random() < probability) fractal(x + dx, y + dy, 0, -newd);
    }
}

function draw() {
    background(0);
    stroke(255);
    fractal(200, 200, 0, -100);
    fractal(200, 200, 0, 100);
}