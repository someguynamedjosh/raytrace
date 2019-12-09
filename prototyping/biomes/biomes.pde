import megamu.mesh.*;
import java.awt.Polygon;

PImage biomeMap;

void setup() {
  size(1000, 1000);
  noSmooth();
  noLoop();
  noiseDetail(4, 0.5);
  biomeMap = loadImage("biome_map.png");
}

void draw() {
  float[][] points = new float[25][2];

  for (int i = 0; i < 25; i++) {
    points[i][0] = i % 5 * 200 + random(300);
    points[i][1] = i / 5 * 200 + random(300);
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
      float dx = (noise(x / 300.0, y / 300.0, 0.5) - 0.5) * 500.0;
      float dy = (noise(x / 300.0, y / 300.0, 5.5) - 0.5) * 500.0;
      for (Polygon region : checkableRegions) {
        if (region.contains(x + dx, y + dy)) {
          stroke(index % 5 * 50, index / 5 * 50, 255);
          point(x, y);
          break;
        }
        index++;
      }
    }
  }
}
