/*
By: <Draydon Levesque>
Date: 2025-12-02
Program Details: <Plinko slot game>
*/


mod modules;

use crate::modules::scale::{use_virtual_resolution, screen_to_virtual};
use macroquad::prelude::*;
use rapier2d::prelude::*;

/// Set up window settings before the app runs
fn window_conf() -> Conf {
    Conf {
        window_title: "plinko2".to_string(),
        window_width: 1024,
        window_height: 768,
        fullscreen: false,
        high_dpi: true,
        window_resizable: true,
        sample_count: 4, // MSAA
        ..Default::default()
    }
}

#[macroquad::main(window_conf)]
async fn main() {
    // -------- Physics Init (Rapier 0.18 compatible) -------------------------
    let gravity = vector![0.0, 800.0];
    let integration_params = IntegrationParameters::default();

    let mut pipeline = PhysicsPipeline::new();
    let mut island_manager = IslandManager::new();
    let mut broad_phase = BroadPhase::new();
    let mut narrow_phase = NarrowPhase::new();
    let mut bodies = RigidBodySet::new();
    let mut colliders = ColliderSet::new();
    let mut joints = ImpulseJointSet::new();
    let mut multibody_joints = MultibodyJointSet::new();
    let mut ccd = CCDSolver::new();

    // ---------------- Ground ------------------------------------------------
    let ground_body = RigidBodyBuilder::fixed().translation(vector![400.0, 580.0]).build();
    let ground_collider = ColliderBuilder::cuboid(400.0, 20.0).friction(0.4).build();
    let gh = bodies.insert(ground_body);
    colliders.insert_with_parent(ground_collider, gh, &mut bodies);

    // ---------------- Pegs --------------------------------------------------
    let peg_radius = 10.0;
    for row in 0..10 {
        let y = 120.0 + row as f32 * 40.0;
        for col in 0..12 {
            let x_offset = if row % 2 == 0 { 30.0 } else { 0.0 };
            let x = 80.0 + col as f32 * 60.0 + x_offset;

            let peg_body = RigidBodyBuilder::fixed().translation(vector![x, y]).build();
            let peg_collider = ColliderBuilder::ball(peg_radius).restitution(0.5).build();
            let ph = bodies.insert(peg_body);
            colliders.insert_with_parent(peg_collider, ph, &mut bodies);
        }
    }

    // First ball
    spawn_ball(&mut bodies, &mut colliders, 400.0, 50.0);

    loop {
        use_virtual_resolution(1024.0, 768.0);
        clear_background(BLACK);

        // Click to spawn new ball
      if is_mouse_button_pressed(MouseButton::Left) {
    let (mx, my) = mouse_position();
    let (vx, vy) = screen_to_virtual(mx, my); // now correctly maps to camera/world
    spawn_ball(&mut bodies, &mut colliders, vx, vy);
}

        // ---- Physics step (Rapier 0.18) ----
        pipeline.step(
            &gravity,
            &integration_params,
            &mut island_manager,
            &mut broad_phase,
            &mut narrow_phase,
            &mut bodies,
            &mut colliders,
            &mut joints,
            &mut multibody_joints,
            &mut ccd,
            None,
            &(),
            &(),
        );

        // ---- Draw all bodies ----
        for (_handle, body) in bodies.iter() {
            let pos = body.translation();
            let rot = body.rotation().angle();

            for col_handle in body.colliders() {
                let collider = &colliders[*col_handle];
                let shape = collider.shape();

                if let Some(ball) = shape.as_ball() {
                    draw_circle(pos.x, pos.y, ball.radius, YELLOW);
                }

                if let Some(cuboid) = shape.as_cuboid() {
                    let hx = cuboid.half_extents.x;
                    let hy = cuboid.half_extents.y;

                    draw_rectangle_ex(
                        pos.x - hx,
                        pos.y - hy,
                        hx * 2.0,
                        hy * 2.0,
                        DrawRectangleParams { rotation: rot, ..Default::default() },
                    );
                }
            }
        }

        next_frame().await;
    }
}

// -------------------- Spawn Function ----------------------------------------
fn spawn_ball(bodies: &mut RigidBodySet, colliders: &mut ColliderSet, x: f32, y: f32) {
    let body = RigidBodyBuilder::dynamic()
        .translation(vector![x, y])
        .linvel(vector![0.0, 0.0])
        .build();

    let collider = ColliderBuilder::ball(12.0)
        .restitution(0.4)
        .friction(0.2)
        .build();

    let bh = bodies.insert(body);
    colliders.insert_with_parent(collider, bh, bodies);
}