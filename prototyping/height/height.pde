float SCALE = 500.0;
int DERIVATIVE_SIZE = 1;

void setup() {
  size(1000, 1000);
  noSmooth();
  noLoop();
  noiseDetail(1, 0.5);
}

class P2 {
  public int x, y;

  public P2(int x, int y) {
    this.x = x;
    this.y = y;
  }
}

int noisefun(int x, int y) {
  return (int) (noise(x / SCALE + 0.1, y / SCALE + 0.1, 0.5) * 999999.0);
}

P2 noiseD1(int x, int y) {
  int middle = noisefun(x, y);
  return new P2(
    noisefun(x+DERIVATIVE_SIZE, y) - middle,
    noisefun(x, y+DERIVATIVE_SIZE) - middle
  );
}

P2 noiseD2(int x, int y) {
  int middle = noisefun(x, y);
  P2 d1a = new P2(
    noisefun(x+DERIVATIVE_SIZE, y) - middle,
    noisefun(x, y+DERIVATIVE_SIZE) - middle
  );
  P2 d1b = new P2(
    middle - noisefun(x-DERIVATIVE_SIZE, y),
    middle - noisefun(x, y-DERIVATIVE_SIZE)
  );
  return new P2(
    d1a.x - d1b.x,
    d1a.y - d1b.y
  );
}

void draw() {
  fill(0, 1);
  for (int x = 0; x < 1000; x++) {
    for (int y = 0; y < 1000; y++) {
      int height = noisefun(x, y);
      int d1 = (height - noisefun(x-1, y)) * 1;
      stroke(d1, d1, d1);//, (d1.x + 1.0) * 128, (d1.y + 1.0) * 128);
      if (height == noisefun(x-1, y)) {
        stroke(255, 0, 255);
      }
      point(x, y);
    }
  }
}
