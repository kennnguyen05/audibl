import { useEffect, useRef } from "react";
import { FRAG_SOURCE, VERT_SOURCE } from "@/shaders/plasma";

/**
 * The app's ambient background: an animated WebGL "Plasma" field, made with
 * the 21st.dev Shader Builder.
 *
 * Everything the look depends on lives in the uniform tables below — the GLSL
 * itself is generated code and is kept verbatim. Retune here, not there.
 */

/** The palette, low -> high. Flattened because WebGL uploads a vec3 array as
 *  one buffer. */
const COLORS = new Float32Array([
  0.063, 0.063, 0.063, // #101010
  0.961, 0.961, 0.961, // #f5f5f5
  0.690, 0.690, 0.690, // #b0b0b0
  0.227, 0.227, 0.227, // #3a3a3a
  0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, // u_colors is vec3[8]; four go unused
]);
const COLOR_COUNT = 4;

/** Uniforms that never change after link. See the shader header for the
 *  meaning of each lane. */
const STATIC_UNIFORMS = {
  // scale, intensity, paramA, warp
  u_shape: [1.26, 0.0, 0.28, 0.0],
  // detail, contrast, brightness, saturation
  u_surface: [1.82, 1.5, -0.02, 1.04],
  // hue, vignette, blur, grain
  u_finish: [5.36, 0.29, 0.006, 0.04],
  // seed, rotation, drift, OKLab toggle
  u_transform: [1503.0, 0.0, 0.06, 0.0],
} as const;

/** Time scale from the recipe: speed 25/100. */
const TIME_SCALE = 0.57;
/** Field pan, u_space lanes 0-1; the pointer fills lanes 2-3. */
const OFFSET_X = -0.2;
const OFFSET_Y = 0.04;
/** Cursor lane 1 is the effect id — 0.0 is push — then strength and radius. */
const CURSOR_EFFECT = 0.0;
const CURSOR_STRENGTH = 0.48;
const CURSOR_RADIUS = 0.38;
/** How fast the push catches up to the pointer, per frame at 60fps. Below 1
 *  so the distortion trails the cursor instead of snapping to it. */
const CURSOR_EASE = 0.12;
const PRESENCE_EASE = 0.06;
/** Retina is worth drawing here — the field has fine grain and a hard
 *  vignette — but nothing above 2x is, so the buffer is capped there. */
const MAX_DPR = 2;

function compile(
  gl: WebGLRenderingContext,
  type: number,
  source: string,
): WebGLShader | null {
  const shader = gl.createShader(type);
  if (!shader) return null;
  gl.shaderSource(shader, source);
  gl.compileShader(shader);
  if (!gl.getShaderParameter(shader, gl.COMPILE_STATUS)) {
    console.warn("[shader] compile failed:", gl.getShaderInfoLog(shader));
    gl.deleteShader(shader);
    return null;
  }
  return shader;
}

export function ShaderBackground() {
  const hostRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const host = hostRef.current;
    if (!host) return;

    // The canvas is created here rather than rendered by React, and thrown
    // away on cleanup. A WebGL context is bound to its canvas element for the
    // element's lifetime: once this effect's teardown calls loseContext(),
    // getContext() on that same element only ever hands back the dead context
    // and every create*() returns null. StrictMode's mount/unmount/mount in
    // dev does exactly that, so each mount gets a canvas of its own.
    const canvas = document.createElement("canvas");
    canvas.setAttribute("aria-hidden", "true");
    // A canvas is a replaced element, so insets alone do not stretch it — it
    // would fall back to its intrinsic 300x150. The size has to be explicit.
    canvas.style.cssText =
      "position:absolute;top:0;left:0;width:100%;height:100%";
    host.appendChild(canvas);

    // `alpha: false` lets the compositor skip blending the canvas with the
    // page; nothing renders behind it and the shader always writes a = 1.
    const gl =
      canvas.getContext("webgl", { alpha: false, antialias: false, depth: false }) ??
      (canvas.getContext("experimental-webgl", {
        alpha: false,
      }) as WebGLRenderingContext | null);
    if (!gl) {
      console.warn("[shader] WebGL unavailable; falling back to the flat background");
      host.removeChild(canvas);
      return;
    }

    const vert = compile(gl, gl.VERTEX_SHADER, VERT_SOURCE);
    const frag = compile(gl, gl.FRAGMENT_SHADER, FRAG_SOURCE);
    const program = gl.createProgram();
    if (!vert || !frag || !program) {
      host.removeChild(canvas);
      return;
    }
    gl.attachShader(program, vert);
    gl.attachShader(program, frag);
    gl.linkProgram(program);
    if (!gl.getProgramParameter(program, gl.LINK_STATUS)) {
      console.warn("[shader] link failed:", gl.getProgramInfoLog(program));
      host.removeChild(canvas);
      return;
    }
    gl.useProgram(program);

    // One triangle big enough to cover clip space: cheaper than a quad and it
    // avoids the diagonal seam two triangles would share.
    const buffer = gl.createBuffer();
    gl.bindBuffer(gl.ARRAY_BUFFER, buffer);
    gl.bufferData(
      gl.ARRAY_BUFFER,
      new Float32Array([-1, -1, 3, -1, -1, 3]),
      gl.STATIC_DRAW,
    );
    const positionLoc = gl.getAttribLocation(program, "a_position");
    gl.enableVertexAttribArray(positionLoc);
    gl.vertexAttribPointer(positionLoc, 2, gl.FLOAT, false, 0, 0);

    const loc = (name: string) => gl.getUniformLocation(program, name);
    gl.uniform3fv(loc("u_colors[0]"), COLORS);
    for (const [name, value] of Object.entries(STATIC_UNIFORMS)) {
      gl.uniform4f(loc(name), value[0], value[1], value[2], value[3]);
    }
    const sceneLoc = loc("u_scene");
    const spaceLoc = loc("u_space");
    const cursorLoc = loc("u_cursor");

    let width = 0;
    let height = 0;
    const resize = () => {
      const dpr = Math.min(window.devicePixelRatio || 1, MAX_DPR);
      const next = [
        Math.max(1, Math.round(canvas.clientWidth * dpr)),
        Math.max(1, Math.round(canvas.clientHeight * dpr)),
      ];
      if (next[0] === width && next[1] === height) return;
      [width, height] = next;
      canvas.width = width;
      canvas.height = height;
      gl.viewport(0, 0, width, height);
    };

    // Pointer, normalized to -1..1 in canvas space as the shader expects.
    let pointerX = 0;
    let pointerY = 0;
    let targetX = 0;
    let targetY = 0;
    let presence = 0;
    let targetPresence = 0;

    const draw = (time: number) => {
      gl.uniform4f(sceneLoc, width, height, time * TIME_SCALE, COLOR_COUNT);
      gl.uniform4f(spaceLoc, OFFSET_X, OFFSET_Y, pointerX, pointerY);
      gl.uniform4f(cursorLoc, presence, CURSOR_EFFECT, CURSOR_STRENGTH, CURSOR_RADIUS);
      gl.drawArrays(gl.TRIANGLES, 0, 3);
    };

    // `prefers-reduced-motion` in index.css only neuters CSS animation, so the
    // shader has to honour it itself: one still frame, no loop, no pointer.
    const motionQuery = window.matchMedia("(prefers-reduced-motion: reduce)");
    let frame = 0;
    /** Seconds of animation elapsed. Advanced by real frame deltas rather than
     *  read from the timestamp, so time does not jump while the window is
     *  hidden — the main window hides to the tray rather than closing. */
    let elapsed = 0;
    let lastFrame = 0;

    const tick = (now: number) => {
      frame = requestAnimationFrame(tick);
      // Clamped: a long hidden stretch or a stalled frame must not teleport
      // the field forward.
      elapsed += Math.min((now - lastFrame) / 1000, 1 / 20);
      lastFrame = now;
      pointerX += (targetX - pointerX) * CURSOR_EASE;
      pointerY += (targetY - pointerY) * CURSOR_EASE;
      presence += (targetPresence - presence) * PRESENCE_EASE;
      resize();
      draw(elapsed);
    };

    const stop = () => {
      if (frame) cancelAnimationFrame(frame);
      frame = 0;
    };
    const start = () => {
      if (frame || motionQuery.matches) return;
      lastFrame = performance.now();
      frame = requestAnimationFrame(tick);
    };

    const applyMotionPreference = () => {
      stop();
      if (motionQuery.matches) {
        targetPresence = 0;
        presence = 0;
        resize();
        draw(elapsed);
      } else {
        start();
      }
    };

    const onPointerMove = (e: PointerEvent) => {
      targetX = (e.clientX / window.innerWidth) * 2 - 1;
      // Canvas y runs up, the pointer's runs down.
      targetY = 1 - (e.clientY / window.innerHeight) * 2;
      targetPresence = 1;
    };
    const onPointerLeave = () => {
      targetPresence = 0;
    };
    // Nothing to animate behind a hidden window; the tray keeps it hidden for
    // long stretches, and an unpaused loop would burn the GPU the whole time.
    const onVisibility = () => {
      if (document.hidden) stop();
      else applyMotionPreference();
    };

    window.addEventListener("pointermove", onPointerMove);
    document.addEventListener("pointerleave", onPointerLeave);
    document.addEventListener("visibilitychange", onVisibility);
    motionQuery.addEventListener("change", applyMotionPreference);
    window.addEventListener("resize", resize);

    resize();
    applyMotionPreference();

    // The canvas, the context and every listener are created inside this
    // effect, so StrictMode's mount/unmount/mount in dev tears the whole thing
    // down and rebuilds it from scratch.
    return () => {
      stop();
      window.removeEventListener("pointermove", onPointerMove);
      document.removeEventListener("pointerleave", onPointerLeave);
      document.removeEventListener("visibilitychange", onVisibility);
      motionQuery.removeEventListener("change", applyMotionPreference);
      window.removeEventListener("resize", resize);
      gl.deleteBuffer(buffer);
      gl.deleteProgram(program);
      gl.deleteShader(vert);
      gl.deleteShader(frag);
      gl.getExtension("WEBGL_lose_context")?.loseContext();
      canvas.remove();
    };
  }, []);

  return (
    <div
      ref={hostRef}
      aria-hidden
      className="pointer-events-none fixed inset-0 z-0 overflow-hidden"
    />
  );
}
