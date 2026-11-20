// window light in window-plane coords: rgb transmission (above 1 where raindrops focus it). Redrawn when it moves
in vec2 vUv; out vec4 o;
uniform vec2 texSize;
void main(){
  vec2 q = AP_MIN + vUv*AP_SIZE, d = AP_SIZE/texSize*0.25;
  o = vec4(0.25*(apertureC(q + vec2(-d.x, -d.y)) + apertureC(q + vec2(d.x, -d.y)) + apertureC(q + vec2(-d.x, d.y)) + apertureC(q + d)), 1.0);
}
