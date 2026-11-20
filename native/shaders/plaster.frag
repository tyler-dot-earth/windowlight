// once at start: plaster height and slope over one TILE of wall
in vec2 vUv; out vec4 o;
uniform vec2 texSize;
void main(){
  vec2 x = vUv*TILE;
  float e = TILE/texSize.x;
  float h0 = plasterP(x);
  vec2 g = vec2(plasterP(x + vec2(e, 0.0)) - h0, plasterP(x + vec2(0.0, e)) - h0)/e;
  o = vec4(h0, g*0.010, 0.0);
}
