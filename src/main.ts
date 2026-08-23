import { Vec2 } from './math';
import { RigidBody, PhysicsWorld } from './physics';
import { Renderer } from './render';
import { UIControls } from './ui';

// Color Palettes
const NEON_PALETTE = ['#38bdf8', '#818cf8', '#c084fc', '#f472b6', '#34d399', '#fbbf24', '#f87171'];

function getRandomNeonColor(): string {
  const idx = Math.floor(Math.random() * NEON_PALETTE.length);
  return NEON_PALETTE.at(idx) ?? '#38bdf8';
}

export function initApp(): void {
  const canvas = document.getElementById('physics-canvas') as HTMLCanvasElement | null;
  if (!canvas) {
    console.error('Canvas element #physics-canvas not found!');
    return;
  }

  const initialWidth = window?.innerWidth || 1280;
  const initialHeight = window?.innerHeight || 720;

  const world = new PhysicsWorld({
    gravity: new Vec2(0, 980),
    wind: new Vec2(0, 0),
    boundsWidth: initialWidth,
    boundsHeight: initialHeight,
    solverIterations: 8
  });

  const renderer = new Renderer(canvas);

  // Resize Handler
  const handleResize = () => {
    const width = window?.innerWidth || 1280;
    const height = window?.innerHeight || 720;
    renderer?.resize(width, height);
    world?.resizeBounds(width, height);
  };

  window.addEventListener('resize', handleResize);
  handleResize();

  // Scene Generators
  const spawnPyramid = () => {
    world?.clearBodies();
    const w = world.bounds.width;
    const h = world.bounds.height;
    const cx = w * 0.5;
    const groundY = h - 50;

    // Static Ground Platform
    world?.addBody(new RigidBody({
      type: 'box',
      position: new Vec2(cx, groundY),
      width: Math.min(w * 0.85, 900),
      height: 30,
      isStatic: true,
      color: '#475569',
      friction: 0.6
    }));

    // Box dimensions
    const boxW = 44;
    const boxH = 40;
    const rows = 6;
    const startY = groundY - 15 - boxH * 0.5;

    for (let r = 0; r < rows; r++) {
      const countInRow = rows - r;
      const rowStartX = cx - (countInRow - 1) * (boxW + 4) * 0.5;
      const rowY = startY - r * (boxH + 2);

      for (let c = 0; c < countInRow; c++) {
        const x = rowStartX + c * (boxW + 4);
        world?.addBody(new RigidBody({
          type: 'box',
          position: new Vec2(x, rowY),
          width: boxW,
          height: boxH,
          mass: 3.0,
          friction: 0.5,
          restitution: 0.1,
          color: NEON_PALETTE.at(r % NEON_PALETTE.length) ?? '#38bdf8'
        }));
      }
    }
  };

  const spawnCratesAndDominoes = () => {
    world?.clearBodies();
    const w = world.bounds.width;
    const h = world.bounds.height;
    const groundY = h - 50;

    // Ground
    world?.addBody(new RigidBody({
      type: 'box',
      position: new Vec2(w * 0.5, groundY),
      width: w * 0.85,
      height: 30,
      isStatic: true,
      color: '#475569',
      friction: 0.6
    }));

    // Domino Chain on Left
    const dominoW = 12;
    const dominoH = 65;
    const dominoCount = 12;
    const dominoStartX = w * 0.2;

    for (let i = 0; i < dominoCount; i++) {
      world?.addBody(new RigidBody({
        type: 'box',
        position: new Vec2(dominoStartX + i * 36, groundY - 15 - dominoH * 0.5),
        width: dominoW,
        height: dominoH,
        mass: 1.5,
        friction: 0.4,
        restitution: 0.15,
        color: '#38bdf8'
      }));
    }

    // Heavy Trigger Ball
    const triggerBall = new RigidBody({
      type: 'circle',
      position: new Vec2(dominoStartX - 60, groundY - 15 - 25),
      velocity: new Vec2(350, 0),
      radius: 25,
      mass: 10.0,
      restitution: 0.2,
      friction: 0.3,
      color: '#f43f5e'
    });
    world?.addBody(triggerBall);

    // Stacks of Crates on Right
    const stackStartX = dominoStartX + dominoCount * 36 + 60;
    for (let col = 0; col < 3; col++) {
      for (let row = 0; row < 5; row++) {
        world?.addBody(new RigidBody({
          type: 'box',
          position: new Vec2(stackStartX + col * 48, groundY - 15 - 22 - row * 46),
          width: 44,
          height: 44,
          mass: 2.0,
          friction: 0.4,
          restitution: 0.1,
          color: getRandomNeonColor()
        }));
      }
    }
  };

  const spawnCircleStorm = () => {
    world?.clearBodies();
    const w = world.bounds.width;
    const h = world.bounds.height;

    for (let i = 0; i < 45; i++) {
      const radius = 14 + Math.random() * 20;
      const x = w * 0.15 + Math.random() * (w * 0.7);
      const y = 80 + Math.random() * (h * 0.5);
      const vx = (Math.random() - 0.5) * 300;
      const vy = (Math.random() - 0.5) * 200;

      world?.addBody(new RigidBody({
        type: 'circle',
        position: new Vec2(x, y),
        velocity: new Vec2(vx, vy),
        radius,
        mass: (radius * radius) * 0.005,
        restitution: 0.75,
        friction: 0.2,
        color: getRandomNeonColor()
      }));
    }
  };

  const spawnNewtonsCradle = () => {
    world?.clearBodies();
    const cx = world.bounds.width * 0.5;
    const cy = world.bounds.height * 0.45;
    const ballRadius = 26;
    const count = 6;
    const startX = cx - (count - 1) * ballRadius;

    // Align static bottom floor far below
    world?.addBody(new RigidBody({
      type: 'box',
      position: new Vec2(cx, world.bounds.height - 30),
      width: world.bounds.width * 0.9,
      height: 30,
      isStatic: true,
      color: '#475569'
    }));

    for (let i = 0; i < count; i++) {
      const x = startX + i * (ballRadius * 2);
      const isLeftmost = i === 0;

      world?.addBody(new RigidBody({
        type: 'circle',
        position: new Vec2(isLeftmost ? x - 180 : x, isLeftmost ? cy - 80 : cy),
        velocity: new Vec2(isLeftmost ? 500 : 0, isLeftmost ? 100 : 0),
        radius: ballRadius,
        mass: 5.0,
        restitution: 0.98,
        friction: 0.05,
        color: isLeftmost ? '#f43f5e' : '#38bdf8'
      }));
    }
  };

  const spawnChaosSandbox = () => {
    world?.clearBodies();
    const w = world.bounds.width;
    const h = world.bounds.height;

    // Sloped Ramps (Static)
    const ramp1 = new RigidBody({
      type: 'box',
      position: new Vec2(w * 0.25, h * 0.35),
      angle: Math.PI * 0.12,
      width: 320,
      height: 20,
      isStatic: true,
      color: '#64748b',
      friction: 0.2
    });
    world?.addBody(ramp1);

    const ramp2 = new RigidBody({
      type: 'box',
      position: new Vec2(w * 0.75, h * 0.55),
      angle: -Math.PI * 0.14,
      width: 340,
      height: 20,
      isStatic: true,
      color: '#64748b',
      friction: 0.2
    });
    world?.addBody(ramp2);

    // Static Bouncy Pegs and Bumpers
    const peg1 = new RigidBody({
      type: 'circle',
      position: new Vec2(w * 0.5, h * 0.4),
      radius: 35,
      isStatic: true,
      restitution: 0.9,
      color: '#e11d48'
    });
    world?.addBody(peg1);

    // Dynamic Objects Shower
    for (let i = 0; i < 25; i++) {
      const isCircle = i % 2 === 0;
      const x = w * 0.15 + (i * 28) % (w * 0.7);
      const y = 40 + i * 20;

      world?.addBody(new RigidBody({
        type: isCircle ? 'circle' : 'box',
        position: new Vec2(x, y),
        radius: 16 + Math.random() * 12,
        width: 32 + Math.random() * 16,
        height: 32 + Math.random() * 16,
        mass: 2.0,
        restitution: 0.6,
        friction: 0.3,
        color: getRandomNeonColor()
      }));
    }
  };

  const clearWorld = () => {
    world?.clearBodies();
    // Recreate bottom floor
    const cx = world.bounds.width * 0.5;
    const groundY = world.bounds.height - 30;
    world?.addBody(new RigidBody({
      type: 'box',
      position: new Vec2(cx, groundY),
      width: world.bounds.width * 0.95,
      height: 30,
      isStatic: true,
      color: '#475569',
      friction: 0.5
    }));
  };

  const controls = new UIControls(canvas, world, renderer, {
    spawnCircleStorm,
    spawnCratesAndDominoes,
    spawnPyramid,
    spawnNewtonsCradle,
    spawnChaosSandbox,
    clearWorld
  });

  // Spawn initial Default Scene
  spawnPyramid();

  // Animation Loop
  let lastTime = performance.now();

  const frameStep = (now: number) => {
    const rawDt = (now - lastTime) * 0.001;
    lastTime = now;

    // Mouse spring interaction update
    controls?.updateMousePhysics();

    // Physics Simulation Step
    world?.update(rawDt);

    // Canvas Rendering
    const mouseState = controls?.getMouseRenderState();
    renderer?.render(world, mouseState);

    // Telemetry Update
    controls?.updateTelemetry();

    requestAnimationFrame(frameStep);
  };

  requestAnimationFrame(frameStep);
}

// Bootstrap once DOM is ready
if (document.readyState === 'loading') {
  document.addEventListener('DOMContentLoaded', initApp);
} else {
  initApp();
}
