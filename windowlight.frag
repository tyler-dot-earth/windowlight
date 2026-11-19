#version 100

precision highp float;
uniform vec2 resolution;
#define res resolution
vec3 sun;
float sunAlt;
uniform float time;
const int style = 1;
const float plant = 1.0;

const float D_WIN = 0.55;   // window plane distance from wall
const float D_PLANT = 1.05; // plant distance from wall (outside the window)
const float TILT = 0.55;    // window plane angle against the wall, radians

float hash(vec2 p){ return fract(sin(dot(p, vec2(127.1,311.7))) * 43758.5453); }
float noise(vec2 p){ vec2 i=floor(p), f=fract(p); f=f*f*(3.-2.*f);
  return mix(mix(hash(i),hash(i+vec2(1,0)),f.x), mix(hash(i+vec2(0,1)),hash(i+vec2(1,1)),f.x), f.y); }
float fbm(vec2 p){ float a=.5, s=0.; for(int i=0;i<4;i++){ s+=a*noise(p); p*=2.03; a*=.5; } return s; }

// window aperture in window-plane coords (q), 1 = open
float aperture(vec2 q){
  vec2 c = vec2(0.15, 0.62), h = vec2(0.42, 0.55);       // centre and half-size of the opening
  vec2 d = abs(q - c) - h;
  float open = step(max(d.x, d.y), 0.0);
  vec2 l = q - c;
  if (style == 0) {           // four-pane window: one mullion, one transom
    open *= step(0.018, abs(l.x)) * step(0.018, abs(l.y - 0.1));
  } else if (style == 1) {    // horizontal blinds, slightly tilted slats
    float s = fract(l.y * 11.0 + l.x * 0.15);
    open *= smoothstep(0.30, 0.36, s) * (1.0 - smoothstep(0.92, 0.98, s));
  } else {                    // diamond lattice
    vec2 r = mat2(0.7071,-0.7071,0.7071,0.7071) * l * 9.0;
    vec2 g = abs(fract(r) - 0.5);
    open *= step(0.07, min(g.x, g.y));
    open *= step(0.02, abs(l.x)); // centre post
  }
  return open;
}

// leaf silhouettes in plant-plane coords, 1 = blocked
float leaf(vec2 p, vec2 c, float ang, float sz){
  vec2 d = p - c; float s = sin(ang), co = cos(ang);
  d = mat2(co, -s, s, co) * d / sz;
  float e = length(vec2(d.x * 1.0, d.y * 2.1)) - 0.5;            // ellipse
  float notch = smoothstep(0.02, 0.0, abs(d.y) - 0.03) * step(0.0, d.x); // split down the middle
  return step(e, 0.0) * (1.0 - notch * 0.9);
}
float foliage(vec2 p){
  float b = 0.0;
  float sway = sin(time * 0.6) * 0.012 + sin(time * 1.7 + p.y * 3.0) * 0.004;
  p.x += sway * (p.y - 0.0);
  // stem
  b = max(b, smoothstep(0.012, 0.004, abs(p.x - 0.42 - 0.06 * sin(p.y * 2.5))) * step(p.y, 1.15) * step(0.2, p.y));
  for (int i = 0; i < 9; i++) {
    float fi = float(i);
    float y = 0.25 + fi * 0.11;
    float side = mod(fi, 2.0) * 2.0 - 1.0;
    vec2 c = vec2(0.42 + side * (0.13 + 0.05 * hash(vec2(fi, 1.0))), y);
    float a = side * (0.5 + 0.3 * hash(vec2(fi, 2.0))) + sin(time * (0.9 + fi * 0.13) + fi) * 0.05;
    b = max(b, leaf(p, c, a, 0.26 + 0.08 * hash(vec2(fi, 3.0))));
  }
  return b;
}

// soft sample: average over a disk to fake the penumbra
float jit = 0.0;
float softAperture(vec2 q, float r){
  float s = 0.0;
  for (int i = 0; i < 16; i++){
    float a = float(i) * 2.39996 + jit; float rr = sqrt((float(i) + .5) / 16.0) * r;
    s += aperture(q + rr * vec2(cos(a), sin(a)));
  }
  return s / 16.0;
}
float softFoliage(vec2 q, float r){
  float s = 0.0;
  for (int i = 0; i < 12; i++){
    float a = float(i) * 2.39996 + jit; float rr = sqrt((float(i) + .5) / 12.0) * r;
    s += foliage(q + rr * vec2(cos(a), sin(a)));
  }
  return s / 12.0;
}

vec3 sunColor(float alt){
  float a = clamp(alt / 0.6, 0.0, 1.0);                  // 0 at horizon, 1 at ~35 deg
  vec3 low = vec3(1.0, 0.55, 0.25), high = vec3(1.0, 0.93, 0.80);
  return mix(low, high, smoothstep(0.0, 1.0, a));
}


// ---- sun position on the GPU (SunCalc formulas), angles baked at start ----
const float RAD = 0.017453292519943295;
const float E = RAD * 23.4397;
const float M0 = 5.503783793;
const float H0 = 3.329782515;
const float PHI = 0.659210858;
const float FACING = 3.141592654;
const float SPEED = 1.000;
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
void main(){
  computeSun();
  jit = hash(gl_FragCoord.xy) * 6.2831;
  vec2 uv = gl_FragCoord.xy / res.y;                   // wall coords, height = 1
  uv.x -= 0.5 * res.x / res.y;                          // centre x
  // base wall: plaster with gentle grain and vignette
  float grain = fbm(uv * 6.0) * 0.06 + noise(uv * 400.0) * 0.025;
  vec3 wall = vec3(0.47, 0.38, 0.30) * (0.92 + grain);
  float vig = 1.0 - 0.35 * length((gl_FragCoord.xy / res - 0.5) * vec2(1.1, 1.3));

  // ambient sky light: dim blue at night, warm grey by day
  float day = smoothstep(-0.12, 0.08, sunAlt);
  vec3 amb = mix(vec3(0.18, 0.20, 0.30), vec3(0.62, 0.58, 0.55), day);
  vec3 col = wall * amb * vig;

  // window plane is tilted against the wall (like a window in a side wall seen at an angle),
  // which turns the patch into a skewed parallelogram instead of a flat rectangle
  vec3 n = vec3(sin(TILT), 0.0, cos(TILT));
  vec3 ua = vec3(cos(TILT), 0.0, -sin(TILT));
  float ns = dot(n, sun);
  if (ns > 0.03 && sunAlt > -0.02) {
    vec3 p = vec3(uv, 0.0);
    float tw = (D_WIN - dot(n, p)) / ns;     vec3 hw = p + tw * sun;
    float tp = (D_PLANT - dot(n, p)) / ns;   vec3 hp = p + tp * sun;
    vec2 q  = vec2(dot(ua, hw), hw.y);
    vec2 qp = vec2(dot(ua, hp), hp.y);
    float pen  = 0.006 + 0.004 * tw;           // penumbra grows with distance travelled from the opening
    float lit = softAperture(q, pen);
    if (plant > 0.5) lit *= 1.0 - 0.8 * softFoliage(qp, 0.006 + 0.012 * tp);
    float inc = clamp(sun.z, 0.0, 1.0);        // grazing light on the wall is weaker
    float strength = smoothstep(-0.02, 0.10, sunAlt) * (0.45 + 0.55 * inc);
    vec3 sc = sunColor(sunAlt);
    float glow = softAperture(q, pen * 10.0) * 0.22;
    col += wall * sc * (lit * 1.05 + glow) * strength;
  }
  // gentle filmic curve
  col = col / (col + 0.55) * 1.45;
  gl_FragColor = vec4(pow(col, vec3(0.95)), 1.0);
}