// Build a glpaper fragment shader from index.html's shader, with the sun computed on the GPU.
// glpaper only gives `time` (s since start) and `resolution`, so we bake the slow-moving
// solar angles at start time in double precision and advance them linearly with `time`.
// usage: node gen-glpaper.mjs [--lat 37.77] [--lon -122.42] [--facing 180] [--style blinds]
//                             [--plant 1] [--at 2026-11-19T09:00] [--speed 1] > windowlight.frag
import fs from 'node:fs';
const args = Object.fromEntries(process.argv.slice(2).join(' ').split('--').filter(Boolean)
  .map(s => s.trim().split(/\s+/)).map(([k, v]) => [k, v]));
const lat = +(args.lat ?? 37.77), lon = +(args.lon ?? -122.42), facing = +(args.facing ?? 180);
const style = { window: 0, blinds: 1, lattice: 2 }[args.style ?? 'blinds'];
const plant = +(args.plant ?? 1), speed = +(args.speed ?? 1);
const start = args.at ? new Date(args.at) : new Date();

const rad = Math.PI / 180, TAU = 2 * Math.PI;
const d0 = start.valueOf() / 86400000 - 0.5 + 2440588 - 2451545;
const mod = x => ((x % TAU) + TAU) % TAU;
const M0 = mod(rad * (357.5291 + 0.98560028 * d0));
const H0 = mod(rad * (280.16 + 360.9856235 * d0) + rad * lon); // before subtracting right ascension

const html = fs.readFileSync(new URL('./index.html', import.meta.url), 'utf8');
let fs_ = html.match(/const fs = `([\s\S]*?)`;/)[1];
fs_ = fs_
  .replace('uniform vec2 res;', 'uniform vec2 resolution;\n#define res resolution')
  .replace(/uniform vec3 sun;.*\n/, 'vec3 sun;\n')
  .replace(/uniform float sunAlt;.*\n/, 'float sunAlt;\n')
  .replace(/uniform int style;.*\n/, `const int style = ${style};\n`)
  .replace(/uniform float plant;\n/, `const float plant = ${plant.toFixed(1)};\n`);

const sunGlsl = `
// ---- sun position on the GPU (SunCalc formulas), angles baked at start ----
const float RAD = 0.017453292519943295;
const float E = RAD * 23.4397;
const float M0 = ${M0.toFixed(9)};
const float H0 = ${H0.toFixed(9)};
const float PHI = ${(lat * rad).toFixed(9)};
const float FACING = ${(facing * rad).toFixed(9)};
const float SPEED = ${speed.toFixed(3)};
void computeSun(){
  float d = time * SPEED / 86400.0;
  float M = M0 + RAD * 0.98560028 * d;
  float C = RAD * (1.9148 * sin(M) + 0.02 * sin(2.0 * M) + 0.0003 * sin(3.0 * M));
  float L = M + C + RAD * 102.9372 + 3.14159265;
  float dec = asin(sin(E) * sin(L));
  float ra = atan(sin(L) * cos(E), cos(L));
  float H = H0 + RAD * 360.9856235 * d - ra;
  float az = atan(sin(H), cos(H) * sin(PHI) - tan(dec) * cos(PHI)) + 3.14159265;
  float alt = asin(sin(PHI) * sin(dec) + cos(PHI) * cos(dec) * cos(H));
  float rel = az - FACING;
  sun = vec3(-cos(alt) * sin(rel), sin(alt), cos(alt) * cos(rel));
  sunAlt = alt;
}
`;
fs_ = fs_.replace('void main(){', sunGlsl + 'void main(){\n  computeSun();');
process.stdout.write('#version 100\n' + fs_);
