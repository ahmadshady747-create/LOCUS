import { Vec2 } from '../math';

export type BodyType = 'circle' | 'box';

export interface BodyOptions {
  type?: BodyType;
  position?: Vec2;
  velocity?: Vec2;
  angle?: number;
  angularVelocity?: number;
  mass?: number;
  radius?: number;
  width?: number;
  height?: number;
  restitution?: number;
  friction?: number;
  isStatic?: boolean;
  color?: string;
}

let nextBodyId = 1;

/**
 * RigidBody - 2D Physical Rigid Body with In-Place Kinematics and Zero-GC Vertex Buffers.
 */
export class RigidBody {
  public id: number;
  public type: BodyType;

  // Translational Kinematics
  public position: Vec2;
  public velocity: Vec2;
  public force: Vec2;

  // Rotational Kinematics
  public angle: number;
  public angularVelocity: number;
  public torque: number;

  // Physical Properties
  public mass: number;
  public invMass: number;
  public inertia: number;
  public invInertia: number;
  public restitution: number;
  public friction: number;
  public isStatic: boolean;

  // Geometric Parameters
  public radius: number;
  public width: number;
  public height: number;
  public halfExtents: Vec2;

  // Pre-allocated Box Geometry Buffers (Zero-GC)
  public vertices: [Vec2, Vec2, Vec2, Vec2];
  public axes: [Vec2, Vec2];
  public aabbMin: Vec2;
  public aabbMax: Vec2;

  // Visual Customization
  public color: string;

  constructor(options: BodyOptions = {}) {
    this.id = nextBodyId++;
    this.type = options?.type || 'circle';

    this.position = options?.position?.clone() ?? new Vec2();
    this.velocity = options?.velocity?.clone() ?? new Vec2();
    this.force = new Vec2();

    this.angle = options?.angle || 0;
    this.angularVelocity = options?.angularVelocity || 0;
    this.torque = 0;

    this.radius = options?.radius || 20;
    this.width = options?.width || 40;
    this.height = options?.height || 40;
    this.halfExtents = new Vec2(this.width * 0.5, this.height * 0.5);

    this.restitution = typeof options?.restitution === 'number' ? Math.max(0, Math.min(1, options.restitution)) : 0.4;
    this.friction = typeof options?.friction === 'number' ? Math.max(0, Math.min(1, options.friction)) : 0.3;
    this.isStatic = Boolean(options?.isStatic);

    this.color = options?.color || '#38bdf8';

    // Pre-allocate 4 Box vertices and 2 normal axes for SAT
    this.vertices = [new Vec2(), new Vec2(), new Vec2(), new Vec2()];
    this.axes = [new Vec2(1, 0), new Vec2(0, 1)];
    this.aabbMin = new Vec2();
    this.aabbMax = new Vec2();

    this.mass = 1.0;
    this.invMass = 1.0;
    this.inertia = 1.0;
    this.invInertia = 1.0;

    const initialMass = options?.mass || (this.type === 'circle' ? Math.PI * this.radius * this.radius * 0.001 : this.width * this.height * 0.001);
    this.setMass(initialMass);

    if (this.isStatic) {
      this.setStatic(true);
    }

    this.updateTransform();
  }

  public setStatic(isStatic: boolean): void {
    this.isStatic = isStatic;
    if (isStatic) {
      this.invMass = 0;
      this.invInertia = 0;
      this.velocity.set(0, 0);
      this.angularVelocity = 0;
    } else {
      this.setMass(this.mass > 0 ? this.mass : 1.0);
    }
  }

  public setMass(m: number): void {
    this.mass = Math.max(0.0001, m);

    if (this.isStatic) {
      this.invMass = 0;
      this.invInertia = 0;
      return;
    }

    if (this.mass != 0) {
      this.invMass = 1.0 / this.mass;
    } else {
      this.invMass = 0;
    }

    if (this.type === 'circle') {
      // Inertia for Solid Circle: I = 0.5 * m * r^2
      this.inertia = 0.5 * this.mass * this.radius * this.radius;
    } else {
      // Inertia for Solid Box: I = (1/12) * m * (w^2 + h^2)
      this.inertia = (1.0 / 12.0) * this.mass * (this.width * this.width + this.height * this.height);
    }

    if (this.inertia != 0) {
      this.invInertia = 1.0 / this.inertia;
    } else {
      this.invInertia = 0;
    }
  }

  public applyForce(f: Vec2): void {
    if (this.isStatic) return;
    this.force.addInPlace(f);
  }

  public applyTorque(t: number): void {
    if (this.isStatic) return;
    this.torque += t;
  }

  public applyImpulse(j: Vec2, r: Vec2): void {
    if (this.isStatic) return;

    // Linear impulse: deltaV = J * invMass
    this.velocity.addScaledInPlace(j, this.invMass);

    // Angular impulse: deltaW = (r x J) * invInertia
    const torqueImpulse = Vec2.cross(r, j);
    this.angularVelocity += torqueImpulse * this.invInertia;
  }

  public updateTransform(): void {
    if (this.type === 'circle') {
      this.aabbMin.set(this.position.x - this.radius, this.position.y - this.radius);
      this.aabbMax.set(this.position.x + this.radius, this.position.y + this.radius);
      return;
    }

    // Box Oriented Bounding Box (OBB) transformation
    const cos = Math.cos(this.angle);
    const sin = Math.sin(this.angle);

    const hw = this.halfExtents.x;
    const hh = this.halfExtents.y;

    // Calculate 4 rotated vertices in world space
    // 0: top-left (-hw, -hh)
    this.vertices[0].set(
      this.position.x + (-hw * cos - -hh * sin),
      this.position.y + (-hw * sin + -hh * cos)
    );
    // 1: top-right (hw, -hh)
    this.vertices[1].set(
      this.position.x + (hw * cos - -hh * sin),
      this.position.y + (hw * sin + -hh * cos)
    );
    // 2: bottom-right (hw, hh)
    this.vertices[2].set(
      this.position.x + (hw * cos - hh * sin),
      this.position.y + (hw * sin + hh * cos)
    );
    // 3: bottom-left (-hw, hh)
    this.vertices[3].set(
      this.position.x + (-hw * cos - hh * sin),
      this.position.y + (-hw * sin + hh * cos)
    );

    // Primary edge axes for SAT: Axis 0 = (cos, sin), Axis 1 = (-sin, cos)
    this.axes[0].set(cos, sin);
    this.axes[1].set(-sin, cos);

    // Compute AABB for fast broadphase
    const v0 = this.vertices[0];
    const v1 = this.vertices[1];
    const v2 = this.vertices[2];
    const v3 = this.vertices[3];

    const minX = Math.min(v0.x, v1.x, v2.x, v3.x);
    const maxX = Math.max(v0.x, v1.x, v2.x, v3.x);
    const minY = Math.min(v0.y, v1.y, v2.y, v3.y);
    const maxY = Math.max(v0.y, v1.y, v2.y, v3.y);

    this.aabbMin.set(minX, minY);
    this.aabbMax.set(maxX, maxY);
  }

  public containsPoint(p: Vec2): boolean {
    if (this.type === 'circle') {
      return this.position.distSq(p) <= this.radius * this.radius;
    }

    // Transform world point into box local unrotated coordinates
    const dx = p.x - this.position.x;
    const dy = p.y - this.position.y;
    const cos = Math.cos(-this.angle);
    const sin = Math.sin(-this.angle);

    const localX = Math.abs(dx * cos - dy * sin);
    const localY = Math.abs(dx * sin + dy * cos);

    return localX <= this.halfExtents.x && localY <= this.halfExtents.y;
  }

  public integrateForces(gravity: Vec2, wind: Vec2, dt: number): void {
    if (this.isStatic) return;

    // v += (gravity + wind + force * invMass) * dt
    this.velocity.addScaledInPlace(gravity, dt);
    this.velocity.addScaledInPlace(wind, dt);
    this.velocity.addScaledInPlace(this.force, this.invMass * dt);

    // angularVelocity += (torque * invInertia) * dt
    this.angularVelocity += this.torque * this.invInertia * dt;

    // Reset force and torque accumulators
    this.force.set(0, 0);
    this.torque = 0;
  }

  public integrateVelocity(dt: number, linearDamping: number = 0.999, angularDamping: number = 0.995): void {
    if (this.isStatic) return;

    // Position += Velocity * dt
    this.position.addScaledInPlace(this.velocity, dt);

    // Angle += AngularVelocity * dt
    this.angle += this.angularVelocity * dt;

    // Apply linear and angular damping
    this.velocity.scaleInPlace(linearDamping);
    this.angularVelocity *= angularDamping;

    // Update transformed vertices & AABB
    this.updateTransform();
  }
}
