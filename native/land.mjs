// packs Natural Earth's 1:110m land (ne_110m_land.geojson, public domain) into land.txt for the control page's globe
// usage: node land.mjs  (with land.json downloaded from github.com/nvkelso/natural-earth-vector, geojson/)
import { readFileSync, writeFileSync } from 'node:fs';
const g = JSON.parse(readFileSync('land.json', 'utf8'));
// rings as lon,lat in quarter degrees, each point a delta from the last, rings split by ';'
const Q = 4, out = [];
let pts = 0;
for (const f of g.features) {
  const polys = f.geometry.type == 'Polygon' ? [f.geometry.coordinates] : f.geometry.coordinates;
  for (const poly of polys) for (const ring of poly) {
    let px = 0, py = 0, s = [];
    for (const [lo, la] of ring) {
      const x = Math.round(lo * Q), y = Math.round(la * Q);
      if (s.length && x == px && y == py) continue;
      s.push((x - px) + ',' + (y - py)); px = x; py = y; pts++;
    }
    if (s.length > 2) out.push(s.join(' '));
  }
}
const txt = out.join(';');
writeFileSync('land.txt', txt);
console.error(pts, 'points,', txt.length, 'chars');
