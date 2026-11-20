// shared scene code for the native renderer. Generated from index.html, then edited by hand.
precision highp float;
precision highp int;
uniform float time;
uniform int style;      // 0 window 1 blinds 2 lattice 3 georgian 4 arch 5 shutters 6 sheer 7 porthole 8 stained 9 slats 10 jali
                        // 11 rose 12 tracery 13 palladian 14 shoji 15 moorish 16 prairie 17 moon 18 persian 19 nouveau
uniform int plantType;  // 0 none 1 ficus 2 monstera 3 palm 4 olive 5 fern 6 pothos 7 bird of paradise 8 string of pearls 9 fiddle-leaf fig
uniform int treeType;   // the tree outside: 0 none 1 oak 2 birch 3 willow 4 pine
uniform int caustic;    // 0 none 1 pool 2 glass 3 both
uniform float tilt;     // window plane angle against the wall, radians
uniform vec3 sun;       // toward the sun, room frame
uniform float sunAlt;
uniform float haze;
uniform float rain;      // 0..1 drops on the glass
uniform float gust;      // 0..1 extra wind, kicked by desktop activity
uniform vec4 cat;        // x position on the sill, pose (0 walk, 1 sit), leg phase, direction (+1/-1); direction 0 = no cat
uniform float life;      // 0..1 tiny life: a spider in the window corner, dandelion seeds drifting in
uniform float shelf;     // 0..1 a shelf with a glass of water and a little mirror
uniform float prism;     // 0..1 a crystal suncatcher hanging in the window
uniform vec4 passing;    // something outside: x kind (1 birds, 2 car at night), y progress 0..1, z seed, w direction
uniform vec4 season;     // weights of spring, summer, autumn, winter
// driven by signals (see signals.map)
uniform vec4 flock[6];   // birds: x, y, how perched (0 flying outside, 1 on the sill), wing beat or head dip
uniform int flockN;
uniform float dust;      // 0..1 more dust in the light shaft
uniform float frostSig;  // 0..1 frost on the glass, any season
uniform float spinPh;    // extra turn of the suncatcher
uniform int lamp;        // 0 none 1 neon 2 sodium 3 halide 4 blurple 5 full-spectrum 6 t5 7-10 flames 11-13 projectors
uniform float lampK;     // 0..1 how far on it is
uniform float heat;      // 0..1 warm air rising off a radiator under the window
uniform vec3 boltU;      // lightning: seconds since the strike (negative: none), side of the sky, seed

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
// the stretch of wall the sun-fade map covers, wider than any view pans
// the tree outside, on a plane well beyond the glass, and the part of it the patch can see
const float D_TREE = 4.0;
const vec2 TREE_MIN = vec2(-1.45, -0.98);
const vec2 TREE_SIZE = vec2(3.2, 3.2);
const vec2 FADE_MIN = vec2(-3.2, -1.4);
const vec2 FADE_SIZE = vec2(6.4, 3.9);
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
  float edge = -0.02 + 0.05*sin(l.y*2.0 + time*0.35) + 0.03*sin(time*0.21) + gust*(0.07*sin(l.y*3.0 + time*2.3) + 0.04*sin(time*3.7));
  float cov = smoothstep(edge-0.015, edge+0.015, l.x);
  float folds = 0.5+0.5*sin(l.x*48.0 + (1.6 + 2.0*gust)*sin(l.y*2.3 + time*(0.4 + 1.6*gust)) + time*0.25);
  float folds2 = 0.5+0.5*sin(l.x*19.0 - time*0.17 + l.y*1.4);
  float T = 0.18 + 0.42*folds*folds2 + 0.05*noise(l*vec2(40.0,3.0));
  return mix(1.0, T, cov);
}

// ---------------- rain on the glass: each drop is a little lens, so its shadow has a dark rim and a bright core
float dropT(vec2 d, float r){
  float x = length(d)/max(r, 1e-4);
  if (x >= 1.0) return 1.0;
  return mix(1.0, 0.06, smoothstep(0.35, 0.9, x)) + 0.9*(1.0 - smoothstep(0.0, 0.4, x));
}
float rainT(vec2 l){
  if (rain <= 0.0) return 1.0;
  float T = 1.0;
  // beads that land, sit, and dry
  for (int k = 0; k < 2; k++) {
    float sc = k == 0 ? 11.0 : 19.0;
    vec2 g = l*sc + float(k)*13.1, id = floor(g), f = fract(g) - 0.5;
    vec2 h = vec2(hash(id + 1.7), hash(id + 5.3));
    float life = fract(time*0.045 + hash(id + 9.1)*7.0);
    float r = (0.12 + 0.2*h.y) * smoothstep(0.0, 0.03, life) * (1.0 - smoothstep(0.55, 1.0, life)) * step(hash(id + 2.9), rain*0.75);
    T *= dropT(f - (h - 0.5)*0.5, r);
  }
  // drops that slide down and leave a wet trail of tiny beads
  for (int k = 0; k < 2; k++) {
    float w = k == 0 ? 0.075 : 0.11;
    float cx = floor(l.x/w + float(k)*0.37);
    float h1 = hash(vec2(cx, 3.0 + float(k))), h2 = hash(vec2(cx, 7.0 + float(k)));
    if (h2 > rain) continue;
    float span = 2.0*WH.y + 0.35;
    float y = WH.y + 0.15 - fract(time*(0.035 + 0.06*h1) + h2*5.0)*span;
    float x = (cx + 0.5 - float(k)*0.37)*w + (h1 - 0.5)*0.4*w + sin(y*23.0 + h1*9.0)*0.1*w;
    vec2 d = l - vec2(x + sin(l.y*31.0 + h2*4.0)*0.003, y);
    T *= dropT(d*vec2(1.0, 0.72), 0.02 + 0.012*h1);
    float above = d.y;
    float trail = step(0.0, above) * smoothstep(0.35, 0.0, above) * (1.0 - smoothstep(0.003, 0.008, abs(d.x)));
    T *= 1.0 - 0.55*trail;
    vec2 tb = vec2(d.x, (fract(above*30.0) - 0.5)/30.0);
    T *= mix(1.0, dropT(tb, 0.007), step(0.0, above)*smoothstep(0.3, 0.02, above)*step(0.5, hash(vec2(cx, floor(above*30.0)))));
  }
  return T;
}

// ---------------- the cat that sometimes walks along the sill (window plane), 0 = blocked
float sdEll(vec2 p, vec2 r){ return (length(p/r) - 1.0)*min(r.x, r.y); }
float sdCap(vec2 p, vec2 a, vec2 b, float r){ return sdSeg(p, a, b) - r; }
float sdTri(vec2 p, vec2 a, vec2 b, vec2 c){
  vec2 e0 = b-a, e1 = c-b, e2 = a-c, v0 = p-a, v1 = p-b, v2 = p-c;
  vec2 pq0 = v0 - e0*clamp(dot(v0,e0)/dot(e0,e0), 0.0, 1.0);
  vec2 pq1 = v1 - e1*clamp(dot(v1,e1)/dot(e1,e1), 0.0, 1.0);
  vec2 pq2 = v2 - e2*clamp(dot(v2,e2)/dot(e2,e2), 0.0, 1.0);
  float s = sign(e0.x*e2.y - e0.y*e2.x);
  vec2 d = min(min(vec2(dot(pq0,pq0), s*(v0.x*e0.y-v0.y*e0.x)), vec2(dot(pq1,pq1), s*(v1.x*e1.y-v1.y*e1.x))), vec2(dot(pq2,pq2), s*(v2.x*e2.y-v2.y*e2.x)));
  return -sqrt(d.x)*sign(d.y);
}
float catHead(vec2 p, vec2 hc){
  float d = length(p - hc) - 0.04;
  d = min(d, sdEll(p - hc - vec2(0.03, -0.012), vec2(0.026, 0.019)));
  d = min(d, sdTri(p, hc + vec2(-0.034, 0.018), hc + vec2(-0.012, 0.034), hc + vec2(-0.03, 0.072)));
  d = min(d, sdTri(p, hc + vec2(0.0, 0.034), hc + vec2(0.024, 0.02), hc + vec2(0.016, 0.07)));
  return d;
}
float catTail(vec2 p, vec2 base, float a0, float bend, float segLen){
  float d = 1e3; vec2 a = base; float ang = a0;
  for (int i = 0; i < 6; i++) {
    vec2 b = a + segLen*vec2(cos(ang), sin(ang));
    d = min(d, sdCap(p, a, b, 0.011 - 0.0008*float(i)));
    a = b; ang += bend;
  }
  return d;
}
float catSD(vec2 p){
  float ph = cat.z, swish = sin(time*1.7)*0.12 + sin(time*0.9)*0.08;
  // walking
  float bob = 0.004*sin(ph*2.0);
  vec2 bc = vec2(0.0, 0.11 + bob);
  float w = sdEll(p - bc, vec2(0.118, 0.048));
  w = min(w, sdCap(p, bc + vec2(0.07, 0.015), vec2(0.12, 0.155 + bob), 0.03));
  w = min(w, catHead(p, vec2(0.135, 0.17 + bob)));
  for (int k = 0; k < 4; k++) {
    float fk = float(k), side = k < 2 ? 1.0 : -1.0;
    float s = sin(ph + fk*1.6 + (k < 2 ? 0.0 : 3.14159));
    vec2 hip = vec2(side*0.075 + (mod(fk, 2.0) - 0.5)*0.01, 0.1 + bob);
    vec2 foot = vec2(hip.x + 0.028*s, max(0.0, -s)*0.016);
    w = min(w, sdCap(p, hip, foot, 0.0115));
  }
  w = min(w, catTail(p, bc + vec2(-0.105, 0.012), 1.9 + swish, -0.22, 0.034));
  // sitting, facing the room a little, tail curled along the sill
  vec2 sb = vec2(-0.005, 0.085);
  float s = sdEll(p - sb, vec2(0.062, 0.088));
  s = min(s, sdEll(p - vec2(-0.03, 0.045), vec2(0.072, 0.047)));
  s = min(s, sdCap(p, vec2(0.03, 0.11), vec2(0.04, 0.004), 0.012));
  s = min(s, sdCap(p, vec2(0.015, 0.11), vec2(0.022, 0.004), 0.011));
  s = min(s, catHead(p, vec2(0.012, 0.205)));
  s = min(s, catTail(p, vec2(-0.08, 0.012), -0.05, 0.08 + swish*0.6, 0.032));
  return mix(w, s, smoothstep(0.0, 1.0, cat.y));
}
float catT(vec2 l){
  if (cat.w == 0.0) return 1.0;
  vec2 p = vec2((l.x - cat.x)*cat.w, l.y + WH.y);
  if (abs(p.x) > 0.25 || p.y > 0.3) return 1.0;
  return smoothstep(-0.002, 0.002, catSD(p));
}


// ---------------- pretty things: suncatcher, winter glass, birds, falling petals and leaves ----------------
const vec2 PRISM_PIVOT = vec2(-0.14, 0.535);
const float PRISM_LEN = 0.3;
float prismSwing(){ return 0.045*sin(time*0.57) + 0.02*sin(time*1.31 + 1.0) + gust*0.22*sin(time*2.4); }
float prismSpin(){ return spinPh + time*0.09 + 0.9*sin(time*0.05) + gust*1.4*sin(time*1.3); }
vec2 prismAt(){ float a = prismSwing(); return PRISM_PIVOT + PRISM_LEN*vec2(sin(a), -cos(a)); }
// light through the hanging crystal and its thread, window coords: glass bends most light away, the facet
// edges are dark, and a bright knot sits where it focuses
float prismT(vec2 l, float pen){
  if (prism <= 0.0) return 1.0;
  vec2 c = prismAt();
  vec2 d = rot(prismSwing())*(l - c)/1.5;
  pen /= 1.5;
  if (abs(d.x) > 0.05 + pen && sdSeg(l, PRISM_PIVOT, c) > 0.01 + pen) return 1.0;
  float th = sdSeg(l, PRISM_PIVOT, c + (PRISM_PIVOT - c)*0.13);
  float T = 1.0 - 0.6*(1.0 - smoothstep(0.0, 0.0012 + pen*1.5, th));
  float y = d.y;
  float w = y > 0.0 ? 0.027*sqrt(max(1.0 - pow(y/0.026, 2.0), 0.0)) : 0.027*max(1.0 + y/0.052, 0.0);
  float e = w - abs(d.x);
  float inside = smoothstep(-pen, pen, e) * smoothstep(-0.052 - pen, -0.052 + pen, y) * smoothstep(0.026 + pen, 0.026 - pen, y);
  float spin = prismSpin(), lines = 1.0;
  for (int k = 0; k < 4; k++) {
    float fx = w*sin(spin + float(k)*0.785);
    lines *= smoothstep(0.0, 0.0018 + pen, abs(d.x - fx));
  }
  lines *= smoothstep(0.0, 0.0016 + pen, abs(y));
  float glass = mix(0.05, 0.32, smoothstep(0.0, 0.008 + pen, e))*mix(0.35, 1.0, lines);
  vec2 f = d - vec2(0.005*sin(spin*2.0), -0.014);
  glass += 2.4*exp(-dot(f, f)/(0.00003 + pen*pen));
  return T*mix(1.0, glass, inside*prism);
}
vec3 spectral(float x){                 // 0 violet .. 1 red
  vec3 b = 1.0 - pow((vec3(x) - vec3(0.84, 0.55, 0.25))/vec3(0.32, 0.27, 0.3), vec3(2.0));
  return max(b, 0.0) + vec3(0.35, 0.0, 0.4)*max(1.0 - x*7.0, 0.0);
}

// winter glass: frost creeping in from the frame, snow banked on the sill outside
vec3 winterT(vec2 l){
  float w = season.w, fw = max(w, frostSig);
  if (fw <= 0.0) return vec3(1.0);
  vec2 dd = abs(l) - WH;
  float e = -(length(max(dd, 0.0)) + min(max(dd.x, dd.y), 0.0));
  float grow = 0.035 + 0.1*w + 0.2*frostSig + 0.05*(fbm(l*9.0) - 0.5);
  float frost = smoothstep(grow, grow*0.35, e);
  float fern = smoothstep(0.42, 0.75, fbm(l*34.0 + 3.0*vec2(fbm(l*7.0), fbm(l*7.0 + 4.1))));
  float T = mix(1.0, 0.42 + 0.45*fern, frost*fw);
  float top = -WH.y + w*(0.035 + 0.025*noise(vec2(l.x*7.0, 1.0)) + 0.01*noise(vec2(l.x*31.0, 3.0)));
  float snow = smoothstep(0.003, -0.003, l.y - top)*step(0.001, w);
  return vec3(T*mix(1.0, 0.1, snow))*vec3(1.0, 1.0, 1.0 + 0.04*frost*w);
}

// birds crossing: a soft V that flaps, in its own plane outside
float birdSD(vec2 p, float flap){
  vec2 q = vec2(abs(p.x), p.y);
  vec2 m = vec2(0.45, 0.1 + 0.35*flap), t = vec2(1.0, flap);
  float wing = min(sdSeg(q, vec2(0.0), m), sdSeg(q, m, t)) - 0.1*(1.0 - 0.7*clamp(q.x, 0.0, 1.0));
  return min(wing, sdEll(p - vec2(0.0, -0.03), vec2(0.16, 0.08)));
}
float birdsT(vec2 qb, float pen){
  if (passing.x != 1.0) return 1.0;
  float T = 1.0, n = 1.0 + floor(passing.z*3.0);
  for (int i = 0; i < 3; i++) {
    float fi = float(i);
    if (fi >= n) break;
    float lag = fi*0.06, s = 0.075*(1.0 - 0.15*fi);
    float x = mix(-1.3, 1.3, passing.y - lag)*passing.w;
    float y = WC.y + 0.25 + (passing.z - 0.5)*0.4 + fi*0.06*sin(fi*2.3) + 0.04*sin(passing.y*7.0 + fi);
    float flap = mix(0.5*sin(time*13.0 + fi*1.9), -0.15, step(0.6, fract(passing.y*2.0 + fi*0.3)));
    float d = birdSD(vec2((qb.x - WC.x - x)*passing.w, qb.y - y)/s, flap)*s;
    T *= 1.0 - 0.88*(1.0 - smoothstep(-pen, pen, d));
  }
  return T;
}

// a small bird sitting on the sill, side on and facing +x; dip 0 head up .. 1 pecking down at the glass
float perchSD(vec2 p, float dip){
  float d = sdEll(p - vec2(0.0, 0.03), vec2(0.034, 0.021));
  vec2 hc = vec2(0.027, 0.05) + dip*vec2(0.014, -0.028);
  d = min(d, length(p - hc) - 0.014);
  d = min(d, sdTri(p, hc + vec2(0.008, 0.006), hc + vec2(0.008, -0.006), hc + vec2(0.024 + 0.004*dip, -0.002 - 0.01*dip)));
  d = min(d, sdTri(p, vec2(-0.024, 0.038), vec2(-0.02, 0.02), vec2(-0.064, 0.022)));
  d = min(d, sdCap(p, vec2(-0.004, 0.012), vec2(-0.007, 0.0), 0.0025));
  d = min(d, sdCap(p, vec2(0.007, 0.012), vec2(0.009, 0.0), 0.0025));
  return d;
}
// birds driven by signals: circling in the plane outside (qb), or perched on the sill at the glass (l)
float flockT(vec2 l, vec2 qb, float penW, float penB){
  float T = 1.0;
  for (int i = 0; i < 6; i++) {
    if (i >= flockN) break;
    vec4 b = flock[i];
    float face = (i % 2 == 0) ? 1.0 : -1.0;
    vec2 p = mix(qb - WC - b.xy, l - vec2(b.x, -WH.y + b.y), b.z);
    float pen = mix(penB, penW, b.z);
    if (dot(p, p) > pow(0.13 + 2.0*pen, 2.0)) continue;   // nowhere near this bird
    p.x *= face;
    float sF = 0.085 - 0.008*float(i % 3), sP = 1.45;
    float d = mix(birdSD(p/sF, b.w)*sF, perchSD(p/sP, b.w)*sP, smoothstep(0.55, 0.9, b.z));
    T *= 1.0 - 0.9*(1.0 - smoothstep(-pen, pen, d));
  }
  return T;
}

// spring petals and autumn leaves drifting down past the window
vec3 fallT(vec2 qf, float pen){
  float sp = season.x, au = season.z;
  if (sp + au <= 0.02) return vec3(1.0);
  float cell = 0.17;
  vec2 g = qf/cell + vec2(sin(time*0.13)*0.6 + gust*2.0*sin(time*0.9), time*0.32);
  vec2 id0 = floor(g);
  vec3 T = vec3(1.0);
  for (int j = -1; j <= 1; j++) for (int i = -1; i <= 1; i++) {
    vec2 id = id0 + vec2(i, j);
    float h = hash(id*1.37 + 11.0);
    if (h > 0.2*sp + 0.13*au) continue;
    float h2 = hash(id + 5.1), h3 = hash(id + 9.7);
    vec2 c = id + 0.5 + 0.3*vec2(sin(time*(0.8 + h2) + h3*20.0), 0.25*cos(time*0.7 + h2*9.0));
    vec2 d = (g - c)*cell;
    d = rot(time*(1.5 + 2.0*h3)*(h2 > 0.5 ? 1.0 : -1.0))*d;
    bool leaf = hash(id + 2.2) < au/(sp + au);
    d.x *= 0.45 + 0.55*abs(sin(time*(1.1 + h3) + h2*7.0));   // tumbling, seen edge-on now and then
    float sd = leaf ? max(sdEll(d, vec2(0.03, 0.016)), -sdEll(d - vec2(0.0, 0.03), vec2(0.03, 0.02))*0.3) : sdEll(d, vec2(0.015, 0.011));
    float m = 1.0 - smoothstep(-pen, pen, sd);
    vec3 through = leaf ? vec3(0.16, 0.1, 0.04) : vec3(0.55, 0.36, 0.44);
    T *= mix(vec3(1.0), through, m);
  }
  return T;
}

// ---------------- stained glass: leaded panes in a few jewel colours, light takes their colour
vec3 stained(vec2 l){
  vec2 g = l*vec2(5.2, 4.6);
  vec2 i = floor(g), f = fract(g);
  float d1 = 8.0, d2 = 8.0; vec2 best = vec2(0.0);
  for (int y = -1; y <= 1; y++) for (int x = -1; x <= 1; x++) {
    vec2 c = vec2(float(x), float(y));
    vec2 o = 0.5 + 0.38*sin(6.2831*vec2(hash(i + c), hash(i + c + 4.2)));
    float d = length(c + o - f);
    if (d < d1) { d2 = d1; d1 = d; best = i + c; } else if (d < d2) d2 = d;
  }
  float lead = smoothstep(0.035, 0.06, d2 - d1);
  float h = hash(best + 11.0);
  vec3 c = h < 0.22 ? vec3(0.95, 0.55, 0.12) : h < 0.4 ? vec3(0.75, 0.1, 0.12) : h < 0.55 ? vec3(0.12, 0.28, 0.8)
         : h < 0.68 ? vec3(0.2, 0.62, 0.3) : h < 0.78 ? vec3(0.95, 0.85, 0.35) : vec3(0.92, 0.9, 0.85);
  // a round rose in the middle
  float rr = length(l - vec2(0.0, 0.12));
  float petal = 0.5 + 0.5*cos(atan(l.y - 0.12, l.x)*8.0);
  if (rr < 0.2) { c = mix(vec3(0.85, 0.15, 0.25), vec3(0.98, 0.7, 0.2), smoothstep(0.05, 0.16, rr*(0.8 + 0.3*petal))); lead = smoothstep(0.004, 0.009, abs(rr - 0.19)) * smoothstep(0.003, 0.007, abs(rr - 0.09)) * mix(1.0, smoothstep(0.0, 0.1, abs(petal - 0.5)), step(0.09, rr)); }
  return c*lead*(0.85 + 0.15*noise(l*40.0));
}

// ---------------- historical windows (styles 11..23): each returns rgb light colour and a = open
float archIn(vec2 p, float hw, float s, float k, float bot){   // jambs to s, then a round (k=0) to equilateral (k=1) head
  float R = hw*(1.0 + k);
  float head = step(length(vec2(abs(p.x) + R - hw, p.y - s)), R) * step(s, p.y);
  return max(head, step(abs(p.x), hw) * step(bot, p.y) * step(p.y, s));
}
float foil(vec2 p, float r, float n){ float a = atan(p.x, p.y); return step(length(p), r*(0.76 + 0.24*abs(cos(0.5*n*a)))); }   // cusped n-foil, a lobe up
float sdBox(vec2 p, vec2 h){ vec2 d = abs(p) - h; return length(max(d, 0.0)) + min(max(d.x, d.y), 0.0); }
vec2 cells(vec2 g, float jit){   // voronoi: x = gap between the two nearest (0 on an edge), y = cell hash
  vec2 i = floor(g), f = fract(g);
  float d1 = 8.0, d2 = 8.0; vec2 best = vec2(0.0);
  for (int y = -1; y <= 1; y++) for (int x = -1; x <= 1; x++) {
    vec2 c = vec2(float(x), float(y));
    vec2 o = 0.5 + jit*sin(6.2831*vec2(hash(i + c), hash(i + c + 4.2)));
    float d = length(c + o - f);
    if (d < d1) { d2 = d1; d1 = d; best = i + c; } else if (d < d2) d2 = d;
  }
  return vec2(d2 - d1, hash(best + 11.0));
}
vec3 jewel(float h){
  return h < 0.2 ? vec3(0.92, 0.18, 0.2) : h < 0.38 ? vec3(1.0, 0.68, 0.16) : h < 0.56 ? vec3(0.22, 0.38, 0.95)
       : h < 0.7 ? vec3(0.18, 0.7, 0.38) : h < 0.84 ? vec3(0.95, 0.45, 0.62) : vec3(0.98, 0.92, 0.6);
}

// Gothic rose (13th c.): twelve lancet petals round a quatrefoil, a ring of foils and roundels, Chartres blues and rubies
vec4 roseW(vec2 l){
  vec2 p = l - vec2(0.0, 0.03);
  float r = length(p), a = atan(p.x, p.y), sec = 2.0*PI/12.0;
  float k = floor(a/sec + 0.5), u = a - k*sec;
  vec2 s = r*vec2(sin(u), cos(u));
  float hw = 0.30*sin(sec*0.5) - 0.012, R = hw*1.6;
  float petal = max(step(abs(s.x), s.y*sin(sec*0.5) - 0.012) * step(0.15, s.y) * step(s.y, 0.30),
                    step(length(vec2(abs(s.x) + R - hw, s.y - 0.30)), R) * step(0.30, s.y));
  float u2 = a - (floor(a/sec) + 0.5)*sec;
  vec2 s2 = r*vec2(sin(u2), cos(u2));
  float small = step(length(s2 - vec2(0.0, 0.395)), 0.024);
  float outer = foil(s - vec2(0.0, 0.455), 0.042, 4.0);
  float centre = foil(p, 0.115, 4.0);
  float o = max(max(centre, petal), max(small, outer));
  vec3 blue = vec3(0.2, 0.36, 0.95), ruby = vec3(0.9, 0.16, 0.22), gold = vec3(1.0, 0.74, 0.22);
  vec3 c = mod(k, 2.0) < 0.5 ? blue : ruby;
  c = mix(c, mod(k, 2.0) < 0.5 ? ruby : gold, step(s.y, 0.225));
  float lead = smoothstep(0.003, 0.006, abs(s.y - 0.225)) * step(0.15, s.y) + step(s.y, 0.15);
  if (centre > 0.5) { c = mix(gold, ruby, smoothstep(0.035, 0.06, r)); lead = smoothstep(0.003, 0.006, abs(r - 0.05)); }
  if (outer > 0.5) { float rr = length(s - vec2(0.0, 0.455)); c = mix(gold, blue, smoothstep(0.012, 0.02, rr)); lead = 1.0; }
  if (small > 0.5) { c = ruby; lead = 1.0; }
  vec2 v = cells(l*26.0, 0.38);
  lead *= smoothstep(0.03, 0.07, v.x);
  return vec4(c*(0.8 + 0.35*v.y), o*lead);
}

// Gothic two-light window with bar tracery (c. 1260): cusped lancets, a sexfoil oculus, trefoils, diamond quarries
vec4 traceryW(vec2 l){
  float o = archIn(vec2(abs(l.x) - 0.19, l.y), 0.165, 0.02, 0.6, -0.55);
  o = max(o, foil(l - vec2(0.0, 0.43), 0.125, 6.0));
  o = max(o, foil(vec2(abs(l.x) - 0.27, l.y - 0.35), 0.05, 3.0));
  vec2 g = mat2(0.7071, -0.7071, 0.7071, 0.7071) * (l*vec2(13.0, 8.0));
  vec2 d = abs(fract(g) - 0.5);
  return vec4(0.93, 0.96, 0.87, o*smoothstep(0.02, 0.045, min(d.x, d.y)));
}

// Palladian (Venetian) window, 18th c.: a round-headed centre light with a fanlight, flat side lights, sash bars
vec4 palladianW(vec2 l){
  float bot = -0.55, s = 0.17, top = 0.09, sp = 0.19;
  float hb = rail((fract((l.y - bot)/sp + 0.5) - 0.5)*sp, 0.006);
  vec2 a = l - vec2(0.0, s);
  float ang = atan(a.y, a.x), st = PI/6.0;
  float fan = rail(length(a)*sin(ang - st*floor(ang/st + 0.5)), 0.006) * rail(length(a) - 0.09, 0.006) * step(0.035, length(a));
  float cen = archIn(l, 0.2, s, 0.0, bot) * mix(hb*rail(abs(l.x) - 0.067, 0.006), fan, step(s, l.y)) * rail(l.y - s, 0.009);
  vec2 sd = vec2(abs(l.x) - 0.345, l.y - (bot + top)*0.5);
  float side = inBox(sd, vec2(0.095, (top - bot)*0.5)) * rail(sd.x, 0.006) * hb;
  return vec4(1.0, 1.0, 1.0, max(cen, side));
}

// Japanese shoji (Edo): paper on a kumiko grid, two sliding panels, one pushed back and overlapping
float shojiPanel(vec2 l, float x0, float x1){
  if (l.x < x0 || l.x > x1) return 1.0;
  float w = x1 - x0, u = l.x - x0, v = l.y + WH.y, H = 2.0*WH.y;
  float wood = 1.0 - step(0.02, u)*step(u, w - 0.02)*step(0.02, v)*step(v, H - 0.02);
  wood = max(wood, step(v, 0.14));
  float ku = abs(fract(u/(w/3.0) + 0.5) - 0.5)*(w/3.0), kv = abs(fract((v - 0.14)/0.105 + 0.5) - 0.5)*0.105;
  wood = max(wood, step(min(ku, kv), 0.0045));
  return (1.0 - wood) * 0.52 * (0.86 + 0.14*fbm(l*vec2(70.0, 9.0)));
}
vec4 shojiW(vec2 l){
  float o = inBox(l, WH) * shojiPanel(l, -0.07, 0.36) * shojiPanel(l, 0.0, 0.42);
  return vec4(mix(vec3(1.0), vec3(1.0, 0.93, 0.8), step(-0.07, l.x)), o);
}

// Moorish ajimez (Nasrid, 14th c.): twin horseshoe arches on a slender column, a celosia of round holes above
float horseshoe(vec2 p, float r, float w, float s, float bot){
  float jt = s - sqrt(r*r - w*w);
  return max(step(length(p - vec2(0.0, s)), r) * step(jt, p.y), step(abs(p.x), w) * step(bot, p.y) * step(p.y, jt));
}
vec4 ajimezW(vec2 l){
  float o = horseshoe(vec2(abs(l.x) - 0.2, l.y), 0.175, 0.148, 0.12, -0.55);
  vec2 g = (l - vec2(0.0, 0.5))*vec2(22.0, 22.0/0.866);
  float row = floor(g.y);
  vec2 f = vec2(fract(g.x + 0.5*mod(row, 2.0)) - 0.5, (fract(g.y) - 0.5)*0.866);
  o = max(o, inBox(l - vec2(0.0, 0.5), vec2(0.3, 0.085)) * step(length(f), 0.3));
  return vec4(1.0, 1.0, 1.0, o);
}

// Prairie art glass (c. 1908): paired casements, clear glass, straight leading in chevrons, small squares of colour
vec4 prairieW(vec2 l){
  float o = inBox(l, WH) * rail(l.x, 0.016);
  vec2 q = vec2(abs(abs(l.x) - 0.21), l.y);
  float d = sdSeg(q, vec2(0.15, -0.47), vec2(0.15, 0.44));
  d = min(d, sdSeg(q, vec2(0.0, 0.44), vec2(0.15, 0.44)));
  d = min(d, sdSeg(q, vec2(0.0, -0.47), vec2(0.15, -0.47)));
  d = min(d, sdSeg(q, vec2(0.0, 0.36), vec2(0.15, 0.36)));
  d = min(d, sdSeg(q, vec2(0.03, -0.47), vec2(0.03, 0.125)));
  for (int k = 0; k < 3; k++) { float y = 0.16 + 0.07*float(k); d = min(d, sdSeg(q, vec2(0.0, y), vec2(0.15, y - 0.11))); }
  d = min(d, sdSeg(q, vec2(0.03, 0.125), vec2(0.15, -0.1)));
  vec3 c = vec3(1.0);
  float sq1 = sdBox(q - vec2(0.0, 0.40), vec2(0.022)), sq2 = sdBox(q - vec2(0.095, 0.40), vec2(0.016)), sq3 = sdBox(q - vec2(0.09, -0.41), vec2(0.014));
  d = min(d, min(abs(sq1), min(abs(sq2), abs(sq3))));
  if (sq1 < 0.0) c = vec3(1.0, 0.72, 0.2); else if (sq2 < 0.0) c = vec3(0.35, 0.72, 0.4); else if (sq3 < 0.0) c = vec3(0.92, 0.32, 0.22);
  else if (q.y > 0.12 && q.y < 0.37 && q.y > 0.16 - 0.733*q.x && q.y < 0.30 - 0.733*q.x + 0.08) c = vec3(1.0, 0.93, 0.74);
  return vec4(c, o*smoothstep(0.003, 0.0055, d));
}

// Chinese garden moon window (Ming and Qing, Suzhou): a round opening filled with ice-ray lattice
vec4 moonW(vec2 l){
  vec2 p = l - vec2(0.0, 0.05);
  vec2 c = cells(p*6.5 + 3.7, 0.45);
  float rim = smoothstep(0.004, 0.007, abs(length(p) - 0.395));
  return vec4(1.0, 1.0, 1.0, step(length(p), 0.41) * smoothstep(0.055, 0.075, c.x) * rim);
}

// Persian orosi (Qajar, 19th c.): three pointed-arch sashes, a wooden star lattice holding jewel-coloured glass
vec4 orosiW(vec2 l){
  float pw = 0.28, i = floor((l.x + 0.42)/pw);
  float u = l.x - (-0.42 + pw*(i + 0.5));
  float o = step(abs(l.x), 0.42) * archIn(vec2(u, l.y), 0.122, 0.28, 0.7, -0.55);
  vec2 g = vec2(u*9.0 + 0.5, (l.y + 0.55)*9.0), f = fract(g) - 0.5, id = floor(g), sym = vec2(abs(id.x), id.y);
  float m = max(max(abs(f.x), abs(f.y)), (abs(f.x) + abs(f.y))*0.7071);
  float star = step(m, 0.3);
  float wood = max(1.0 - smoothstep(0.018, 0.034, abs(m - 0.3)), smoothstep(0.455, 0.475, max(abs(f.x), abs(f.y))));
  vec3 c = star > 0.5 ? jewel(hash(sym + 3.1)) : jewel(hash(sym*1.3 + 8.7 + vec2(0.0, step(0.0, f.x*f.y))));
  return vec4(c, o*(1.0 - wood));
}

// Art Nouveau (c. 1900): whiplash stems rising to stylised roses, leaves, a curved band of amber across the top
vec4 nouveauW(vec2 l){
  float o = inBox(l, WH);
  vec3 c = vec3(0.97, 0.95, 0.88);
  float d = 1e3;
  for (int k = 0; k < 3; k++) {
    float fk = float(k) - 1.0, fy = 0.24 + 0.08*(1.0 - abs(fk));
    float t = clamp(l.y + 0.55, 0.0, fy + 0.55);
    float sx = fk*0.2 + 0.05*sin(t*5.5 + fk*1.7)*t + 0.06*fk*t;
    if (l.y < fy) d = min(d, abs(l.x - sx)*0.85);
    float tf = fy + 0.55, fx = fk*0.2 + 0.05*sin(tf*5.5 + fk*1.7)*tf + 0.06*fk*tf;
    vec2 pf = l - vec2(fx, fy + 0.05);
    float r = length(pf), a = atan(pf.y, pf.x);
    if (r < 0.075) {
      float sp = r/0.024 + a/6.2831;
      d = min(d, min(fract(sp), 1.0 - fract(sp))*0.024*0.9);
      c = mix(vec3(0.85, 0.22, 0.38), vec3(0.98, 0.55, 0.65), smoothstep(0.0, 0.07, r));
    }
    d = min(d, abs(r - 0.075));
    for (int j = 0; j < 2; j++) {
      float yl = -0.2 + 0.2*float(j) + 0.05*fk, side = (mod(float(j) + fk, 2.0) < 0.5 ? 1.0 : -1.0);
      float tl = yl + 0.55, xl = fk*0.2 + 0.05*sin(tl*5.5 + fk*1.7)*tl + 0.06*fk*tl;
      vec2 pl = rot(side*0.7) * (l - vec2(xl + side*0.055, yl + 0.03));
      float e = sdEll(pl, vec2(0.065, 0.02));
      if (e < 0.0) c = vec3(0.5, 0.8, 0.38);
      d = min(d, abs(e));
    }
  }
  float arc = l.y - (0.43 + 0.07*cos(l.x*7.0));
  if (arc > 0.0) c = vec3(1.0, 0.8, 0.45);
  d = min(d, abs(arc)*0.8);
  d = min(d, abs(l.y + 0.3 + 0.025*sin(l.x*9.0 + 1.0))*0.9);
  return vec4(c, o*smoothstep(0.0035, 0.006, d));
}

// a cusped arch, the Mughal and Rajput opening: jambs up to s, then a head of lobes (with an even count the
// springing lobes meet the jambs; odd leaves a cusp there), raised toward a point; depth is how far cusps bite
float cusped(vec2 p, float hw, float s, float bot, float lobes, float depth){
  float jambs = step(abs(p.x), hw)*step(bot, p.y)*step(p.y, s);
  vec2 h = vec2(p.x, (p.y - s)/1.25);
  return max(jambs, step(s, p.y)*step(length(h), hw*(1.0 - depth + depth*abs(cos(lobes*atan(h.x, h.y))))));
}

// Cairene mashrabiya (Ottoman era, 18th c.): a screen of turned wooden spindles with a bead between each pair
// of joints, set closer below for privacy, and a little hatch left open at eye level
float spindles(vec2 l, float p){   // 1 = wood
  vec2 a = (fract(l/p + 0.5) - 0.5)*p, t = fract(l/p) - 0.5;   // a: across the nearest bars, t: along them
  vec2 w = p*(0.07 + 0.2*exp(-t*t*36.0));                     // bars swell into a bead mid-way
  float bars = max(step(abs(a.x), w.y), step(abs(a.y), w.x));
  return max(bars, step(max(abs(a.x), abs(a.y)), p*0.17));    // a block at each joint
}
vec4 mashrabiyaW(vec2 l){
  float o = inBox(l, vec2(0.44, 0.58));
  float lower = step(l.y, -0.16);
  float wood = spindles(l - vec2(0.0, 0.02), mix(0.088, 0.062, lower));
  wood = max(wood, 1.0 - rail(l.y + 0.16, 0.014));
  // the hatch: a small framed opening at eye level, its shutter slid aside
  vec2 hp = l - vec2(0.0, 0.1);
  float hatch = inBox(hp, vec2(0.085, 0.07));
  wood = mix(wood, 0.0, hatch);
  wood = max(wood, inBox(hp, vec2(0.1, 0.085))*(1.0 - hatch));
  // a frieze of small pointed arches along the top
  vec2 fp = vec2((fract(l.x/0.11) - 0.5)*0.11, l.y - 0.44);
  float frieze = step(0.4, l.y);
  wood = mix(wood, 1.0 - archIn(fp, 0.03, 0.02, 0.8, -0.03)*step(fp.y, 0.13), frieze);
  return vec4(1.0, 1.0, 1.0, o*(1.0 - wood));
}

// Persian girih (Timurid and Safavid, 15th–17th c.): ten-pointed stars whose straps run straight on past
// their tips to meet the neighbours, filling a pointed arch
float girihStraps(vec2 l){
  vec2 a = vec2(0.25, 0.25*1.3764);   // centred lattice, its diagonal at 54 degrees
  vec2 p1 = l - a*floor(l/a + 0.5), p2 = l - a*(floor(l/a) + 0.5);
  vec2 p = dot(p1, p1) < dot(p2, p2) ? p1 : p2;
  // fold into one tenth of the star, a tip on the axis
  float th = mod(atan(p.y, p.x) - PI*0.5 + PI*0.1, PI*0.2) - PI*0.1;
  vec2 q = length(p)*vec2(cos(abs(th)), sin(abs(th)));
  vec2 T = vec2(0.105, 0.0);
  float d = min(abs(dot(q - T, vec2(0.5878, 0.809))), abs(dot(q - T, vec2(-0.5878, 0.809))));
  return d;   // distance to the nearest strap's centre line
}
vec4 girihW(vec2 l){
  float o = archIn(l, 0.4, 0.08, 0.45, -0.58);
  float d = girihStraps(l - vec2(0.0, 0.03));
  return vec4(1.0, 1.0, 1.0, o*smoothstep(0.009, 0.013, d));
}

// Mughal jali (Fatehpur Sikri, 1580s): a marble screen pierced with six-pointed stars on a hexagonal grid,
// small triangles between, set in a cusped arch
float sdHexagram(vec2 p, float r){
  const vec4 k = vec4(-0.5, 0.8660254, 0.5773503, 1.7320508);
  p = abs(p);
  p -= 2.0*min(dot(k.xy, p), 0.0)*k.xy;
  p -= 2.0*min(dot(k.yx, p), 0.0)*k.yx;
  p -= vec2(clamp(p.x, r*k.z, r*k.w), r);
  return length(p)*sign(p.y);
}
vec4 sikriW(vec2 l){
  float o = cusped(l, 0.36, 0.1, -0.58, 4.0, 0.24);
  vec2 s = vec2(0.13, 0.13*1.7320508);
  vec2 p1 = l - s*floor(l/s + 0.5), p2 = l - s*(floor(l/s) + 0.5);
  vec2 p = dot(p1, p1) < dot(p2, p2) ? p1 : p2;
  float star = step(sdHexagram(p, 0.03), 0.0);
  // the small holes sit where three cells meet
  vec2 v = vec2(abs(p.x), abs(p.y));
  float tri = step(length(v - vec2(0.065, 0.0375)), 0.011);
  return vec4(1.0, 1.0, 1.0, o*max(star, tri));
}

// Rajput jharokha (Rajasthan, 17th–18th c.): a balcony window of three trefoil arches on slim columns
// over a low parapet pierced with quatrefoils
vec4 jharokhaW(vec2 l){
  float pw = 0.27, i = clamp(floor((l.x + 0.405)/pw), 0.0, 2.0);
  float u = l.x - (-0.405 + pw*(i + 0.5));
  float o = step(abs(l.x), 0.405)*cusped(vec2(u, l.y), 0.1, 0.13, -0.28, 3.0, 0.3);
  vec2 pp = l - vec2(0.0, -0.43);
  vec2 g = (fract(pp/0.065 + 0.5) - 0.5)*0.065;
  o = max(o, inBox(pp, vec2(0.39, 0.075))*foil(g, 0.025, 4.0));
  return vec4(1.0, 1.0, 1.0, o);
}

vec4 historic(vec2 l){
  if (style == 20) return mashrabiyaW(l);
  if (style == 21) return girihW(l);
  if (style == 22) return sikriW(l);
  if (style == 23) return jharokhaW(l);
  if (style == 11) return roseW(l);
  if (style == 12) return traceryW(l);
  if (style == 13) return palladianW(l);
  if (style == 14) return shojiW(l);
  if (style == 15) return ajimezW(l);
  if (style == 16) return prairieW(l);
  if (style == 17) return moonW(l);
  if (style == 18) return orosiW(l);
  return nouveauW(l);
}

float aperture(vec2 q);
vec3 apertureC(vec2 q){
  vec2 l = q - WC;
  vec3 c = vec3(1.0);
  vec2 lw = l + (vec2(noise(q*6.0), noise(q*6.0+7.3)) - 0.5) * 0.007;
  if (style >= 11) { vec4 h = historic(lw); return h.rgb * h.a * rainT(l) * catT(l) * winterT(l); }
  if (style == 8) c = stained(lw);
  return c * aperture(q) * rainT(l) * catT(l) * winterT(l);
}

float aperture(vec2 q){
  vec2 l = q - WC;
  l += (vec2(noise(q*6.0), noise(q*6.0+7.3)) - 0.5) * 0.007;   // old glass wobble
  float o;
  if (style >= 11) return historic(l).a;
  if (style == 4) {                      // arched window with fan muntins
    float r = WH.x, sy = WH.y - r;
    vec2 a = l - vec2(0.0, sy);
    o = max(inBox(l - vec2(0.0, (sy - WH.y)*0.5), vec2(WH.x, (sy + WH.y)*0.5)), step(length(a), r)*step(0.0, a.y));
    o *= rail(l.x, 0.014) * rail(l.y - sy, 0.014) * rail(l.y - (sy - WH.y)*0.5, 0.010);
    float fan = rail(dot(a, vec2(-sin(0.8), cos(0.8))), 0.009) * rail(dot(a, vec2(-sin(2.34), cos(2.34))), 0.009) * rail(length(a) - r*0.38, 0.009);
    o *= mix(1.0, fan, step(0.0, a.y));
    return o;
  }
  if (style == 7) {                      // ship's porthole: round glass in a thick ring with a hinge bar
    float r = length(l - vec2(0.0, 0.08));
    return (1.0 - smoothstep(0.33, 0.334, r)) * rail(l.y - 0.08, 0.006);
  }
  if (style == 8) {                      // stained glass under a pointed arch
    float r = WH.x;
    vec2 a = vec2(abs(l.x), l.y - (WH.y - r*1.2));
    float arch = step(length(a + vec2(r*0.5, 0.0)), r*1.5) * step(0.0, a.y);
    return max(inBox(l - vec2(0.0, -0.02 - 0.5*r*1.2*0.5), vec2(WH.x, WH.y - r*0.6)), arch) * inBox(l, WH + vec2(0.0, 0.2));
  }
  o = inBox(l, WH);
  if (style == 9) {                      // weathered vertical board slats with uneven gaps
    float bw = 0.105, i = floor(l.x/bw);
    float gap = 0.006 + 0.014*hash(vec2(i, 1.0)), xo = fract(l.x/bw)*bw;
    float knot = step(length((l - vec2((i + 0.4 + 0.2*hash(vec2(i, 5.0)))*bw, (hash(vec2(i, 3.0)) - 0.5)*0.8))*vec2(1.0, 0.5)), 0.012 + 0.008*hash(vec2(i, 9.0)));
    return o * max(step(xo, gap + 0.004*sin(l.y*9.0 + i)), knot*step(0.4, hash(vec2(i, 7.0)))) * rail(l.y - 0.3, 0.025) * rail(l.y + 0.32, 0.025);
  }
  if (style == 10) {                     // carved jali screen: eight-pointed stars and small crosses
    vec2 g = l*7.5, f = fract(g) - 0.5;
    vec2 r1 = abs(f), r2 = abs(mat2(0.7071,-0.7071,0.7071,0.7071)*f);
    float star = step(max(max(r1.x, r1.y), max(r2.x, r2.y)*1.0), 0.27);
    vec2 f2 = fract(g + 0.5) - 0.5;
    float cross_ = step(min(abs(f2.x), abs(f2.y)), 0.035) * step(length(f2), 0.15);
    return o * max(star, cross_) * rail(l.x, 0.012);
  }
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
float gustSway(float seed){ return gust*(0.55*sin(time*2.1 + seed) + 0.3*sin(time*4.7 + seed*1.7) + 0.15*sin(time*8.3 + seed*2.3)); }
float ficus(vec2 p){
  p.x += (sin(time*0.6)*0.012 + sin(time*1.7 + p.y*3.0)*0.004 + gustSway(p.y*2.0)*0.05) * p.y;
  float b = step(abs(p.x - 0.42 - 0.06*sin(p.y*2.5)), 0.008) * step(p.y, 1.2) * step(0.1, p.y);
  for (int i = 0; i < 10; i++) {
    float fi = float(i), y = 0.22 + fi*0.1, side = mod(fi, 2.0)*2.0 - 1.0;
    vec2 base = vec2(0.42 + 0.06*sin(y*2.5), y);
    float th = PI*0.5 - side*(0.85 + 0.35*hash(vec2(fi, 2.0))) + sin(time*(0.9 + fi*0.13) + fi)*0.06 + gustSway(fi*1.3)*0.45;
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
  p.x += (sin(time*0.5)*0.01 + sin(time*1.3 + p.y*2.0)*0.003 + gustSway(p.y)*0.04) * p.y;
  vec2 root = vec2(0.42, -0.45);
  float b = 0.0;
  for (int i = 0; i < 4; i++) {
    float fi = float(i);
    vec2 base = i==0 ? vec2(0.08, 0.78) : i==1 ? vec2(0.36, 0.98) : i==2 ? vec2(0.16, 0.36) : vec2(0.48, 0.5);
    float ang = i==0 ? 2.35 : i==1 ? 0.95 : i==2 ? 3.45 : -0.15;
    float sz = i==0 ? 0.42 : i==1 ? 0.4 : i==2 ? 0.36 : 0.4;
    ang += sin(time*0.8 + fi*1.7)*0.035 + gustSway(fi*2.1)*0.18;
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
  float s = sin(time*0.45)*0.03 + sin(time*1.1)*0.01 + gustSway(0.0)*0.14;
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
    float a = side*(0.55 + 0.35*hash(vec2(i, seed))) + sin(time*1.1 + i*0.7 + seed)*0.08 + gustSway(i*0.9 + seed)*0.5;
    vec2 ld = rot(a)*vec2(u - uc, v);
    float Ll = 0.12 + 0.035*hash(vec2(seed, i));
    float x = ld.x/Ll;
    b = max(b, ok * step(0.0, x) * step(x, 1.0) * step(abs(ld.y), 0.14*Ll*sin(PI*clamp(x, 0.0, 1.0))));
  }
  return b;
}
float olive(vec2 p){
  float s = sin(time*0.4)*0.02 + gustSway(1.0)*0.1;
  float b = olBranch(p, vec2(-0.45, 1.45), -0.55 + s, 1.3, 0.12, 1.0);
  b = max(b, olBranch(p, vec2(-0.4, 0.95), -0.18 + s*1.3, 1.05, 0.18, 2.0));
  b = max(b, olBranch(p, vec2(1.0, 1.55), -2.45 - s, 0.95, -0.15, 3.0));
  return b;
}
// a hanging pot: its cord up to the hook and its round-bottomed bowl, centred on c
float hangPot(vec2 p, vec2 c, float r){
  float cord = step(abs(p.x - c.x), 0.0025)*step(c.y, p.y)*step(p.y, 2.1);
  vec2 d = (p - c)/vec2(r, r*0.75);
  float bowl = step(length(d), 1.0)*step(d.y, 0.35);
  float rim = step(abs(d.x), 1.08)*step(abs(d.y - 0.42), 0.09);
  return max(cord, max(bowl, rim));
}
// the whole hanging plant swings a little on its cord, from the hook
vec2 swing(vec2 p, vec2 hook, float s){ return hook + rot(-s)*(p - hook); }
float fern(vec2 p){
  vec2 hook = vec2(0.12, 2.1), c = vec2(0.12, 1.42);
  p = swing(p, hook, sin(time*0.35)*0.012 + gustSway(0.5)*0.03);
  float b = hangPot(p, c, 0.13);
  // fronds arch out over the rim and fall away under their own weight; each pinna small and many
  for (int i = 0; i < 9; i++) {
    float fi = float(i), h = hash(vec2(fi, 11.0));
    // from out to the right round to out to the left; the outer ones start upward and arch over
    float a = -mix(0.35, PI - 0.35, fi/8.0) + (fi < 0.5 ? 0.85 : fi > 7.5 ? -0.85 : 0.0)
              + (h - 0.5)*0.25 + gustSway(fi*1.1)*0.12 + sin(time*0.7 + fi)*0.02;
    float sc = 0.4 + 0.08*h;
    vec2 base = c + vec2(cos(a)*0.09, 0.05);
    // curl that bends each frond downward whichever way it points
    b = max(b, frond((p - base)/sc, vec2(0.0), a, (0.5 + 0.3*h)/sc, 0.55*cos(a)));
  }
  return b;
}
// pothos: vines trail down from a pot, heart-shaped leaves along them, turning this way and that
float heart(vec2 d){                      // a heart: two lobes at the top, its tip at y = -1
  vec2 a = vec2(abs(d.x), d.y);
  float lobe = step(length(a - vec2(0.36, 0.28)), 0.42);
  float wedge = step(-1.0, d.y)*step(d.y, 0.35)*step(a.x, 0.74*(d.y + 1.0)/1.3);
  return max(lobe, wedge);
}
float pothos(vec2 p){
  vec2 hook = vec2(-0.05, 2.1), c = vec2(-0.05, 1.4);
  p = swing(p, hook, sin(time*0.3)*0.01 + gustSway(0.2)*0.025);
  float b = hangPot(p, c, 0.14);
  for (int v = 0; v < 4; v++) {
    float fv = float(v), x0 = c.x + (fv - 1.5)*0.075, len = 0.55 + 0.6*hash(vec2(fv, 4.0));
    float y = c.y - p.y;                                   // how far down this vine
    if (y < -0.05 || y > len + 0.12) continue;
    float sway = (sin(time*0.6 + fv*1.9)*0.02 + gustSway(fv*1.7)*0.08)*y*y;
    float vx = x0 + (fv - 1.5)*0.12*y + 0.035*sin(y*5.0 + fv*2.0) + sway;
    b = max(b, step(abs(p.x - vx), 0.0035)*step(0.0, y)*step(y, len));
    // the leaf nearest this height on either side of the vine
    float sp = 0.11, k = floor(y/sp);
    for (int j = -1; j <= 1; j++) {
      float i = k + float(j), yl = (i + 0.5)*sp;
      if (yl > len || yl < 0.06) continue;
      float side = mod(i + fv, 2.0)*2.0 - 1.0, hh = hash(vec2(i, fv));
      float lx = x0 + (fv - 1.5)*0.12*yl + 0.035*sin(yl*5.0 + fv*2.0) + (sin(time*0.6 + fv*1.9)*0.02 + gustSway(fv*1.7)*0.08)*yl*yl;
      float sz = 0.04 + 0.025*hh*(1.0 - 0.4*yl/len);
      // the tip points down and away from the vine
      float th = -PI*0.5 + side*(0.55 + 0.45*hh) + sin(time*1.1 + i)*0.05 + gustSway(i*0.7)*0.3;
      vec2 at = vec2(lx, c.y - yl) + vec2(cos(th), sin(th))*sz*1.05;
      b = max(b, heart(rot(th + PI*0.5)*(p - at)/sz));
    }
  }
  return b;
}
// bird of paradise: tall paddle leaves on long stalks, torn along their ribs, and one crane-headed flower
float paddle(vec2 p, vec2 base, float ang, float stalk, float len, float wid, float seed){
  vec2 d = rot(ang)*(p - base);
  float st = step(abs(d.y), 0.008)*step(0.0, d.x)*step(d.x, stalk);
  float x = (d.x - stalk)/len;
  float w = wid*pow(max(sin(PI*clamp(x, 0.0, 1.0)), 0.0), 0.7);
  float leaf = step(0.0, x)*step(x, 1.0)*step(abs(d.y), w);
  // tears: thin gaps running from the edge in toward the midrib, slanting toward the tip
  float t = (x*len - abs(d.y)*0.9)*14.0 + seed;
  float tear = step(abs(fract(t) - 0.5), 0.05)*step(0.45 + 0.4*hash(vec2(floor(t), seed)), abs(d.y)/max(w, 1e-3))*step(0.5, hash(vec2(floor(t), seed + 3.0)));
  return max(st, leaf*(1.0 - tear));
}
float birdOfParadise(vec2 p){
  p.x += (sin(time*0.45)*0.008 + gustSway(p.y)*0.03)*max(p.y + 0.5, 0.0);
  vec2 root = vec2(0.42, -0.5);
  float b = 0.0;
  for (int i = 0; i < 5; i++) {
    float fi = float(i), h = hash(vec2(fi, 21.0));
    float ang = 1.2 + fi*0.17 + (h - 0.5)*0.12 + sin(time*0.7 + fi*1.3)*0.015 + gustSway(fi)*0.05;
    float stalk = 0.75 + 0.35*h;
    b = max(b, paddle(p, root, ang, stalk, 0.62 + 0.15*h, 0.13, fi*7.0));
  }
  // the flower: a long beak on its stalk with a crest of upright petals
  vec2 fb = root + vec2(cos(1.75), sin(1.75))*1.05;
  b = max(b, step(sdSeg(p, root, fb), 0.008));
  vec2 d = p - fb;
  float beak = step(sdSeg(d, vec2(0.0), vec2(0.26, 0.05)), 0.022*(1.0 - clamp(d.x/0.26, 0.0, 1.0)) + 0.004);
  for (int k = 0; k < 3; k++) {
    float fk = float(k);
    vec2 a = vec2(0.05 + fk*0.04, 0.03), e = a + vec2(-0.03 + fk*0.05, 0.16 - fk*0.02);
    beak = max(beak, step(sdSeg(d, a, e), 0.012*(1.0 - clamp(dot(d - a, e - a)/dot(e - a, e - a), 0.0, 1.0)) + 0.002));
  }
  return max(b, beak);
}
// string of pearls: strands of little round beads spilling over the rim of a hanging pot
float pearls(vec2 p){
  vec2 hook = vec2(0.2, 2.1), c = vec2(0.2, 1.38);
  p = swing(p, hook, sin(time*0.33)*0.012 + gustSway(0.9)*0.03);
  float b = hangPot(p, c, 0.11);
  for (int v = 0; v < 9; v++) {
    float fv = float(v), h = hash(vec2(fv, 31.0)), len = 0.25 + 0.75*h;
    float y = c.y - 0.02 - p.y;
    if (y < 0.0 || y > len + 0.02) continue;
    float x0 = c.x + (fv - 4.0)*0.028;
    float vx = x0 + (fv - 4.0)*0.02*y + 0.02*sin(y*7.0 + fv*1.3) + (sin(time*0.7 + fv)*0.015 + gustSway(fv*1.3)*0.06)*y*y;
    b = max(b, step(abs(p.x - vx), 0.0015));
    float sp = 0.032, i = floor(y/sp + 0.5);
    float yc = i*sp;
    if (yc > len) continue;
    float vxc = x0 + (fv - 4.0)*0.02*yc + 0.02*sin(yc*7.0 + fv*1.3) + (sin(time*0.7 + fv)*0.015 + gustSway(fv*1.3)*0.06)*yc*yc;
    float r = 0.0115*(0.85 + 0.3*hash(vec2(i, fv)));
    b = max(b, step(length(vec2(p.x - vxc - 0.004*(hash(vec2(fv, i)) - 0.5), y - yc)), r));
  }
  return b;
}
// fiddle-leaf fig: a straight stem with big leaves, broad near the tip and pinched in the middle like a violin
float fiddleLeaf(vec2 p, vec2 base, float ang, float sz){
  vec2 d = rot(ang)*(p - base)/sz;
  float x = d.x;
  float w = 0.5*pow(clamp(x, 0.0, 1.0), 0.55)*sqrt(max(1.0 - x, 0.0))*(1.0 - 0.18*exp(-pow((x - 0.45)/0.12, 2.0))) + 0.01;
  float vein = 1.0 - step(abs(d.y), 0.008)*step(0.05, x)*step(x, 0.85);
  return step(0.0, x)*step(x, 1.0)*step(abs(d.y), w)*vein;
}
float fiddle(vec2 p){
  p.x += (sin(time*0.5)*0.01 + gustSway(p.y*1.5)*0.035)*max(p.y + 0.3, 0.0);
  float sx = 0.38;
  float b = step(abs(p.x - sx - 0.02*sin(p.y*2.0)), 0.011)*step(p.y, 1.3)*step(-0.5, p.y);
  for (int i = 0; i < 7; i++) {
    float fi = float(i), y = 0.05 + fi*0.18, side = mod(fi, 2.0)*2.0 - 1.0, h = hash(vec2(fi, 41.0));
    vec2 base = vec2(sx + 0.02*sin(y*2.0), y);
    float th = PI*0.5 - side*(0.7 + 0.45*h) + sin(time*(0.7 + fi*0.1) + fi)*0.04 + gustSway(fi*1.4)*0.3;
    b = max(b, fiddleLeaf(p, base, th, 0.42 + 0.08*h - 0.015*fi));
  }
  return b;
}
float foliage(vec2 p){
  if (plantType == 1) return ficus(p);
  if (plantType == 2) return monstera(p);
  if (plantType == 3) return palm(p);
  if (plantType == 5) return fern(p);
  if (plantType == 6) return pothos(p);
  if (plantType == 7) return birdOfParadise(p);
  if (plantType == 8) return pearls(p);
  if (plantType == 9) return fiddle(p);
  return olive(p);
}
vec3 leafTint(){                        // light that gets through a leaf (green, faint; amber in autumn)
  vec3 t = plantType == 4 ? vec3(0.035, 0.042, 0.026) : plantType == 3 ? vec3(0.03, 0.05, 0.014) : vec3(0.025, 0.048, 0.012);
  t = mix(t, vec3(0.05, 0.085, 0.02), season.x*0.7);
  return mix(t, vec3(0.2, 0.1, 0.018), season.z*0.85);
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
  float arc = exp(-pow((length(v - vec2(0.0, 0.05)) - 0.115)/(0.006 + w), 2.0)) * smoothstep(-0.02, -0.07, v.y) * inside;
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
// gradient noise, so no square lattice shows through in raking light; 0..1 like value noise
vec2 gradP(vec2 i){ float a = 6.2831853*hash(i); return vec2(cos(a), sin(a)); }
float noiseP(vec2 p, float per){ vec2 i=floor(p), f=fract(p); vec2 u=f*f*f*(f*(f*6.-15.)+10.);
  vec2 a=mod(i,per), b=mod(i+1.,per);
  float n = mix(mix(dot(gradP(a), f), dot(gradP(vec2(b.x,a.y)), f - vec2(1.,0.)), u.x),
                mix(dot(gradP(vec2(a.x,b.y)), f - vec2(0.,1.)), dot(gradP(b), f - 1.), u.x), u.y);
  return 0.5 + 0.75*n; }
float fbmP(vec2 p, float per){ float a=.5, s=0.; for(int i=0;i<4;i++){ s+=a*noiseP(p,per); p*=2.; per*=2.; a*=.5; } return s; }
float plasterP(vec2 x){ return fbmP(x*7.0, 7.0*TILE)*0.55 + fbmP(x*38.0, 38.0*TILE)*0.3 + noiseP(x*170.0, 170.0*TILE)*0.08 + 0.035; }

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

