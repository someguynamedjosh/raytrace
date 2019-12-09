import megamu.mesh.*;
import java.awt.Polygon;

PImage biomeMap;

int ZOOM = 2;

void setup() {
  size(1000, 1000);
  noSmooth();
  noLoop();
  noiseDetail(4, 0.5);
  biomeMap = loadImage("biome_map.png");
}

void draw() {
  float[][] points = new float[100][2];

  for (int i = 0; i < 100; i++) {
    points[i][0] = i % 10 * 300 + random(300);
    points[i][1] = i / 10 * 300 + random(300);
  }
  // Shuffle up the points.
  for (int i = 0; i < 1000; i++) {
    int index1 = (int) random(0, points.length - 1), index2 = (int) random(0, points.length - 1);
    float tempx = points[index1][0], tempy = points[index1][1];
    points[index1][0] = points[index2][0];
    points[index1][1] = points[index2][1];
    points[index2][0] = tempx;
    points[index2][1] = tempy;
  }

  Voronoi biomes = new Voronoi( points );
  
  MPolygon[] voroniRegions = biomes.getRegions();
  Polygon[] checkableRegions = new Polygon[points.length];
  for (int i = 0; i < points.length; i++) {
    checkableRegions[i] = new Polygon();
    float[][] coords = voroniRegions[i].getCoords();
    for (int point = 0; point < coords.length; point++) {
      checkableRegions[i].addPoint(
        (int) coords[point][0], 
        (int) coords[point][1]
      );
    }
  }

  for (int x = 0; x < 1000; x++) {
    for (int y = 0; y < 1000; y++) {
      int index = 0;
      noiseDetail(4, 0.3);
      float dx = (noise(x / 100.0, y / 100.0, 0.5) - 0.5) * 200.0 + random(-20, 20);
      float dy = (noise(x / 100.0, y / 100.0, 5.5) - 0.5) * 200.0 + random(-20, 20);
      for (Polygon region : checkableRegions) {
        if (region.contains((x + dx) * ZOOM, (y + dy) * ZOOM)) {
          stroke(index % 8 % 2 * 256, index % 8 / 2 % 2 * 256, index % 8 / 4 * 256);
          point(x, y);
          break;
        }
        index++;
      }
    }
  }
}
