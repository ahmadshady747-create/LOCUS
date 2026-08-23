import { Vec2 } from '../math';
import { RigidBody, PhysicsWorld } from '../physics';
import { Renderer } from '../render';

export interface SceneCallbacks {
  spawnCircleStorm: () => void;
  spawnCratesAndDominoes: () => void;
  spawnPyramid: () => void;
  spawnNewtonsCradle: () => void;
  spawnChaosSandbox: () => void;
  clearWorld: () => void;
}

/**
 * UIControls - Manages Glassmorphism HUD Controls, Mouse Physics, and Performance Telemetry.
 */
export class UIControls {
  private canvas: HTMLCanvasElement;
  private world: PhysicsWorld;
  private renderer: Renderer;
  private callbacks: SceneCallbacks;

  // Mouse Physics Interaction State
  public isDragging: boolean = false;
  public selectedBody: RigidBody | null = null;
  public mouseWorld: Vec2 = new Vec2();
  public prevMouseWorld: Vec2 = new Vec2();
  public mouseVelocity: Vec2 = new Vec2();
  public localAnchor: Vec2 = new Vec2();

  // Performance Telemetry
  private lastFpsUpdate: number = performance.now();
  private frameCount: number = 0;
  public currentFps: number = 60;

  constructor(
    canvas: HTMLCanvasElement,
    world: PhysicsWorld,
    renderer: Renderer,
    callbacks: SceneCallbacks
  ) {
    this.canvas = canvas;
    this.world = world;
    this.renderer = renderer;
    this.callbacks = callbacks;

    this.initMouseListeners();
    this.bindControlElements();
  }

  private initMouseListeners(): void {
    const canvas = this.canvas;

    const getCanvasPos = (evt: MouseEvent | Touch): Vec2 => {
      const rect = canvas.getBoundingClientRect();
      const clientX = evt.clientX;
      const clientY = evt.clientY;
      const x = clientX - rect.left;
      const y = clientY - rect.top;
      return new Vec2(x, y);
    };

    // Mouse Down
    canvas.addEventListener('mousedown', (evt: MouseEvent) => {
      const pos = getCanvasPos(evt);
      this.mouseWorld?.copy(pos);
      this.prevMouseWorld?.copy(pos);
      this.mouseVelocity?.set(0, 0);

      // Right Click -> Radial Shockwave Blast
      if (evt.button === 2) {
        evt.preventDefault();
        this.world?.applyExplosion(pos, 220, 900);
        return;
      }

      // Left Click -> Body Grabbing
      if (evt.button === 0) {
        const body = this.world.getBodyAt(pos);
        if (body && !body.isStatic) {
          this.isDragging = true;
          this.selectedBody = body;
          this.localAnchor?.set(pos.x - body.position.x, pos.y - body.position.y);
        }
      }
    });

    // Context Menu Disable
    canvas.addEventListener('contextmenu', (evt: MouseEvent) => {
      evt.preventDefault();
    });

    // Mouse Move
    window.addEventListener('mousemove', (evt: MouseEvent) => {
      const pos = getCanvasPos(evt);
      this.mouseVelocity?.set(pos.x - this.mouseWorld.x, pos.y - this.mouseWorld.y);
      this.prevMouseWorld?.copy(this.mouseWorld);
      this.mouseWorld?.copy(pos);
    });

    // Mouse Up
    window.addEventListener('mouseup', () => {
      if (this.isDragging && this.selectedBody) {
        // Fling body with mouse velocity
        const body = this.selectedBody;
        body.velocity?.addScaledInPlace(this.mouseVelocity, 12);
      }
      this.isDragging = false;
      this.selectedBody = null;
    });

    // Touch Support
    canvas.addEventListener('touchstart', (evt: TouchEvent) => {
      const touch = evt.touches.item(0);
      if (!touch) return;
      const pos = getCanvasPos(touch);
      this.mouseWorld?.copy(pos);

      const body = this.world.getBodyAt(pos);
      if (body && !body.isStatic) {
        this.isDragging = true;
        this.selectedBody = body;
        this.localAnchor?.set(pos.x - body.position.x, pos.y - body.position.y);
      }
    }, { passive: true });

    window.addEventListener('touchmove', (evt: TouchEvent) => {
      const touch = evt.touches.item(0);
      if (!touch) return;
      const pos = getCanvasPos(touch);
      this.mouseVelocity?.set(pos.x - this.mouseWorld.x, pos.y - this.mouseWorld.y);
      this.mouseWorld?.copy(pos);
    }, { passive: true });

    window.addEventListener('touchend', () => {
      this.isDragging = false;
      this.selectedBody = null;
    });
  }

  /**
   * Applies Spring Force while Dragging a Rigid Body.
   */
  public updateMousePhysics(): void {
    if (!this.isDragging || !this.selectedBody) return;

    const b = this.selectedBody;
    const targetPos = this.mouseWorld;

    // Linear Spring Pull
    const dx = targetPos.x - b.position.x;
    const dy = targetPos.y - b.position.y;

    const stiffness = 30.0;
    const damping = 0.85;

    b.velocity.x = (b.velocity.x + dx * stiffness * (1.0 / 60.0)) * damping;
    b.velocity.y = (b.velocity.y + dy * stiffness * (1.0 / 60.0)) * damping;
  }

  private bindControlElements(): void {
    // Gravity Slider
    const gravSlider = document.getElementById('slider-gravity') as HTMLInputElement | null;
    const gravVal = document.getElementById('val-gravity');
    gravSlider?.addEventListener('input', () => {
      const val = parseFloat(gravSlider.value);
      this.world.gravity.y = val;
      if (gravVal) gravVal.textContent = `${val.toFixed(0)} px/s²`;
    });

    // Wind Slider
    const windSlider = document.getElementById('slider-wind') as HTMLInputElement | null;
    const windVal = document.getElementById('val-wind');
    windSlider?.addEventListener('input', () => {
      const val = parseFloat(windSlider.value);
      this.world.windForce.x = val;
      if (windVal) windVal.textContent = `${val.toFixed(0)} px/s²`;
    });

    // Restitution Slider
    const restSlider = document.getElementById('slider-restitution') as HTMLInputElement | null;
    const restVal = document.getElementById('val-restitution');
    restSlider?.addEventListener('input', () => {
      const val = parseFloat(restSlider.value);
      const bodies = this.world.bodies;
      const count = bodies.length;
      for (let i = 0; i < count; i++) {
        const b = bodies.at(i);
        if (b) b.restitution = val;
      }
      if (restVal) restVal.textContent = val.toFixed(2);
    });

    // Friction Slider
    const fricSlider = document.getElementById('slider-friction') as HTMLInputElement | null;
    const fricVal = document.getElementById('val-friction');
    fricSlider?.addEventListener('input', () => {
      const val = parseFloat(fricSlider.value);
      const bodies = this.world.bodies;
      const count = bodies.length;
      for (let i = 0; i < count; i++) {
        const b = bodies.at(i);
        if (b) b.friction = val;
      }
      if (fricVal) fricVal.textContent = val.toFixed(2);
    });

    // Solver Iterations Slider
    const iterSlider = document.getElementById('slider-iterations') as HTMLInputElement | null;
    const iterVal = document.getElementById('val-iterations');
    iterSlider?.addEventListener('input', () => {
      const val = parseInt(iterSlider.value, 10);
      this.world.solverIterations = val;
      if (iterVal) iterVal.textContent = `${val}`;
    });

    // Time Scale Slider
    const timeSlider = document.getElementById('slider-timescale') as HTMLInputElement | null;
    const timeVal = document.getElementById('val-timescale');
    timeSlider?.addEventListener('input', () => {
      const val = parseFloat(timeSlider.value);
      this.world.timeScale = val;
      if (timeVal) timeVal.textContent = `${val.toFixed(1)}x`;
    });

    // Pause / Play Button
    const btnPause = document.getElementById('btn-pause');
    btnPause?.addEventListener('click', () => {
      this.world.isPaused = !this.world.isPaused;
      if (btnPause) {
        btnPause.textContent = this.world.isPaused ? '▶ Resume' : '⏸ Pause';
      }
    });

    // Shockwave Blast Button
    const btnBlast = document.getElementById('btn-blast');
    btnBlast?.addEventListener('click', () => {
      const cx = this.world.bounds.width * 0.5;
      const cy = this.world.bounds.height * 0.5;
      this.world.applyExplosion(new Vec2(cx, cy), 350, 1200);
    });

    // Preset Scene Buttons
    document.getElementById('btn-scene-circles')?.addEventListener('click', () => this.callbacks.spawnCircleStorm());
    document.getElementById('btn-scene-crates')?.addEventListener('click', () => this.callbacks.spawnCratesAndDominoes());
    document.getElementById('btn-scene-pyramid')?.addEventListener('click', () => this.callbacks.spawnPyramid());
    document.getElementById('btn-scene-cradle')?.addEventListener('click', () => this.callbacks.spawnNewtonsCradle());
    document.getElementById('btn-scene-chaos')?.addEventListener('click', () => this.callbacks.spawnChaosSandbox());
    document.getElementById('btn-clear')?.addEventListener('click', () => this.callbacks.clearWorld());

    // Debug Checkboxes
    const chkVel = document.getElementById('chk-debug-vel') as HTMLInputElement | null;
    chkVel?.addEventListener('change', () => {
      this.renderer.debug.showVelocities = !!chkVel.checked;
    });

    const chkContacts = document.getElementById('chk-debug-contacts') as HTMLInputElement | null;
    chkContacts?.addEventListener('change', () => {
      this.renderer.debug.showContacts = !!chkContacts.checked;
    });

    const chkAABBs = document.getElementById('chk-debug-aabbs') as HTMLInputElement | null;
    chkAABBs?.addEventListener('change', () => {
      this.renderer.debug.showAABBs = !!chkAABBs.checked;
    });
  }

  /**
   * Updates Real-Time Diagnostics HUD.
   */
  public updateTelemetry(): void {
    this.frameCount++;
    const now = performance.now();
    const elapsed = now - this.lastFpsUpdate;

    if (elapsed >= 500) {
      if (elapsed != 0) {
        this.currentFps = Math.round((this.frameCount * 1000) / elapsed);
      }
      this.frameCount = 0;
      this.lastFpsUpdate = now;

      // Update HUD elements
      const elFps = document.getElementById('stat-fps');
      const elBodies = document.getElementById('stat-bodies');
      const elParticles = document.getElementById('stat-particles');
      const elContacts = document.getElementById('stat-contacts');

      if (elFps) elFps.textContent = `${this.currentFps} FPS`;
      if (elBodies) elBodies.textContent = `${this.world.bodies.length}`;
      if (elParticles) elParticles.textContent = `${this.world.particlePool.activeCount}`;
      if (elContacts) elContacts.textContent = `${this.world.activeManifolds.length}`;
    }
  }

  public getMouseRenderState() {
    return {
      isDragging: this.isDragging,
      dragTarget: this.localAnchor,
      mouseWorld: this.mouseWorld,
      selectedBody: this.selectedBody
    };
  }
}
