/*
By: <Draydon Levesque>
Date: 2025-12-02
Program Details: <Plinko slot game>
*/

mod modules;

use crate::modules::scale::{screen_to_virtual, use_virtual_resolution};
use crate::modules::text_button::TextButton;
use macroquad::prelude::*;
// Import Rapier2D physics engine - provides 2D rigid body physics simulation
use rapier2d::prelude::*;
use miniquad::date;

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
   
    // Define gravity vector: no horizontal force (x=0), downward force (y=800)
    // Positive Y points down in screen coordinates
    let gravity = vector![0.0, 800.0];
    
    // Integration parameters control the physics simulation timestep and solver iterations
    // Default values provide a good balance between accuracy and performance
    let integration_params = IntegrationParameters::default();

    // PhysicsPipeline orchestrates all physics computations each frame
    let mut pipeline = PhysicsPipeline::new();
    
    // IslandManager groups bodies that can interact for more efficient simulation
    // Bodies that are far apart won't be tested for collisions
    let mut island_manager = IslandManager::new();
    
    // BroadPhase performs coarse collision detection to quickly eliminate impossible collisions
    let mut broad_phase = BroadPhase::new();
    
    // NarrowPhase performs precise collision detection on pairs identified by broad phase
    let mut narrow_phase = NarrowPhase::new();
    
    // RigidBodySet stores all rigid bodies in the simulation (balls, ground, pegs)
    let mut bodies = RigidBodySet::new();
    
    // ColliderSet stores all collision shapes attached to rigid bodies
    let mut colliders = ColliderSet::new();
    
    // ImpulseJointSet manages impulse-based joints (e.g., hinges, springs)
    // Not used in this plinko game but required by the physics pipeline
    let mut joints = ImpulseJointSet::new();
    
    // MultibodyJointSet manages reduced-coordinate joints for articulated bodies
    // Not used in this plinko game but required by the physics pipeline
    let mut multibody_joints = MultibodyJointSet::new();
    
    // CCDSolver enables Continuous Collision Detection to prevent fast objects
    // from tunneling through thin obstacles
    let mut ccd = CCDSolver::new();

    // ---------------- Ground ------------------------------------------------
    // Create a fixed (immovable) rigid body for the ground platform
    // RigidBodyBuilder::fixed() creates a body with infinite mass that won't move
    // translation() sets the center position at (400, 580) in screen coordinates
    let ground_body = RigidBodyBuilder::fixed().translation(vector![400.0, 580.0]).build();
    
    // Create a rectangular collision shape (cuboid) for the ground
    // cuboid(400, 20) creates a box with half-extents: 400 units wide, 20 units tall
    // friction(0.4) sets surface friction to slow down sliding objects
    let ground_collider = ColliderBuilder::cuboid(400.0, 20.0).friction(0.4).build();
    
    // Insert the ground body into the physics world and get its handle
    let gh = bodies.insert(ground_body);
    
    // Attach the collider to the ground body using the body's handle
    colliders.insert_with_parent(ground_collider, gh, &mut bodies);

    // ---------------- Pegs --------------------------------------------------
    let peg_radius = 10.0;
    // Create a 10x12 grid of pegs in a staggered pattern (like traditional plinko)
    for row in 0..10 {
        let y = 120.0 + row as f32 * 40.0;
        for col in 0..12 {
            // Offset every other row to create a zigzag pattern
            let x_offset = if row % 2 == 0 { 30.0 } else { 0.0 };
            let x = 80.0 + col as f32 * 60.0 + x_offset;

            // Create a fixed (stationary) rigid body for each peg at position (x, y)
            let peg_body = RigidBodyBuilder::fixed().translation(vector![x, y]).build();
            
            // Create a circular collision shape (ball) with radius 10.0
            // restitution(0.5) makes pegs bouncy - balls will bounce off with 50% energy retained
            let peg_collider = ColliderBuilder::ball(peg_radius).restitution(0.5).build();
            
            // Insert the peg body into the physics world and get its handle
            let ph = bodies.insert(peg_body);
            
            // Attach the circular collider to the peg body
            colliders.insert_with_parent(peg_collider, ph, &mut bodies);
        }
    }

    // First ball
    spawn_ball(&mut bodies, &mut colliders, 400.0, 50.0);

    let btn_text = TextButton::new(800.0, 200.0, 200.0, 60.0, "Click Me", BLUE, GREEN, 30);
rand::srand(date::now() as u64);
let mut place =0;
    loop {
        use_virtual_resolution(1024.0, 768.0);
        clear_background(BLACK);
        

        // Click to spawn new ball
        if btn_text.click() {
            // Dice roll between 1 and 6
let dice = rand::gen_range(1, 7);
if dice==1 {
    place =201;
}else if dice==2 {
    place =300;
}else if dice==3 {
    place =400;
}else if dice==4 {
    place =501;
}else if dice==5 {
    place =600;
}else if dice==6 {
    place =700;
    
}
            spawn_ball(&mut bodies, &mut colliders, place as f32, 50.0);
        }

        // ---- Physics step (Rapier 0.18) ----
        // Advance the physics simulation by one timestep
        // This computes forces, integrates velocities, detects collisions, and resolves them
        pipeline.step(
            &gravity,              // Apply gravity force to all dynamic bodies
            &integration_params,   // Timestep and solver settings
            &mut island_manager,   // Manages groups of interacting bodies
            &mut broad_phase,      // Coarse collision detection
            &mut narrow_phase,     // Precise collision detection and contact generation
            &mut bodies,           // All rigid bodies (balls, pegs, ground)
            &mut colliders,        // All collision shapes attached to bodies
            &mut joints,           // Joint constraints (not used here)
            &mut multibody_joints, // Articulated body joints (not used here)
            &mut ccd,              // Continuous collision detection solver
            None,                  // Optional query pipeline for raycasts/shape casts
            &(),                   // Physics hooks for custom collision filtering
            &(),                   // Event handler for collision/contact events
        );

        // ---- Draw all bodies ----
        // Iterate through all rigid bodies in the physics simulation
        for (_handle, body) in bodies.iter() {
            // Get the body's position (translation vector from Rapier)
            let pos = body.translation();
            
            // Get the body's rotation angle in radians
            let rot = body.rotation().angle();

            // Each body can have multiple colliders attached to it
            // Iterate through all colliders attached to this body
            for col_handle in body.colliders() {
                let collider = &colliders[*col_handle];
                
                // Get the geometric shape of this collider
                let shape = collider.shape();

                // Check if the shape is a ball (sphere/circle)
                // Used for drawing plinko balls and pegs
                if let Some(ball) = shape.as_ball() {
                    draw_circle(pos.x, pos.y, ball.radius, GREEN);
                }

                // Check if the shape is a cuboid (rectangle/box)
                // Used for drawing the ground platform
                if let Some(cuboid) = shape.as_cuboid() {
                    // Get half-extents (distance from center to edge)
                    let hx = cuboid.half_extents.x;
                    let hy = cuboid.half_extents.y;

                    // Draw rectangle using top-left corner position and full dimensions
                    // Apply rotation from the physics body
                    draw_rectangle_ex(pos.x - hx, pos.y - hy, hx * 2.0, hy * 2.0, DrawRectangleParams { rotation: rot, ..Default::default() });
                }
            }
        }

        next_frame().await;
    }
}

// -------------------- Spawn Function ----------------------------------------
/// Spawns a new dynamic ball at the specified position in the physics simulation
fn spawn_ball(bodies: &mut RigidBodySet, colliders: &mut ColliderSet, x: f32, y: f32) {
    // Create a dynamic (movable) rigid body that responds to forces and gravity
    // RigidBodyBuilder::dynamic() creates a body that can move and rotate
    // translation() sets the initial position at (x, y)
    // linvel() sets the initial linear velocity to zero (ball starts stationary)
    let body = RigidBodyBuilder::dynamic().translation(vector![x, y]).linvel(vector![0.0, 0.0]).build();

    // Create a circular collision shape with radius 12.0
    // restitution(0.4) makes the ball somewhat bouncy (retains 40% energy on bounce)
    // friction(0.2) provides slight resistance when sliding against surfaces
    let collider = ColliderBuilder::ball(12.0).restitution(0.4).friction(0.2).build();

    // Insert the ball body into the physics world and get its handle
    let bh = bodies.insert(body);
    
    // Attach the circular collider to the ball body
    // The collider inherits the position and movement of its parent body
    colliders.insert_with_parent(collider, bh, bodies);
}
