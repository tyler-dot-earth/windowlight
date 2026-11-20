// where the sun has landed on the wall over a year, blurred: twelve dates' worth of the window's patch, summed.
// The paint fades there. Redrawn only when the window, its angle, its facing or the place changes
in vec2 vUv; out vec4 o;
uniform vec2 texSize;
uniform vec4 path[72];   // toward the sun in the room frame, w 1 when it is up
uniform sampler2D uAp;
void main(){
  vec3 p = vec3(FADE_MIN + vUv*FADE_SIZE, 0.0);
  vec3 n = vec3(sin(tilt), 0.0, cos(tilt)), ua = vec3(cos(tilt), 0.0, -sin(tilt));
  float acc = 0.0, cnt = 0.0;
  for (int i = 0; i < 72; i++) {
    vec4 s = path[i];
    float ns = dot(n, s.xyz);
    if (s.w <= 0.0 || ns < 0.03 || s.z <= 0.0) continue;
    cnt += 1.0;
    vec3 h = p + (D_WIN - dot(n, p))/ns*s.xyz;
    vec2 a = (vec2(dot(ua, h), h.y) - AP_MIN)/AP_SIZE;
    if (a.x < 0.0 || a.y < 0.0 || a.x > 1.0 || a.y > 1.0) continue;
    acc += dot(textureLod(uAp, a, 6.0).rgb, vec3(0.3333))*s.z*smoothstep(0.0, 0.2, s.y);
  }
  // as a share of the hours the sun reaches the window at all
  o = vec4(acc/max(cnt, 1.0), 0.0, 0.0, 1.0);
}
