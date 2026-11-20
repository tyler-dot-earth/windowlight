// once at start: plaster height over one TILE of wall (the slope is taken per pixel in composite)
in vec2 vUv; out vec4 o;
void main(){ o = vec4(plasterP(vUv*TILE), 0.0, 0.0, 1.0); }
