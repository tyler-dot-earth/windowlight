// every frame, full resolution: wall, light and grading, reading the baked textures
in vec2 vUv; out vec4 fragColor;
uniform vec2 res;
uniform sampler2D uPlaster, uAp, uFol, uCaus;
uniform vec2 apTexel, folTexel;   // texels per unit
uniform vec3 wallA, sunTint, shadeTint, nightTint;
uniform float mottle, bumpAmt, expo, sat, contrast, ambAmt, fade, lightK;

float jit;
vec2 apUV(vec2 q){ return (q - AP_MIN)/AP_SIZE; }
vec2 folUV(vec2 q){ return (q - FOL_MIN)/FOL_SIZE; }
// penumbra: six rotated taps over the blur disk, each from a pre-blurred mip level
vec3 softAp(vec2 q, float r){
  float lod = max(log2(r*apTexel.x*0.55), 0.0); vec3 s = vec3(0.0);
  for (int i = 0; i < 6; i++){
    float a = float(i)*2.39996 + jit, rr = sqrt((float(i) + .5)/6.0)*r;
    s += textureLod(uAp, apUV(q + rr*vec2(cos(a), sin(a))), lod).rgb;
  }
  return s/6.0;
}
float softFol(vec2 q, float r){
  float lod = max(log2(r*folTexel.x*0.55), 0.0), s = 0.0;
  for (int i = 0; i < 6; i++){
    float a = float(i)*2.39996 + jit, rr = sqrt((float(i) + .5)/6.0)*r;
    s += textureLod(uFol, folUV(q + rr*vec2(cos(a), sin(a))), lod).r;
  }
  return s/6.0;
}

void main(){
  vec2 fc = gl_FragCoord.xy;
  jit = hash(fc)*6.2831;
  vec2 sc = fc/res;
  bool pool = caustic == 1 || caustic == 3;
  vec2 uv = (fc/res.y - vec2(0.5*res.x/res.y, 0.5))*(pool ? 1.9 : 1.4) + vec2(0.0, 0.5);

  vec3 n = vec3(sin(tilt), 0.0, cos(tilt)), ua = vec3(cos(tilt), 0.0, -sin(tilt));
  bool isSun = sunAlt > 0.0;
  vec3 L = isSun ? sun : -sun;
  float alt = abs(sunAlt), ns = dot(n, L);

  vec3 wc = WC.x*ua + vec3(0.0, WC.y, 0.0) + D_WIN*n;
  vec2 pc = wc.xy - (wc.z/max(L.z, 0.12))*L.xy;
  vec2 pcR = wc.xy - (wc.z/max(L.z, 0.12))*vec2(L.x, -L.y);
  vec2 aim = pool && isSun ? mix(pc, pcR, 0.5) : pc;
  uv += clamp(aim - vec2(0.0, 0.5), vec2(-0.6, -0.45), vec2(0.6, pool ? 0.9 : 0.45)) * 0.8 * smoothstep(0.0, 0.06, alt) * smoothstep(0.03, 0.12, ns);

  vec3 wall = pow(wallA, vec3(2.2));
  wall *= 1.0 + mottle*(fbm(uv*2.2 + 5.0) - 0.5)*2.4;
  vec4 pl = texture(uPlaster, uv/TILE);
  vec3 nb = normalize(vec3(-pl.gb*bumpAmt, 1.0));
  wall *= 0.93 + 0.14*pl.r;

  float day = smoothstep(-0.10, 0.06, sunAlt);
  float twi = exp(-pow((sunAlt + 0.03)/0.07, 2.0));
  vec3 amb = mix(0.04*mix(nightTint, vec3(0.6), 0.4), ambAmt*shadeTint, day) + twi*vec3(0.05, 0.03, 0.065)*ambAmt*5.0;
  amb *= 0.72 + 0.5*smoothstep(-1.4, 1.2, uv.x);
  vec3 col = wall*amb;

  if (ns > 0.03 && L.z > 0.0) {
    vec3 p = vec3(uv, 0.0);
    float tw = (D_WIN - dot(n, p))/ns;   vec3 hw = p + tw*L;
    float tp = (D_PLANT - dot(n, p))/ns; vec3 hp = p + tp*L;
    vec2 q = vec2(dot(ua, hw), hw.y), qp = vec2(dot(ua, hp), hp.y);
    vec3 pcw = vec3(pc, 0.0); vec3 hc = pcw + (D_PLANT - dot(n, pcw))/ns*L;
    qp -= (vec2(dot(ua, hc), hc.y) - WC)*0.8;
    float hz = 1.0 + 7.0*haze;
    float pen = (0.004 + 0.004*tw)*hz;
    vec3 lit = softAp(q, pen);
    float litM = clamp(dot(lit, vec3(0.3333)), 0.0, 1.0);
    vec3 trans = vec3(1.0);
    if (plantType > 0) {
      float occ = softFol(qp, (0.003 + 0.006*tp)*hz);
      trans = mix(vec3(1.0), leafTint(), occ*(0.95 - 0.45*haze));
      float tb = (D_BACK - dot(n, p))/ns; vec3 hb = p + tb*L;
      vec3 hcb = pcw + (D_BACK - dot(n, pcw))/ns*L;
      vec2 qb = vec2(dot(ua, hb), hb.y) - (vec2(dot(ua, hcb), hcb.y) - WC)*0.8;
      float occB = softFol(vec2(0.95 - qb.x, qb.y)*0.9 + vec2(0.0, 0.12), (0.004 + 0.007*tb)*hz*0.9);
      trans *= mix(vec3(1.0), leafTint(), occB*0.8*(0.95 - 0.45*haze));
    }
    if (caustic >= 2) trans *= carafe(q - WC, pen);
    float E = isSun ? 1.7*(1.0 - 0.62*haze) : 0.32;
    float strength = E*smoothstep(0.0, 0.08, alt)*lightK;
    vec3 lc = isSun ? sunColor(alt)*sunTint : mix(nightTint, vec3(0.8, 0.84, 0.92), 0.55);
    float flat_ = max(L.z, 0.0);
    float bumpR = clamp(max(dot(nb, L), 0.0)/max(flat_, 0.05), 0.0, 3.0);
    float shade = (0.4 + 0.6*flat_)*mix(1.0, bumpR, 0.75);
    float edge = 4.0*litM*(1.0 - litM);
    vec3 fringe = mix(vec3(1.0), vec3(1.14, 0.88, 0.64), edge*0.55*(isSun ? 1.0 : 0.0)*(1.0 - haze));
    vec2 lq = q - WC;
    float halo = sbox(lq, WH, pen*6.0 + 0.03)*0.07 + sbox(lq, WH, 0.18 + 0.25*haze)*(0.05 + 0.08*haze);
    vec3 direct = lc*strength*(lit*trans*fringe*shade + halo*(0.4 + 0.6*flat_)*vec3(1.0, 0.9, 0.8));
    col += wall*direct;
    col += wall*wall*lc*strength*0.06*(isSun ? 1.0 : 0.0);
    if (isSun) col += lc*strength*motes(uv)*lit*0.05*(1.0 - haze);
    if (isSun && pool) {
      vec3 Lr = vec3(L.x, -L.y, L.z);
      vec3 hr = p + tw*Lr;
      vec3 wp = hr + max(hr.y - Y_WATER, 0.0)/max(L.y, 0.03)*Lr;
      vec2 wq = wp.xz*3.0;
      vec2 wob = (vec2(noise(wq*0.5 + vec2(time*0.3, 0.0)), noise(wq*0.5 + vec2(3.7, time*0.27))) - 0.5)*0.06;
      vec2 qr = vec2(dot(ua, hr), hr.y);
      vec3 litR = softAp(qr + wob, pen*1.4 + 0.006);
      float cz = texture(uCaus, apUV(qr + wob)).r;
      float fres = 0.22 + 0.4*pow(1.0 - sin(alt), 3.0);
      col += wall*lc*strength*litR*fres*(0.2 + 1.5*cz)*vec3(0.94, 1.0, 1.02)*(1.0 - 0.7*haze);
    }
  }

  float vig = 1.0 - 0.38*pow(length((sc - 0.5)*vec2(1.0, 1.2))*1.25, 2.0);
  col *= vig*expo*fade;
  col = aces(col);
  col = mix(vec3(dot(col, vec3(0.2126, 0.7152, 0.0722))), col, sat);
  col = pow(col, vec3(1.0/2.2));
  col = clamp((col - 0.45)*contrast + 0.45, 0.0, 1.0);
  col += (hash(fc + 17.0) - 0.5)/255.0;
  fragColor = vec4(col, 1.0);
}
