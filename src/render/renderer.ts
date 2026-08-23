import { Vec2 } from '../math';
import { RigidBody, PhysicsWorld } from '../physics';

export interface DebugOptions {
  showVelocities: boolean;
  showContacts: boolean;
  showAABBs: boolean;
  showGrid: boolean;
}

export interface MouseRenderState {
  isDragging: boolean;
  dragTarget: Vec2;
  mouseWorld: Vec2;
  selectedBody: RigidBody | null;
}

/**
 * Universal safe rounded rectangle path generator (compatible with 100% of browsers).
 */
function drawRoundedRectPath(
  ctx: CanvasRenderingContext2D,
  x: number,
  y: number,
  w: number,
  h: number,
  r: number = 4
): void {
  const rad = Math.min(r, w * 0.5, h * 0.5);
  ctx.beginPath();
  ctx.moveTo(x + rad, y);
  ctx.lineTo(x + w - rad, y);
  ctx.quadraticCurveTo(x + w, y, x + w, y + rad);
  ctx.lineTo(x + w, y + h - rad);
  ctx.quadraticCurveTo(x + w, y + h, x + w - rad, y + h);
  ctx.lineTo(x + rad, y + h);
  ctx.quadraticCurveTo(x, y + h, x, y + h - rad);
  ctx.lineTo(x, y + rad);
  ctx.quadraticCurveTo(x, y, x + rad, y);
  ctx.closePath();
}

/**
 * Renderer - HiDPI Hardware-Accelerated 2D Canvas Renderer.
 */
export class Renderer {
  public canvas: HTMLCanvasElement;
  public ctx: CanvasRenderingContext2D;
  public dpr: number = 1.0;
  public width: number = 1280;
  public height: number = 720;

  public debug: DebugOptions = {
    showVelocities: false,
    showContacts: true,
    showAABBs: false,
    showGrid: true
  };

  constructor(canvas: HTMLCanvasElement) {
    this.canvas = canvas;
    const context = canvas.getContext('2d', { alpha: false });
    if (!context) {
      throw new Error('Failed to get 2D rendering context from canvas');
    }
    this.ctx = context;
    this.updateDPR();
  }

  public updateDPR(): void {
    this.dpr = window?.devicePixelRatio || 1.0;
  }

  public resize(displayWidth: number, displayHeight: number): void {
    this.updateDPR();
    this.width = Math.max(300, displayWidth);
    this.height = Math.max(300, displayHeight);

    const ratio = this.dpr;
    this.canvas.width = Math.floor(this.width * ratio);
    this.canvas.height = Math.floor(this.height * ratio);
    this.canvas.style.width = `${this.width}px`;
    this.canvas.style.height = `${this.height}px`;

    this.ctx?.setTransform(1, 0, 0, 1, 0, 0);
    this.ctx?.scale(ratio, ratio);
  }

  /**
   * Main Render Pipeline.
   */
  public render(world: PhysicsWorld, mouseState?: MouseRenderState): void {
    const ctx = this.ctx;
    const w = this.width;
    const h = this.height;

    // 1. Dark Modern Background with Ambient Radial Glow
    this.renderBackground(ctx, w, h);

    // 2. Render Rigid Bodies
    const bodies = world.bodies;
    const bodyCount = bodies.length;

    for (let i = 0; i < bodyCount; i++) {
      const b = bodies.at(i);
      if (b) {
        this.renderBody(ctx, b);
      }
    }

    // 3. Render Particles with Additive Light Blending (Zero-GC)
    this.renderParticles(ctx, world);

    // 4. Render Mouse Spring Drag Interaction
    if (mouseState && mouseState.isDragging && mouseState.selectedBody) {
      this.renderMouseJoint(ctx, mouseState);
    }

    // 5. Render Debug Overlay Layer
    this.renderDebug(ctx, world);
  }

  private renderBackground(ctx: CanvasRenderingContext2D, w: number, h: number): void {
    // Deep slate background fill
    ctx.fillStyle = '#090d16';
    ctx.fillRect(0, 0, w, h);

    // Subtle Radial Ambient Glow in Center
    const grad = ctx.createRadialGradient(w * 0.5, h * 0.4, 40, w * 0.5, h * 0.5, Math.max(w, h) * 0.75);
    grad?.addColorStop(0, 'rgba(30, 41, 59, 0.55)');
    grad?.addColorStop(1, 'rgba(9, 13, 22, 0.98)');
    ctx.fillStyle = grad;
    ctx.fillRect(0, 0, w, h);

    // Modern Tech Grid Lines
    if (this.debug.showGrid) {
      ctx.strokeStyle = 'rgba(51, 65, 85, 0.22)';
      ctx.lineWidth = 1;
      ctx.beginPath();
      const gridSize = 50;

      for (let x = 0; x <= w; x += gridSize) {
        ctx.moveTo(x, 0);
        ctx.lineTo(x, h);
      }
      for (let y = 0; y <= h; y += gridSize) {
        ctx.moveTo(0, y);
        ctx.lineTo(w, y);
      }
      ctx.stroke();
    }
  }

  private renderBody(ctx: CanvasRenderingContext2D, b: RigidBody): void {
    ctx.save();
    ctx.translate(b.position.x, b.position.y);
    ctx.rotate(b.angle);

    if (b.type === 'circle') {
      this.renderCircle(ctx, b);
    } else {
      this.renderBox(ctx, b);
    }

    ctx.restore();
  }

  private renderCircle(ctx: CanvasRenderingContext2D, b: RigidBody): void {
    const r = b.radius;

    // Body Fill
    ctx.beginPath();
    ctx.arc(0, 0, r, 0, Math.PI * 2);
    ctx.fillStyle = b.isStatic ? 'rgba(71, 85, 105, 0.9)' : b.color;
    ctx.fill();

    // Outer Glow / Stroke
    ctx.lineWidth = b.isStatic ? 2 : 2.5;
    ctx.strokeStyle = b.isStatic ? '#94a3b8' : '#ffffff';
    ctx.stroke();

    // Orientation Indicator Line
    ctx.beginPath();
    ctx.moveTo(0, 0);
    ctx.lineTo(r - 2, 0);
    ctx.strokeStyle = 'rgba(255, 255, 255, 0.85)';
    ctx.lineWidth = 2;
    ctx.stroke();

    // Center pivot point
    ctx.beginPath();
    ctx.arc(0, 0, 3, 0, Math.PI * 2);
    ctx.fillStyle = '#ffffff';
    ctx.fill();
  }

  private renderBox(ctx: CanvasRenderingContext2D, b: RigidBody): void {
    const hw = b.halfExtents.x;
    const hh = b.halfExtents.y;
    const w = b.width;
    const h = b.height;

    // Universal safe rounded rectangle box fill
    drawRoundedRectPath(ctx, -hw, -hh, w, h, 4);

    ctx.fillStyle = b.isStatic ? 'rgba(71, 85, 105, 0.9)' : b.color;
    ctx.fill();

    // Highlight Stroke
    ctx.lineWidth = b.isStatic ? 2 : 2.5;
    ctx.strokeStyle = b.isStatic ? '#94a3b8' : '#ffffff';
    ctx.stroke();

    // Inner orientation marker
    ctx.beginPath();
    ctx.moveTo(0, 0);
    ctx.lineTo(hw * 0.7, 0);
    ctx.strokeStyle = 'rgba(255, 255, 255, 0.8)';
    ctx.lineWidth = 2;
    ctx.stroke();

    // Center pivot point
    ctx.beginPath();
    ctx.arc(0, 0, 3, 0, Math.PI * 2);
    ctx.fillStyle = '#ffffff';
    ctx.fill();
  }

  private renderParticles(ctx: CanvasRenderingContext2D, world: PhysicsWorld): void {
    const pool = world.particlePool;
    const particles = pool.particles;
    const count = particles.length;

    // Glowing Additive Light Mode
    ctx.save();
    ctx.globalCompositeOperation = 'lighter';

    for (let i = 0; i < count; i++) {
      const p = particles.at(i);
      if (!p || !p.active) continue;

      ctx.save();
      ctx.globalAlpha = p.alpha;
      ctx.fillStyle = p.color;

      ctx.beginPath();
      ctx.arc(p.position.x, p.position.y, Math.max(0.5, p.size), 0, Math.PI * 2);
      ctx.fill();
      ctx.restore();
    }

    ctx.restore();
  }

  private renderMouseJoint(ctx: CanvasRenderingContext2D, mouseState: MouseRenderState): void {
    const body = mouseState.selectedBody;
    if (!body) return;

    const mousePos = mouseState.mouseWorld;
    const bodyPos = body.position;

    // Draw elastic tension spring line
    ctx.save();
    ctx.beginPath();
    ctx.moveTo(mousePos.x, mousePos.y);
    ctx.lineTo(bodyPos.x, bodyPos.y);
    ctx.strokeStyle = '#38bdf8';
    ctx.lineWidth = 2.5;
    ctx.setLineDash([4, 4]);
    ctx.stroke();

    // Mouse Anchor point
    ctx.beginPath();
    ctx.arc(mousePos.x, mousePos.y, 6, 0, Math.PI * 2);
    ctx.fillStyle = '#38bdf8';
    ctx.fill();
    ctx.restore();
  }

  private renderDebug(ctx: CanvasRenderingContext2D, world: PhysicsWorld): void {
    const bodies = world.bodies;
    const bodyCount = bodies.length;

    // 1. AABB Bounding Boxes
    if (this.debug.showAABBs) {
      ctx.save();
      ctx.strokeStyle = 'rgba(234, 179, 8, 0.4)';
      ctx.lineWidth = 1;
      ctx.setLineDash([3, 3]);

      for (let i = 0; i < bodyCount; i++) {
        const b = bodies.at(i);
        if (b) {
          const min = b.aabbMin;
          const max = b.aabbMax;
          ctx.strokeRect(min.x, min.y, max.x - min.x, max.y - min.y);
        }
      }
      ctx.restore();
    }

    // 2. Velocity Vectors
    if (this.debug.showVelocities) {
      ctx.save();
      ctx.strokeStyle = '#10b981';
      ctx.lineWidth = 1.5;

      for (let i = 0; i < bodyCount; i++) {
        const b = bodies.at(i);
        if (b && !b.isStatic) {
          const px = b.position.x;
          const py = b.position.y;
          const vx = b.velocity.x * 0.1;
          const vy = b.velocity.y * 0.1;

          ctx.beginPath();
          ctx.moveTo(px, py);
          ctx.lineTo(px + vx, py + vy);
          ctx.stroke();
        }
      }
      ctx.restore();
    }

    // 3. Contact Manifolds & Collision Normals
    if (this.debug.showContacts) {
      const manifolds = world.activeManifolds;
      const mCount = manifolds.length;

      ctx.save();
      for (let i = 0; i < mCount; i++) {
        const m = manifolds.at(i);
        if (!m) continue;

        const count = m.contactCount;
        const norm = m.normal;

        for (let c = 0; c < count; c++) {
          const cp = c === 0 ? m.contacts[0] : m.contacts[1];

          // Contact Crosshair (Cyan)
          ctx.strokeStyle = '#06b6d4';
          ctx.lineWidth = 1.5;
          ctx.beginPath();
          ctx.arc(cp.x, cp.y, 4, 0, Math.PI * 2);
          ctx.stroke();

          // Normal vector arrow (Orange)
          ctx.strokeStyle = '#f97316';
          ctx.lineWidth = 2;
          ctx.beginPath();
          ctx.moveTo(cp.x, cp.y);
          ctx.lineTo(cp.x + norm.x * 16, cp.y + norm.y * 16);
          ctx.stroke();
        }
      }
      ctx.restore();
    }
  }
}
