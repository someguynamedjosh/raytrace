float SCALE = 100.0;
float PADDING = 2.0;
int DERIVATIVE_SIZE = 1;
int VORONI_TABLE_SIZE = 500;

void setup() {
  size(1000, 1000);
  noSmooth();
  noLoop();
  noiseDetail(1, 0.5);
}

class P2 {
  public float x, y;

  public P2(float x, float y) {
    this.x = x;
    this.y = y;
  }

  public float distSquared(P2 other) {
    float dx = other.x - x, dy = other.y - y;
    return dx * dx + dy * dy;
  }

  public float dist(P2 other) {
    return (float) Math.sqrt(distSquared(other));
  }
}

float clip(float value) {
  return Math.min(Math.max(value, 0), 1);
}

// Converts a value from range oldMin-oldMax to range 0-1
float mapFromRange(float value, float oldMin, float oldMax) {
  return clip((value - oldMin) / (oldMax - oldMin));
}

// Converts a vale from range 0-1 to range min-max
float mapToRange(float value, float min, float max) {
  return clip(value * (max - min) + min);
}

P2[][] voroniPointTable = new P2[VORONI_TABLE_SIZE][VORONI_TABLE_SIZE];

void populateVoroniPointTable() {
  for (int x = 0; x < VORONI_TABLE_SIZE; x++) {
    for (int y = 0; y < VORONI_TABLE_SIZE; y++) {
      voroniPointTable[x][y] = new P2(
        x + random(0.0, 1.0), 
        y + random(0.0, 1.0)
      );
    }
  }
}

P2 voroniPoint(int tilex, int tiley) {
  return voroniPointTable[tilex][tiley];
}

float voroni(float x, float y) {
  P2 center = new P2(x, y);
  int tilex = (int) x, tiley = (int) y;
  float minDistSquared = center.distSquared(voroniPoint(tilex, tiley));
  minDistSquared = Math.min(minDistSquared, center.distSquared(voroniPoint(tilex + 1, tiley + 1)));
  minDistSquared = Math.min(minDistSquared, center.distSquared(voroniPoint(tilex + 0, tiley + 1)));
  minDistSquared = Math.min(minDistSquared, center.distSquared(voroniPoint(tilex - 1, tiley + 1)));
  minDistSquared = Math.min(minDistSquared, center.distSquared(voroniPoint(tilex + 1, tiley + 0)));
  minDistSquared = Math.min(minDistSquared, center.distSquared(voroniPoint(tilex - 1, tiley + 0)));
  minDistSquared = Math.min(minDistSquared, center.distSquared(voroniPoint(tilex + 1, tiley - 1)));
  minDistSquared = Math.min(minDistSquared, center.distSquared(voroniPoint(tilex + 0, tiley - 1)));
  minDistSquared = Math.min(minDistSquared, center.distSquared(voroniPoint(tilex - 1, tiley - 1)));
  minDistSquared = Math.min(minDistSquared, center.distSquared(voroniPoint(tilex + 1, tiley + 2)));
  minDistSquared = Math.min(minDistSquared, center.distSquared(voroniPoint(tilex + 0, tiley + 2)));
  minDistSquared = Math.min(minDistSquared, center.distSquared(voroniPoint(tilex - 1, tiley + 2)));
  minDistSquared = Math.min(minDistSquared, center.distSquared(voroniPoint(tilex + 1, tiley - 2)));
  minDistSquared = Math.min(minDistSquared, center.distSquared(voroniPoint(tilex + 0, tiley - 2)));
  minDistSquared = Math.min(minDistSquared, center.distSquared(voroniPoint(tilex - 1, tiley - 2)));
  minDistSquared = Math.min(minDistSquared, center.distSquared(voroniPoint(tilex + 2, tiley + 1)));
  minDistSquared = Math.min(minDistSquared, center.distSquared(voroniPoint(tilex + 2, tiley + 0)));
  minDistSquared = Math.min(minDistSquared, center.distSquared(voroniPoint(tilex + 2, tiley - 1)));
  minDistSquared = Math.min(minDistSquared, center.distSquared(voroniPoint(tilex - 2, tiley + 1)));
  minDistSquared = Math.min(minDistSquared, center.distSquared(voroniPoint(tilex - 2, tiley + 0)));
  minDistSquared = Math.min(minDistSquared, center.distSquared(voroniPoint(tilex - 2, tiley - 1)));
  return (float) Math.sqrt(minDistSquared);
}

void computePixel(int x, int y) {
  float fx = x / SCALE + PADDING, fy = y / SCALE + PADDING;

  float base = voroni(fx, fy); // Large-scale features.

  float detail = voroni(fx * 4, fy * 4); // Small-scale details.
  detail = mapToRange(detail, 0.73, 1.0); // Make the minimum value of the texture be 0.73
  detail *= mapFromRange(base, 0.34, 0.79); // Only show details where the base is high.

  base = mapFromRange(base, 0.4, 1.0); // Cut out low values.
  base += detail; // Add the details.
  base /= 2.0;
  base = (float) Math.pow(base, 2.2); // Make the slope of the texture exponential to be mountainy-er.

  noiseDetail(0, 0.0);
  float rustle = noise(fx * 0.8, fy * 0.8); // This texture will be used to make the heights of mountains more varied.
  rustle = mapToRange(mapFromRange(rustle, 0.15, 1.0), 0.15, 1.0);
  rustle = (float) Math.pow(rustle, 2.0);
  base *= rustle;

  float v = base;
  stroke(v * 256, v * 256, v * 256, 256);
}

void draw() {
  fill(0, 1);
  populateVoroniPointTable();
  for (int x = 0; x < 1000; x++) {
    for (int y = 0; y < 1000; y++) {
      computePixel(x, y);
      point(x, y);
    }
  }
  System.out.println("Done");
}
