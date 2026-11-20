// shared scene code for the native renderer. Generated from index.html, then edited by hand.
precision highp float;
precision highp int;
uniform float time;
uniform int style;      // 0 window 1 blinds 2 lattice 3 georgian 4 arch 5 shutters 6 sheer
uniform int plantType;  // 0 none 1 ficus 2 monstera 3 palm 4 olive
uniform int caustic;    // 0 none 1 pool 2 glass 3 both
uniform float tilt;     // window plane angle against the wall, radians
uniform vec3 sun;       // toward the sun, room frame
uniform float sunAlt;
uniform float haze;

const float D_WIN = 0.55;
const float D_PLANT = 1.05;
const float D_BACK = 2.1;
const float Y_WATER = -0.9;
const float PI = 3.14159265;
const vec2 WC = vec2(0.15, 0.62);
const vec2 WH = vec2(0.42, 0.55);
// texture domains: window plane (aperture, pool caustics) and plant plane (leaf masks)
const vec2 AP_MIN = vec2(-0.45, -0.05);
const vec2 AP_SIZE = vec2(1.2, 1.35);
const vec2 FOL_MIN = vec2(-1.25, -0.65);
const vec2 FOL_SIZE = vec2(2.9, 2.7);
const float TILE = 2.0;  // plaster tile size in wall units

float hash(vec2 p){ return fract(sin(dot(p, vec2(127.1,311.7))) * 43758.5453); }
float noise(vec2 p){ vec2 i=floor(p), f=fract(p); f=f*f*(3.-2.*f);
  return mix(mix(hash(i),hash(i+vec2(1,0)),f.x), mix(hash(i+vec2(0,1)),hash(i+vec2(1,1)),f.x), f.y); }
float fbm(vec2 p){ float a=.5, s=0.; for(int i=0;i<4;i++){ s+=a*noise(p); p*=2.03; a*=.5; } return s; }
mat2 rot(float a){ float c=cos(a), s=sin(a); return mat2(c,-s,s,c); }   // rot(a)*v projects v onto direction a
float sdSeg(vec2 p, vec2 a, vec2 b){ vec2 pa=p-a, ba=b-a; float h=clamp(dot(pa,ba)/dot(ba,ba),0.,1.); return length(pa-ba*h); }

// ---------------- window apertures (window-plane coords), 1 = open ----------------
float inBox(vec2 l, vec2 h){ vec2 d = abs(l) - h; return step(max(d.x, d.y), 0.0); }
float rail(float x, float w){ return step(w, abs(x)); }   // 0 on a bar of half-width w
float sbox(vec2 l, vec2 h, float r){ vec2 a = smoothstep(-h-r, -h+r, l) * (1.0 - smoothstep(h-r, h+r, l)); return a.x*a.y; }

float curtainT(vec2 l){   // sheer curtain over part of the window, transmittance
  float edge = -0.02 + 0.05*sin(l.y*2.0 + time*0.35) + 0.03*sin(time*0.21);
  float cov = smoothstep(edge-0.015, edge+0.015, l.x);
  float folds = 0.5+0.5*sin(l.x*48.0 + 1.6*sin(l.y*2.3 + time*0.4) + time*0.25);
  float folds2 = 0.5+0.5*sin(l.x*19.0 - time*0.17 + l.y*1.4);
  float T = 0.18 + 0.42*folds*folds2 + 0.05*noise(l*vec2(40.0,3.0));
  return mix(1.0, T, cov);
}

float aperture(vec2 q){
  vec2 l = q - WC;
  l += (vec2(noise(q*6.0), noise(q*6.0+7.3)) - 0.5) * 0.007;   // old glass wobble
  float o;
  if (style == 4) {                      // arched window with fan muntins
    float r = WH.x, sy = WH.y - r;
    vec2 a = l - vec2(0.0, sy);
    o = max(inBox(l - vec2(0.0, (sy - WH.y)*0.5), vec2(WH.x, (sy + WH.y)*0.5)), step(length(a), r)*step(0.0, a.y));
    o *= rail(l.x, 0.014) * rail(l.y - sy, 0.014) * rail(l.y - (sy - WH.y)*0.5, 0.010);
    float fan = rail(dot(a, vec2(-sin(0.8), cos(0.8))), 0.009) * rail(dot(a, vec2(-sin(2.34), cos(2.34))), 0.009) * rail(length(a) - r*0.38, 0.009);
    o *= mix(1.0, fan, step(0.0, a.y));
    return o;
  }
  o = inBox(l, WH);
  if (style == 0) {                      // four-pane casement
    o *= rail(l.x, 0.018) * rail(l.y - 0.1, 0.018);
  } else if (style == 1) {               // venetian blinds, raised a little, with ladder cords
    float s = fract(l.y*11.0 + l.x*0.15);
    float slats = smoothstep(0.30, 0.36, s) * (1.0 - smoothstep(0.92, 0.98, s));
    float bot = -WH.y + 0.17;
    float cov = step(bot, l.y);
    o *= mix(1.0, slats * rail(l.x + 0.22, 0.003) * rail(l.x - 0.24, 0.003), cov) * rail(l.y - bot, 0.012);
  } else if (style == 2) {               // diamond lattice
    vec2 r = mat2(0.7071,-0.7071,0.7071,0.7071) * l * 9.0;
    vec2 g = abs(fract(r) - 0.5);
    o *= step(0.07, min(g.x, g.y)) * rail(l.x, 0.02);
  } else if (style == 3) {               // georgian six-over-six sash
    o *= rail(l.y, 0.022) * rail(abs(l.x) - WH.x/3.0, 0.010) * rail(abs(l.y) - WH.y*0.5, 0.010);
  } else if (style == 5) {               // plantation shutters, louvres half open
    float s = fract(l.y*7.0 - l.x*0.05);
    float louv = smoothstep(0.52, 0.58, s) * (1.0 - smoothstep(0.86, 0.92, s));
    o *= louv * rail(l.x, 0.022) * rail(l.y + 0.06, 0.03) * rail(abs(l.x) - WH.x*0.5, 0.007);
  } else {                               // casement with a sheer curtain drawn across part of it
    o *= rail(l.x, 0.016) * rail(l.y - 0.25, 0.014) * curtainT(l);
  }
  return o;
}

// ---------------- plants (plant-plane coords), 1 = blocked ----------------
float vesica(vec2 d){                    // pointed leaf from x=0 (stalk) to x=1 (tip)
  float x = clamp(d.x, 0.0, 1.0);
  float w = 0.17 * pow(sin(PI * pow(x, 0.8)), 0.9);
  float inside = step(0.0, d.x) * step(d.x, 1.0) * step(abs(d.y), w);
  float vein = 1.0 - step(abs(d.y), 0.011) * step(0.08, x) * step(x, 0.8);
  return inside * vein;
}
float ficus(vec2 p){
  p.x += (sin(time*0.6)*0.012 + sin(time*1.7 + p.y*3.0)*0.004) * p.y;
  float b = step(abs(p.x - 0.42 - 0.06*sin(p.y*2.5)), 0.008) * step(p.y, 1.2) * step(0.1, p.y);
  for (int i = 0; i < 10; i++) {
    float fi = float(i), y = 0.22 + fi*0.1, side = mod(fi, 2.0)*2.0 - 1.0;
    vec2 base = vec2(0.42 + 0.06*sin(y*2.5), y);
    float th = PI*0.5 - side*(0.85 + 0.35*hash(vec2(fi, 2.0))) + sin(time*(0.9 + fi*0.13) + fi)*0.06;
    b = max(b, vesica(rot(th)*(p - base) / (0.27 + 0.09*hash(vec2(fi, 3.0)))));
  }
  return b;
}
float monLeaf(vec2 p, vec2 base, float ang, float sz){
  vec2 d = rot(ang)*(p - base)/sz;       // x along the leaf, y across
  float x = d.x, ax = abs(d.y);
  float body = step(length(vec2((x - 0.5)/0.52, d.y/0.47)), 1.0);
  float notch = step(x, 0.12) * step(ax, (0.13 - x)*0.9);
  float t = (x - 0.55*ax)*6.0;
  float slit = step(0.2, ax) * step(abs(fract(t) - 0.5), 0.07 + 0.3*(ax - 0.2));
  float g = fract(t + 0.5) - 0.5;
  float hole = step(length(vec2((ax - 0.12)/0.04, g/0.13)), 1.0) * step(0.25, x) * step(x, 0.85);
  return body * (1.0 - notch) * (1.0 - slit) * (1.0 - hole);
}
float monstera(vec2 p){
  p.x += (sin(time*0.5)*0.01 + sin(time*1.3 + p.y*2.0)*0.003) * p.y;
  vec2 root = vec2(0.42, -0.45);
  float b = 0.0;
  for (int i = 0; i < 4; i++) {
    float fi = float(i);
    vec2 base = i==0 ? vec2(0.08, 0.78) : i==1 ? vec2(0.36, 0.98) : i==2 ? vec2(0.16, 0.36) : vec2(0.48, 0.5);
    float ang = i==0 ? 2.35 : i==1 ? 0.95 : i==2 ? 3.45 : -0.15;
    float sz = i==0 ? 0.42 : i==1 ? 0.4 : i==2 ? 0.36 : 0.4;
    ang += sin(time*0.8 + fi*1.7)*0.035;
    b = max(b, monLeaf(p, base, ang, sz));
    b = max(b, step(sdSeg(p, root, base), 0.008));
  }
  return b;
}
float frond(vec2 p, vec2 base, float ang, float len, float curl){
  vec2 d = rot(ang)*(p - base);
  float u = d.x, v = d.y + curl*u*u;
  float tu = clamp(u/len, 0.0, 1.0), inR = step(0.0, u) * step(u, len), av = abs(v);
  float rach = step(av, 0.005*(1.0 - tu) + 0.0015) * inR;
  float L = 0.24*pow(sin(PI*min(tu*1.08, 1.0)), 0.6) + 0.012;
  float sp = 0.06;
  float w = (u - av*0.8 - av*av*1.2)/sp + step(0.0, v)*0.5;
  float fw = abs(fract(w) - 0.5)*sp;
  float lf = step(fw, 0.017*(1.0 - av/L) + 0.003) * step(av, L) * inR;
  return max(rach, lf);
}
float palm(vec2 p){
  vec2 base = vec2(0.22, 1.42);
  float s = sin(time*0.45)*0.03 + sin(time*1.1)*0.01;
  float b = frond(p, base, -0.55 + s, 1.25, 0.32);
  b = max(b, frond(p, base, -1.15 + s*0.8, 1.35, 0.2));
  b = max(b, frond(p, base, -2.05 + s*1.2, 1.2, -0.25));
  return b;
}
float olBranch(vec2 p, vec2 base, float ang, float len, float bend, float seed){
  vec2 d = rot(ang)*(p - base);
  float u = d.x, v = d.y - bend*u*u;
  float b = step(abs(v), 0.0045*(1.0 - 0.6*u/len)) * step(0.0, u) * step(u, len);
  float sp = 0.055, ci = floor(u/sp);
  for (int k = -2; k <= 1; k++) {
    float i = ci + float(k), uc = (i + 0.5)*sp;
    float ok = step(0.0, uc) * step(uc, len);
    float side = mod(i, 2.0)*2.0 - 1.0;
    float a = side*(0.55 + 0.35*hash(vec2(i, seed))) + sin(time*1.1 + i*0.7 + seed)*0.08;
    vec2 ld = rot(a)*vec2(u - uc, v);
    float Ll = 0.12 + 0.035*hash(vec2(seed, i));
    float x = ld.x/Ll;
    b = max(b, ok * step(0.0, x) * step(x, 1.0) * step(abs(ld.y), 0.14*Ll*sin(PI*clamp(x, 0.0, 1.0))));
  }
  return b;
}
float olive(vec2 p){
  float s = sin(time*0.4)*0.02;
  float b = olBranch(p, vec2(-0.45, 1.45), -0.55 + s, 1.3, 0.12, 1.0);
  b = max(b, olBranch(p, vec2(-0.4, 0.95), -0.18 + s*1.3, 1.05, 0.18, 2.0));
  b = max(b, olBranch(p, vec2(1.0, 1.55), -2.45 - s, 0.95, -0.15, 3.0));
  return b;
}
float foliage(vec2 p){
  if (plantType == 1) return ficus(p);
  if (plantType == 2) return monstera(p);
  if (plantType == 3) return palm(p);
  return olive(p);
}
vec3 leafTint(){                        // light that gets through a leaf (green, faint)
  if (plantType == 4) return vec3(0.035, 0.042, 0.026);
  if (plantType == 3) return vec3(0.03, 0.05, 0.014);
  return vec3(0.025, 0.048, 0.012);
}

// ---------------- caustics ----------------
float vor(vec2 p, float t){             // distance between the two nearest moving cells: 0 on cell borders
  vec2 i = floor(p), f = fract(p);
  float d1 = 8.0, d2 = 8.0;
  for (int y = -1; y <= 1; y++) for (int x = -1; x <= 1; x++) {
    vec2 g = vec2(float(x), float(y));
    vec2 h = vec2(hash(i + g), hash(i + g + 19.7));
    vec2 o = 0.5 + 0.42*sin(t*(0.6 + 0.5*h) + 6.2831*h);
    float d = length(g + o - f);
    if (d < d1) { d2 = d1; d1 = d; } else if (d < d2) { d2 = d; }
  }
  return d2 - d1;
}
float poolCaustic(vec2 w, float blur){  // bright wavy network from a rippling pool surface, w in water-plane coords
  float t = time;
  w += 0.75*vec2(noise(w*0.55 + t*0.1), noise(w*0.55 - t*0.09 + 4.0)) - 0.375
     + 0.3*vec2(noise(w*1.7 - t*0.22), noise(w*1.7 + 9.0 + t*0.19)) - 0.15;   // curve the cell borders
  float a = vor(w*1.25, t*0.8), b = vor(w*1.8 + 7.0, t*1.05);
  float lw = 0.045 + blur;
  float la = exp(-a*a/(lw*lw)), lb = exp(-b*b/(lw*lw*1.6));
  float bright = 0.45 + 1.1*noise(w*0.8 + t*0.13);                 // some strands brighter than others
  return (0.75*la + 0.45*lb + 1.6*la*lb) * bright * (0.055/lw);    // knots where strands cross
}
// a round glass carafe of water on the sill: soft shadow, dark refracting rim,
// a focused bright spot with a rainbow fringe, and a bright arc at its foot. l in window-plane coords from WC
vec3 carafe(vec2 l, float pen){
  vec2 v = l - vec2(0.2, -WH.y + 0.105);
  float body = length(v) - 0.1;
  vec2 nq = v - vec2(0.0, 0.16);
  float neckW = 0.026 + 0.03*smoothstep(0.0, -0.09, nq.y) + 0.008*smoothstep(0.07, 0.085, nq.y);
  float neck = max(abs(nq.x) - neckW, abs(nq.y) - 0.09);
  float sd = min(body, neck);
  sd = max(sd, -0.098 - v.y);           // flat base
  float w = pen + 0.002;
  float inside = 1.0 - smoothstep(-w, w, sd);
  if (inside <= 0.0) return vec3(1.0);
  float water = 1.0 - smoothstep(0.028 - w, 0.028 + w, v.y);
  float ring = smoothstep(-0.045, -0.008, sd);           // edge refracts light away
  float streak = 0.5 + 0.5*sin(v.x*90.0 + 6.0*sin(v.y*30.0));
  vec3 T = vec3(mix(0.26, 0.17, water)) * (1.0 - 0.85*ring) * (0.75 + 0.5*streak*(1.0 - ring));
  vec3 spot = vec3(0.0);
  for (int k = 0; k < 3; k++) {                          // focus, slightly different size per colour
    float r = 0.026*(1.0 + 0.3*float(k - 1));
    float d = length((v - vec2(0.004, -0.024))*vec2(1.35, 0.6));
    float s = 1.0 - smoothstep(r - w*1.5, r + w*1.5, d);
    if (k == 0) spot.r = s; else if (k == 1) spot.g = s; else spot.b = s;
  }
  float arc = exp(-pow((length(v - vec2(0.0, 0.05)) - 0.115)/(0.006 + w), 2.0)) * smoothstep(-0.02, -0.07, v.y) * step(sd, 0.0);
  T += water*(spot*vec3(3.4, 3.2, 3.0) + arc*2.2*vec3(1.0, 0.95, 0.85));
  return mix(vec3(1.0), T, inside);
}

vec3 sunColor(float a){                 // linear colour of direct sun by altitude (radians)
  float d = a*57.2958;
  vec3 c0 = vec3(1.0, 0.30, 0.10), c1 = vec3(1.0, 0.56, 0.26), c2 = vec3(1.0, 0.82, 0.62), c3 = vec3(1.0, 0.94, 0.86);
  if (d < 5.0) return mix(c0, c1, d/5.0);
  if (d < 18.0) return mix(c1, c2, (d - 5.0)/13.0);
  return mix(c2, c3, clamp((d - 18.0)/27.0, 0.0, 1.0));
}

// tileable plaster: the noise lattice wraps every TILE wall units, so it bakes into one repeating texture
float noiseP(vec2 p, float per){ vec2 i=floor(p), f=fract(p); f=f*f*(3.-2.*f);
  vec2 a=mod(i,per), b=mod(i+1.,per);
  return mix(mix(hash(a),hash(vec2(b.x,a.y)),f.x), mix(hash(vec2(a.x,b.y)),hash(b),f.x), f.y); }
float fbmP(vec2 p, float per){ float a=.5, s=0.; for(int i=0;i<4;i++){ s+=a*noiseP(p,per); p*=2.; per*=2.; a*=.5; } return s; }
float plasterP(vec2 x){ return fbmP(x*7.0, 7.0*TILE)*0.55 + fbmP(x*38.0, 38.0*TILE)*0.3 + noiseP(x*170.0, 170.0*TILE)*0.15; }

float motes(vec2 uv){
  float m = 0.0;
  for (int k = 0; k < 2; k++){
    float fk = float(k);
    vec2 g = uv*(16.0 + fk*12.0) + vec2(time*0.06, -time*(0.03 + 0.02*fk)) + vec2(sin(time*0.3 + fk), cos(time*0.23))*0.5;
    vec2 id = floor(g), f = fract(g);
    float h = hash(id + fk*7.0);
    vec2 c = vec2(hash(id + 1.3), hash(id + 2.7))*0.6 + 0.2 + 0.15*vec2(sin(time*0.7 + h*20.0), cos(time*0.6 + h*13.0));
    m += step(0.86, h) * smoothstep(0.07, 0.0, length(f - c)) * (0.55 + 0.45*sin(time*1.7 + h*40.0));
  }
  return m;
}

vec3 aces(vec3 x){ return clamp((x*(2.51*x + 0.03))/(x*(2.43*x + 0.59) + 0.14), 0.0, 1.0); }

