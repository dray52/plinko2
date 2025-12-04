/*
By: Draydon Levesque
Date: 2025-12-04
Program: Plinko Slot Game
Details: Spawns balls, squares, triangles with physics

OVERVIEW:
This is a 2D physics-based Plinko game using the Macroquad graphics library and Rapier2D physics engine.
Players can spawn three types of objects (balls, squares, triangles) that fall through a grid of pegs
and collide with obstacles before landing on a ground surface.
*/

// Import custom modules for scaling and UI button management
mod modules;

// Import virtual resolution scaling utility for responsive rendering across different screen sizes
use crate::modules::scale::use_virtual_resolution;
// Import custom TextButton UI component that handles clickable button rendering and interaction
use crate::modules::text_button::TextButton;
// Import all common macroquad graphics and input functionality (drawing, colors, input handling)
use macroquad::prelude::*;
// Import Rapier2D physics engine components for rigid bodies, collision detection, and physics simulation
use rapier2d::prelude::*;
// Import date/time functionality for random seed initialization to ensure non-deterministic gameplay
use miniquad::date;

// ---------------------------
// WINDOW CONFIG
// ---------------------------
/// Configures the Macroquad window properties before the game starts.
/// This function is called automatically by the #[macroquad::main] macro.
/// 
/// Parameters configured:
/// - window_title: The text displayed in the window title bar
/// - window_width/height: Initial window dimensions in pixels (1024x768)
/// - fullscreen: Disabled to allow windowed mode
/// - high_dpi: Enables support for high-resolution displays
/// - window_resizable: Allows the user to resize the window
/// - sample_count: Anti-aliasing quality (4x MSAA provides smooth edges)
fn window_conf() -> Conf {
    Conf {
        window_title: "Plinko Slot Game".to_string(),
        window_width: 1024,
        window_height: 768,
        fullscreen: false,
        high_dpi: true,
        window_resizable: true,
        sample_count: 4, // 4x multi-sample anti-aliasing for smooth edge rendering
        ..Default::default()
    }
}

// ---------------------------
// MAIN GAME ENTRY POINT
// ---------------------------
/// The main async game function decorated with the Macroquad macro.
/// The #[macroquad::main] attribute sets up the graphics context and game loop,
/// calling this function once at startup. The async keyword allows for asynchronous
/// rendering operations (like next_frame().await).
#[macroquad::main(window_conf)]
async fn main() {
    // ---------------------------
    // PHYSICS WORLD INITIALIZATION
    // ---------------------------
    // Define gravity vector: x=0 (no horizontal gravity), y=800 (strong downward pull)
    // This mimics real-world gravity pulling objects downward with consistent acceleration
    let gravity = vector![0.0, 800.0];
    
    // Create integration parameters for the physics simulation
    // Uses default values for timestep duration, damping, and other physics solver properties
    let integration_params = IntegrationParameters::default();

    // Create the physics pipeline that coordinates all physics simulation steps
    // The pipeline manages the sequential execution of broad-phase, narrow-phase, and constraint solving
    let mut pipeline = PhysicsPipeline::new();
    
    // Island manager groups bodies into "islands" for efficient computation
    // Bodies that don't interact with each other are computed separately to improve performance
    let mut island_manager = IslandManager::new();
    
    // Broad-phase collision detection: quickly identifies potential collisions
    // Uses spatial partitioning (AABB tree) to avoid checking every object against every other object
    let mut broad_phase = BroadPhase::new();
    
    // Narrow-phase collision detection: precise collision checks for objects identified by broad-phase
    // Determines exact contact points, normals, and penetration depth for physics response
    let mut narrow_phase = NarrowPhase::new();
    
    // RigidBodySet stores all dynamic and static bodies in the physics world
    // Each body has properties like position, velocity, rotation, mass, and linear/angular damping
    let mut bodies = RigidBodySet::new();
    
    // ColliderSet stores collision shapes (circles, polygons, etc.) attached to bodies
    // Defines the physical boundaries for collision detection and response
    let mut colliders = ColliderSet::new();
    
    // ImpulseJointSet manages simple joints (constraints between bodies like hinges, fixed connections)
    // Not heavily used in this game but initialized for completeness
    let mut joints = ImpulseJointSet::new();
    
    // MultibodyJointSet manages complex multi-body joint chains
    // Also initialized but not used in this simple game scenario
    let mut multibody_joints = MultibodyJointSet::new();
    
    // Continuous Collision Detection solver prevents fast-moving objects from "phasing through" obstacles
    // Important for ensuring high-velocity balls don't skip over pegs or pass through walls
    let mut ccd = CCDSolver::new();

    // ---------------------------
    // GROUND PLATFORM
    // ---------------------------
    // Create a fixed (immobile) ground body positioned at the bottom of the game world
    // Position (512.0, 580.0) places it horizontally centered and near the bottom of the 768-pixel viewport
    // A fixed body means it won't move, rotate, or respond to forces (perfect for static platforms)
    let ground_body = RigidBodyBuilder::fixed()
        .translation(vector![512.0, 580.0])
        .build();
    
    // Create a rectangular cuboid collider shape for the ground platform
    // Dimensions: 512.0 units wide (covers full viewport width) and 20.0 units tall (a thin platform)
    // friction(0.4) provides moderate friction so objects don't slide infinitely - they settle to a stop
    let ground_collider = ColliderBuilder::cuboid(512.0, 20.0)
        .friction(0.4)
        .build();
    
    // Insert the ground body into the physics world and get its handle (reference ID)
    // The handle is used to reference this body when attaching colliders
    let ground_handle = bodies.insert(ground_body);
    
    // Attach the collider to the ground body using the handle
    // This tells the physics engine that collisions with this specific shape belong to the ground
    colliders.insert_with_parent(ground_collider, ground_handle, &mut bodies);

    // ---------------------------
    // PEG GRID - Obstacle Layout
    // ---------------------------
    // Creates a staggered grid of fixed pegs that balls bounce off during gameplay
    // The pegs form the core obstacle course of the Plinko game where objects tumble down
    let peg_radius = 10.0; // All pegs have a consistent radius of 10 units for uniform collisions
    
    // Iterate through 10 rows of pegs (creating vertical layers from top to bottom)
    for row in 0..10 {
        // Calculate Y position: starts at 120.0 and increases by 40.0 units per row
        // This creates vertical spacing of 40 pixels between peg rows to allow objects to fall through
        let y = 120.0 + row as f32 * 40.0;
        
        // Iterate through 12 columns per row (horizontal distribution across the screen width)
        for col in 0..12 {
            // Offset alternating rows horizontally to create the classic Plinko staggered pattern
            // Even rows (0, 2, 4...): x_offset = 30.0, Odd rows: x_offset = 0.0
            // This brick-like offset makes the pegs interlock, creating a maze-like structure
            let x_offset = if row % 2 == 0 { 30.0 } else { 0.0 };
            
            // Calculate X position: base at 80.0, spacing of 60.0 units per peg, plus row offset
            // This spreads pegs across the full width of the game area
            let x = 80.0 + col as f32 * 60.0 + x_offset;

            // Create a fixed (immobile) body for the peg at the calculated position
            // Fixed bodies don't move or respond to forces (they're static obstacles)
            let peg_body = RigidBodyBuilder::fixed()
                .translation(vector![x, y])
                .build();
            
            // Create a circular collision shape for the peg
            // restitution(0.5) means the peg has moderate bounciness (50% energy return on collision)
            // This causes objects to bounce at predictable speeds as they hit pegs
            let peg_collider = ColliderBuilder::ball(peg_radius)
                .restitution(0.5)
                .build();
            
            // Add the peg body to the physics world and get its handle for reference
            let ph = bodies.insert(peg_body);
            
            // Attach the circular collider to the peg body using the handle
            // This completes the peg setup - now it can detect and respond to collisions
            colliders.insert_with_parent(peg_collider, ph, &mut bodies);
        }
    }

    // ---------------------------
    // SPAWN FUNCTIONS
    // ---------------------------
    // These functions create new dynamic objects with physics properties when buttons are clicked
    // Each function takes mutable references to bodies and colliders to add new entities to the world

    /// Spawns a spherical ball at the specified coordinates.
    /// Balls are small, round objects that fall through the peg grid unpredictably.
    /// They demonstrate basic physics with rolling, bouncing, and rotation.
    /// 
    /// Parameters:
    /// - bodies: Mutable reference to the rigid body set to add the new ball
    /// - colliders: Mutable reference to the collider set to add collision shape
    /// - x, y: Initial position coordinates for the ball spawn point
    fn spawn_ball(bodies: &mut RigidBodySet, colliders: &mut ColliderSet, x: f32, y: f32) {
        // Create a dynamic (moveable) rigid body for the ball
        // Dynamic bodies are affected by forces (gravity), velocity changes, and collision responses
        let body = RigidBodyBuilder::dynamic()
            .translation(vector![x, y])  // Position the ball at spawn coordinates
            .linvel(vector![0.0, 0.0])   // Start with zero linear velocity (not moving)
            .angvel(0.0)                  // Start with zero angular velocity (not spinning)
            .ccd_enabled(true)            // Enable continuous collision detection to prevent phasing through obstacles
            .linear_damping(1.0)          // Air resistance that gradually slows downward movement (prevents infinite acceleration)
            .angular_damping(1.0)         // Rotational air resistance that stops spinning over time
            .build();

        // Insert the body into the physics world and get a handle to reference it later
        let handle = bodies.insert(body);

        // Create a spherical collision shape with radius 12.0 units (slightly larger than pegs at 10.0)
        let collider = ColliderBuilder::ball(12.0)
            .restitution(0.4)   // Bounciness coefficient: 0.4 means ball retains 40% of energy after each bounce
            .friction(0.2)      // Low friction allows ball to roll smoothly without excessive grip
            .build();

        // Attach the collision shape to the ball body using its handle
        // This tells the physics engine this shape is part of the ball
        colliders.insert_with_parent(collider, handle, bodies);
    }

    /// Spawns a square-shaped object at the specified coordinates.
    /// Uses a convex polygon to define the square's collision shape.
    /// Squares are larger, more stable objects compared to balls and rotate predictably.
    ///
    /// Parameters:
    /// - bodies: Mutable reference to the rigid body set
    /// - colliders: Mutable reference to the collider set
    /// - x, y: Initial spawn position
    fn spawn_square_as_convex(bodies: &mut RigidBodySet, colliders: &mut ColliderSet, x: f32, y: f32) {
        // Define square dimensions: 24x24 units total size, 12 units from center to each edge
        let size = 24.0;
        let half = size / 2.0;

        // Define the four corner vertices of a square centered at the origin (0,0)
        // These vertices are relative to the body's center and will be rotated/translated by the physics engine
        let vertices = vec![
            Point::new(-half, -half),  // Top-left corner
            Point::new(half, -half),   // Top-right corner
            Point::new(half, half),    // Bottom-right corner
            Point::new(-half, half),   // Bottom-left corner
        ];

        // Create a dynamic body for the square
        let body = RigidBodyBuilder::dynamic()
            .translation(vector![x, y])  // Spawn at specified coordinates
            .linvel(vector![0.0, 0.0])   // Start stationary (no initial velocity)
            .angvel(0.0)                  // No initial rotation
            .ccd_enabled(true)            // Prevent tunneling through obstacles at high speeds
            .linear_damping(1.0)          // Air resistance reduces velocity over time
            .angular_damping(1.0)         // Rotational damping reduces spin
            .build();

        // Insert the body and get its handle for attaching the collider
        let handle = bodies.insert(body);

        // Create a convex hull collision shape from the square vertices
        // A convex hull automatically computes the smallest convex shape containing all vertices
        // unwrap() assumes vertex list is valid (it is, since it's a simple square)
        let collider = ColliderBuilder::convex_hull(&vertices)
            .unwrap()
            .restitution(0.4)   // Moderate bounciness matches the ball (0.4 energy retention)
            .friction(0.3)      // Higher friction than balls (0.3 vs 0.2) reduces sliding behavior
            .build();

        // Attach the collision shape to the square body
        colliders.insert_with_parent(collider, handle, bodies);
    }

    /// Spawns an equilateral triangle-shaped object at the specified coordinates.
    /// Triangles are angular objects that can produce unpredictable and varied bounces.
    /// Their three vertices create interesting collision dynamics compared to rounded objects.
    ///
    /// Parameters:
    /// - bodies: Mutable reference to the rigid body set
    /// - colliders: Mutable reference to the collider set
    /// - x, y: Initial spawn position
    fn spawn_triangle(bodies: &mut RigidBodySet, colliders: &mut ColliderSet, x: f32, y: f32) {
        // Define triangle dimensions: 24-unit sides
        let side = 24.0;
        // Height of equilateral triangle = (√3/2) * side_length
        // This ensures all three sides are equal length (60-degree angles)
        let height = (3.0_f32).sqrt() / 2.0 * side;

        // Define three vertices of an equilateral triangle
        // Vertices are positioned so the center of mass (centroid) is near the origin
        // This ensures the triangle balances properly during physics simulation
        let vertices = vec![
            Point::new(0.0, -height / 3.0),          // Top vertex (pointing upward)
            Point::new(-side / 2.0, height * 2.0 / 3.0),  // Bottom-left vertex
            Point::new(side / 2.0, height * 2.0 / 3.0),   // Bottom-right vertex
        ];

        // Create dynamic body for the triangle
        let body = RigidBodyBuilder::dynamic()
            .translation(vector![x, y])  // Spawn at specified position
            .linvel(vector![0.0, 0.0])   // Start stationary
            .angvel(0.0)                  // No initial rotation
            .ccd_enabled(true)            // Continuous collision detection prevents tunneling
            .linear_damping(1.0)          // Linear air resistance slows velocity
            .angular_damping(1.0)         // Rotational air resistance reduces spin
            .build();

        // Insert body and get handle for collider attachment
        let handle = bodies.insert(body);

        // Create convex hull collision shape from triangle vertices
        // For a triangle, the convex hull is exactly the triangle itself
        let collider = ColliderBuilder::convex_hull(&vertices)
            .unwrap()
            .restitution(0.4)   // Bounciness (same 0.4 as balls)
            .friction(0.2)      // Low friction like balls (0.2), allowing more sliding than squares
            .build();

        // Attach collision shape to the triangle body
        colliders.insert_with_parent(collider, handle, bodies);
    }

    // ---------------------------
    // UI BUTTONS
    // ---------------------------
    // Create three interactive buttons on the right side of the screen
    // Each button spawns a different type of object when clicked by the player
    // Parameters: x_pos, y_pos, width, height, label, background_color, hover_color, font_size
    let btn_ball = TextButton::new(800.0, 200.0, 200.0, 60.0, "Spawn Ball", BLUE, GREEN, 30);
    let btn_square = TextButton::new(800.0, 400.0, 200.0, 60.0, "Spawn Square", BLUE, GREEN, 30);
    let btn_triangle = TextButton::new(800.0, 600.0, 200.0, 60.0, "Spawn Triangle", BLUE, GREEN, 30);

    // Variable to store random spawn position for newly created objects
    // Gets reassigned each time a button is clicked with a random X coordinate
    let mut place;
    
    // Seed the random number generator with current date/time for non-deterministic behavior
    // This ensures different random sequences each time the game runs
    // Without this, the sequence would repeat identically across runs
    rand::srand(date::now() as u64);

    // ---------------------------
    // MAIN GAME LOOP
    // ---------------------------
    // This loop runs once per frame (typically 60 times per second on most displays)
    // It handles player input, updates physics simulation, and renders graphics
    loop {
        // Set virtual resolution to maintain consistent gameplay at 1024x768
        // This handles automatic scaling for different monitor sizes and aspect ratios
        // Ensures the game looks the same regardless of the player's screen resolution
        use_virtual_resolution(1024.0, 768.0);
        
        // Clear the entire screen to black, preparing for fresh rendering
        // This wipes the previous frame's graphics before drawing the new frame
        clear_background(BLACK);

        // ----- BUTTON INTERACTION LOGIC -----
        // Check if the spawn ball button was clicked by the player
        if btn_ball.click() {
            // Roll a random number 1-6 (like rolling a dice) to determine spawn position
            // This creates variety in where objects enter the game
            let dice = rand::gen_range(1, 7);
            // Map dice result to X coordinate: simulates random column selection
            // Results spread across six different horizontal positions: 201, 300, 400, 501, 600, 700
            place = match dice { 1 => 201, 2 => 300, 3 => 400, 4 => 501, 5 => 600, 6 => 700, _ => 400 };
            // Spawn ball at selected X position and Y=50 (near top of screen)
            spawn_ball(&mut bodies, &mut colliders, place as f32, 50.0);
        }

        // Check if the spawn square button was clicked
        if btn_square.click() {
            // Same random position selection as ball spawn for consistency
            let dice = rand::gen_range(1, 7);
            place = match dice { 1 => 201, 2 => 300, 3 => 400, 4 => 501, 5 => 600, 6 => 700, _ => 400 };
            // Spawn square at the randomly selected position
            spawn_square_as_convex(&mut bodies, &mut colliders, place as f32, 50.0);
        }

        // Check if the spawn triangle button was clicked
        if btn_triangle.click() {
            // Same random position selection for consistent gameplay patterns
            let dice = rand::gen_range(1, 7);
            place = match dice { 1 => 201, 2 => 300, 3 => 400, 4 => 501, 5 => 600, 6 => 700, _ => 400 };
            // Spawn triangle at the randomly selected position
            spawn_triangle(&mut bodies, &mut colliders, place as f32, 50.0);
        }

        // ----- PHYSICS SIMULATION STEP -----
        // Execute one frame of physics simulation
        // This single call performs all physics calculations: broad-phase detection, narrow-phase collision,
        // constraint solving, and integration of motion for all bodies
        pipeline.step(
            &gravity,                      // Apply gravity force to all dynamic bodies (accelerates them downward)
            &integration_params,           // Use configured physics parameters for this simulation step
            &mut island_manager,           // Update body islands for optimization (groups related bodies)
            &mut broad_phase,              // Quick collision detection pass (AABB overlap tests)
            &mut narrow_phase,             // Precise collision detection and response calculation
            &mut bodies,                   // Update all body positions, velocities, and rotations
            &mut colliders,                // Update collision shape positions (attached to bodies)
            &mut joints,                   // Process any joint constraints between bodies
            &mut multibody_joints,         // Process multi-body joint constraints
            &mut ccd,                      // Continuous collision detection for fast-moving objects
            None,                          // No custom character controller plugin
            &(),                           // No additional physics hooks
            &(),                           // No event callback for post-step processing
        );

        // ----- RENDER ALL PHYSICS BODIES -----
        // Iterate through all bodies in the physics world and draw them on the screen
        for (_handle, body) in bodies.iter() {
            // Get the body's current world position (center point coordinates)
            // This is where the object is located after physics calculations
            let pos = body.translation();
            
            // Get the body's current rotation angle in radians
            // Used to properly orient polygon shapes (balls rotate too but it's not visible)
            let rot = body.rotation().angle();

            // Iterate through all collision shapes attached to this body
            // A body can have multiple colliders (though our game uses one per body)
            for col_handle in body.colliders() {
                // Get reference to the collision shape object from the collider set
                let collider = &colliders[*col_handle];
                // Extract the geometric shape from the collider (can be ball, convex polygon, etc.)
                let shape = collider.shape();

                // ----- RENDER CIRCLES -----
                // This conditional handles rendering of balls (dynamic) and pegs (static/fixed)
                if let Some(ball) = shape.as_ball() {
                    // Color depends on whether the body is fixed or dynamic
                    // Fixed bodies (pegs and ground) render RED, dynamic bodies (spawned objects) render YELLOW
                    // This visual distinction helps identify static vs moving objects in the game
                    let color = if body.is_fixed() { RED } else { YELLOW };
                    // Draw the circle at the body's position with its collision radius
                    // The circle is rendered filled with the selected color
                    draw_circle(pos.x, pos.y, ball.radius, color);
                }

                // ----- RENDER POLYGONS -----
                // This conditional handles rendering of convex polygons (triangles and squares)
                if let Some(convex) = shape.as_convex_polygon() {
                    // Transform each vertex from local shape coordinates to world coordinates
                    // This accounts for both the body's position and rotation in the world
                    let points: Vec<Vec2> = convex.points().iter().map(|v| {
                        // Apply 2D rotation matrix to rotate the point by the body's angle
                        // Rotation formula: x' = x*cos(θ) - y*sin(θ), y' = x*sin(θ) + y*cos(θ)
                        let x_rot = v.x * rot.cos() - v.y * rot.sin();
                        let y_rot = v.x * rot.sin() + v.y * rot.cos();
                        // Translate rotated point to world position (add body's position)
                        vec2(pos.x + x_rot, pos.y + y_rot)
                    }).collect();

                    // Draw lines connecting consecutive vertices to outline the polygon
                    for i in 0..points.len() {
                        let next = (i + 1) % points.len();  // Wrap around to connect last vertex to first
                        // Draw line segment in BLUE with 2-pixel line thickness
                        // This creates the visible outline of the square or triangle
                        draw_line(points[i].x, points[i].y, points[next].x, points[next].y, 2.0, BLUE);
                    }
                }
            }
        }

        // Advance to the next frame and yield control back to the graphics system
        // The await keyword allows the async runtime to handle frame timing and input processing
        // The graphics system will display the rendered frame on the screen
        next_frame().await;
    }
}