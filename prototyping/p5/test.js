function setup() {
    createCanvas(400, 400);
    noLoop();
}

function fractal(x, y, dx, dy) {
    const d = Math.abs(dx) + Math.abs(dy);
    strokeWeight(d / 4);
    line(x, y, x + dx, y + dy);
    const newd = d * 0.7;
    if (newd < 3) return;
    const probability = 0.8;
    if (dx == 0) {
        if (Math.random() < probability) fractal(x + dx, y + dy, newd, 0);
        if (Math.random() < probability) fractal(x + dx, y + dy, -newd, 0);
    } else {
        if (Math.random() < probability) fractal(x + dx, y + dy, 0, newd);
        if (Math.random() < probability) fractal(x + dx, y + dy, 0, -newd);
    }
}

function ripple(x, y, frequency, strength, seed) {
    return [
        noise(x * frequency, y * frequency, 0.5 + seed) * strength,
        noise(x * frequency, y * frequency, 1.5 + seed) * strength,
    ];
}

function branchingFractal(initialSegments, mutator) {
    let newBranches = initialSegments;
    while (newBranches.length > 0) {
        const nextBranch = newBranches.pop();
        let results = mutator(nextBranch);
        newBranches = results.concat(newBranches);
        if (newBranches.length > 5000) {
            console.error("Too many concurrent segments!");
            return;
        }
    }
}

function rand(min, max) {
    return Math.random() * (max - min) + min;
}

function pick(options) {
    return options[Math.floor(Math.random() * options.length)];
}

function chance(percentChance) {
    return Math.random() < percentChance;
}

const PI = Math.PI;

function curvyFractal() {
    const initialSegments = [
        { x: 20, y: 200, angle: 0, lifetime: 30, power: 10, fertility: 0, }
    ];
    const mutator = element => {
        let { x, y, angle, lifetime, power, fertility } = element;
        const dx = Math.cos(angle) * power;
        const dy = Math.sin(angle) * power;
        const nx = x + dx;
        const ny = y + dy;
        strokeWeight(power);
        line(x, y, nx, ny);
        if (lifetime <= 0) {
            return [];
        } else {
            let branches = [];
            fertility += 0.2;
            if (chance(fertility - 0.3) && power > 3) {
                branches.push({
                    x: nx,
                    y: ny,
                    angle: angle + pick([PI / 2, -PI / 2]) + rand(-0.2, 0.2),
                    lifetime: 30,
                    power: power / 3,
                    fertility: 0,
                });
                fertility = 0;
            }
            branches.push({
                x: nx,
                y: ny,
                angle: angle + rand(-0.2, 0.2),
                lifetime: lifetime - 1,
                power,
                fertility,
            });
            return branches;
        }
    };
    branchingFractal(initialSegments, mutator);
}

function draw() {
    background(0);
    stroke(255);
    curvyFractal();
}