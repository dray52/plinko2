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
use crate::modules::label::Label;
// Helper: create a circle peg map constrained to inside wall edges
fn create_circle_peg_map(bodies: &mut RigidBodySet, colliders: &mut ColliderSet) {
    let peg_radius = 8.0; // smaller pegs to keep denser layout inside walls

    // Keep vertical extent (10 rows) and increase horizontal density to 14 columns
    let rows = 11;
    let cols = 18;
    let wall_inner_left = 70.0 + 10.0;
    let wall_inner_right = 780.0 - 10.0;
    let safety_inset = 10.0;
    let usable_left = wall_inner_left + peg_radius + safety_inset;
    let usable_right = wall_inner_right - peg_radius - safety_inset;
    let start_x = usable_left;
    let spacing = if cols > 1 { (usable_right - usable_left) / (cols as f32 - 1.0) } else { 0.0 };
    let peg_shift = -3.0;

    for row in 0..rows {
        let y = 120.0 + row as f32 * 40.0;
        for col in 0..cols {
            let x_offset = if row % 2 == 0 { spacing / 2.0 } else { 0.0 };
            let x = start_x + col as f32 * spacing + x_offset + peg_shift;

            let peg_body = RigidBodyBuilder::fixed()
                .translation(vector![x, y])
                .build();

            let peg_collider = ColliderBuilder::ball(peg_radius)
                .restitution(0.5)
                .build();

            let ph = bodies.insert(peg_body);
            colliders.insert_with_parent(peg_collider, ph, bodies);
        }
    }

   
}

// Helper: create a triangle peg map constrained to inside wall edges
fn create_triangle_peg_map(bodies: &mut RigidBodySet, colliders: &mut ColliderSet) {
    let peg_size = 12.0; // slightly smaller triangle pegs
    let height = (3.0_f32).sqrt() / 2.0 * peg_size;

    // Keep vertical extent (10 rows) and increase horizontal density to 14 columns
     let rows = 11;
    let cols = 18;
    let wall_inner_left = 70.0 + 10.0;
    let wall_inner_right = 780.0 - 10.0;
    let safety_inset = 10.0;
    // For triangle pegs approximate half-extent as peg_size/2.0
    let peg_extent = peg_size / 2.0;
    let usable_left = wall_inner_left + peg_extent + safety_inset;
    let usable_right = wall_inner_right - peg_extent - safety_inset;
    let start_x = usable_left;
    let spacing = if cols > 1 { (usable_right - usable_left) / (cols as f32 - 1.0) } else { 0.0 };
    let peg_shift = -3.0;

    for row in 0..rows {
        let y = 120.0 + row as f32 * 40.0;
        for col in 0..cols {
            let x_offset = if row % 2 == 0 { spacing / 2.0 } else { 0.0 };
            let x = start_x + col as f32 * spacing + x_offset + peg_shift;

            let peg_body = RigidBodyBuilder::fixed()
                .translation(vector![x, y])
                .build();

            let vertices = vec![
                Point::new(0.0, -height / 3.0),
                Point::new(-peg_size / 2.0, height * 2.0 / 3.0),
                Point::new(peg_size / 2.0, height * 2.0 / 3.0),
            ];

            let peg_collider = ColliderBuilder::convex_hull(&vertices)
                .unwrap()
                .restitution(0.5)
                .build();

            let ph = bodies.insert(peg_body);
            colliders.insert_with_parent(peg_collider, ph, bodies);
        }
    }

    
}
use rapier2d::prelude::*;
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
    // Ground constants for easy layout adjustments
    const GROUND_X: f32 = 432.0;
    const GROUND_Y: f32 = 700.0;
    const GROUND_HALF_WIDTH: f32 = 355.0;
    const GROUND_HALF_HEIGHT: f32 = 20.0;
    const GROUND_TOP: f32 = GROUND_Y - GROUND_HALF_HEIGHT;
    // Create a fixed (immobile) ground body positioned at the bottom of the game world
    // Position (512.0, 700.0) places it horizontally centered and at the very bottom of the 768-pixel viewport
    // A fixed body means it won't move, rotate, or respond to forces (perfect for static platforms)
    let ground_body = RigidBodyBuilder::fixed()
        .translation(vector![GROUND_X, GROUND_Y])
        .build();

    // Create a rectangular cuboid collider shape for the ground platform using constants
    let ground_collider = ColliderBuilder::cuboid(GROUND_HALF_WIDTH, GROUND_HALF_HEIGHT)
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
        // Constrain initial peg grid to wall inner edges and reduce peg radius to 8; keep 10 rows and increase columns to 14.
        let peg_radius = 8.0; // slightly smaller pegs to allow higher density
        let rows = 10;
        let cols = 15; // more pegs per row
        let wall_inner_left = 70.0 + 10.0; // left wall x + half-width
        let wall_inner_right = 780.0 - 10.0; // right wall x - half-width
        // Compute usable region by insetting the wall by peg radius + safety margin so pegs don't overlap walls
        let safety_inset = 12.0;
        let usable_left = wall_inner_left + peg_radius + safety_inset;
        let usable_right = wall_inner_right - peg_radius - safety_inset;
        let start_x = usable_left;
        let spacing = if cols > 1 { (usable_right - usable_left) / (cols as f32 - 1.0) } else { 0.0 };
        let peg_shift = -5.0; // move pegs left by 5 units

    for row in 0..rows {
        let y = 120.0 + row as f32 * 40.0;
        for col in 0..cols {
            let x_offset = if row % 2 == 0 { spacing / 2.0 } else { 0.0 };
            let x = start_x + col as f32 * spacing + x_offset + peg_shift;

            let peg_body = RigidBodyBuilder::fixed()
                .translation(vector![x, y])
                .build();

            let peg_collider = ColliderBuilder::ball(peg_radius)
                .restitution(0.5)
                .build();

            let ph = bodies.insert(peg_body);
            colliders.insert_with_parent(peg_collider, ph, &mut bodies);
        }
    }

    // Extra left-side column for the initial peg grid in main
    let x_extra_base = start_x - spacing;
    for row in 0..rows {
        let y = 120.0 + row as f32 * 40.0;
        let x_offset = if row % 2 == 0 { spacing / 2.0 } else { 0.0 };
        let x = x_extra_base + x_offset + peg_shift;

        let peg_body = RigidBodyBuilder::fixed()
            .translation(vector![x, y])
            .build();

        let peg_collider = ColliderBuilder::ball(peg_radius)
            .restitution(0.5)
            .build();

        let ph = bodies.insert(peg_body);
        colliders.insert_with_parent(peg_collider, ph, &mut bodies);
    }

    // ---------------------------
    // SPAWN FUNCTIONS
    // ---------------------------
    // These functions create new dynamic objects with physics properties when buttons are clicked
    // Each function takes mutable references to bodies and colliders to add new entities to the world

    

    /// Create the bottom bins (vertical dividers) and attach colliders.
    /// There are 6 sections across the full width. Call this after walls/pegs are created
    fn create_bins(bodies: &mut RigidBodySet, colliders: &mut ColliderSet) {
        // Compute bin positions relative to the ground edges so bins fit within walls/ground
        let bins = 6;
        let ground_left = GROUND_X - GROUND_HALF_WIDTH;
        let ground_right = GROUND_X + GROUND_HALF_WIDTH;
        let bin_width = (ground_right - ground_left) / bins as f32;

        // Divider vertical size: make them a bit shorter and thicker
        let half_height = 60.0; // half-height -> full height = 120
        let half_width = 4.0; // thicker divider (8px wide)

        // Place dividers between the bins, inside ground bounds
        for i in 1..bins {
            let x = ground_left + bin_width * i as f32;
            // Center Y so dividers sit directly above ground (bottom aligns with ground top)
            let y = GROUND_TOP - half_height;

            let div_body = RigidBodyBuilder::fixed()
                .translation(vector![x, y])
                .build();

            let div_collider = ColliderBuilder::cuboid(half_width, half_height)
                .friction(0.4)
                .build();

            let h = bodies.insert(div_body);
            colliders.insert_with_parent(div_collider, h, bodies);
        }
    }

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

        // Create a spherical collision shape with radius 8.0 units (smaller than pegs at 10.0)
        let collider = ColliderBuilder::ball(7.0)
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
        let size = 16.0;
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
        let side = 15.0;
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
fn create_square_peg_map(bodies: &mut RigidBodySet, colliders: &mut ColliderSet) {
    let peg_size = 12.0;       // side length
    let half = peg_size / 2.0;
    let angle = std::f32::consts::FRAC_PI_4;  // 45 degrees
    let cos_a = angle.cos();
    let sin_a = angle.sin();

    // Square vertices BEFORE rotation
    let base_vertices = vec![
        Point::new(-half, -half),
        Point::new( half, -half),
        Point::new( half,  half),
        Point::new(-half,  half),
    ];

    // Rotate each vertex by 45° to create a diamond shape
    let rotated_vertices: Vec<Point<f32>> = base_vertices
        .iter()
        .map(|v| {
            Point::new(
                v.x * cos_a - v.y * sin_a,
                v.x * sin_a + v.y * cos_a,
            )
        })
        .collect();


        
    let rows = 11;
    let cols = 18;
    let wall_inner_left = 70.0 + 10.0;
    let wall_inner_right = 780.0 - 10.0;
    let safety_inset = 10.0;
    let usable_left = wall_inner_left + half + safety_inset;
    let usable_right = wall_inner_right - half - safety_inset;

    let start_x = usable_left;
    let spacing = if cols > 1 {
        (usable_right - usable_left) / (cols as f32 - 1.0)
    } else { 0.0 };

    let peg_shift = -3.0;

    for row in 0..rows {
        let y = 120.0 + row as f32 * 40.0;

        for col in 0..cols {
            let x_offset = if row % 2 == 0 { spacing / 2.0 } else { 0.0 };
            let x = start_x + col as f32 * spacing + x_offset + peg_shift;

            let peg_body = RigidBodyBuilder::fixed()
                .translation(vector![x, y])
                .build();

            let peg_collider = ColliderBuilder::convex_hull(&rotated_vertices)
                .unwrap()
                .restitution(0.5)
                .build();

            let ph = bodies.insert(peg_body);
            colliders.insert_with_parent(peg_collider, ph, bodies);
        }
    }

   
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
    let btn_circle_map = TextButton::new(50.0, 20.0, 150.0, 60.0, "Circle Pegs", BLUE, YELLOW, 25);
    let btn_triangle_map = TextButton::new(250.0, 20.0, 150.0, 60.0, "Triangle Pegs", ORANGE, YELLOW, 25);
    let btn_square_map = TextButton::new(650.0, 20.0, 150.0, 60.0, "Square Pegs", BLUE, YELLOW, 25);
    let btn_clear_shapes = TextButton::new(450.0, 20.0, 150.0, 60.0, "Clear Shapes", RED, YELLOW, 25);

    // Variable to store random spawn position for newly created objects
    // Gets reassigned each time a button is clicked with a random X coordinate
    let mut place;
   
    // Seed the random number generator with current date/time for non-deterministic behavior
    // This ensures different random sequences each time the game runs
    // Without this, the sequence would repeat identically across runs
    rand::srand(date::now() as u64);

    // ---------------------------
    // WALL - Left & Right Boundaries
    // ---------------------------
    // Create walls LAST so they render on top of all pegs and objects
    // Create a fixed (immobile) wall body positioned on the left side of the game world
    let wall_body_left = RigidBodyBuilder::fixed()
        .translation(vector![70.0, 400.0])
        .build();
    
    // Create a fixed (immobile) wall body positioned on the right side of the game world
    let wall_body_right = RigidBodyBuilder::fixed()
        .translation(vector![780.0, 400.0])
        .build();
    
    // Create a rectangular cuboid collider shape for the walls
    // Dimensions: 10.0 units wide and 400.0 units tall (tall vertical walls)
    let wall_collider = ColliderBuilder::cuboid(10.0, 400.0)
        .friction(0.4)
        .build();
    
    // Insert the wall bodies into the physics world and get their handles
    let wall_handle_left = bodies.insert(wall_body_left);
    let wall_handle_right = bodies.insert(wall_body_right);
    
    // Attach the collider to both wall bodies
    colliders.insert_with_parent(wall_collider.clone(), wall_handle_left, &mut bodies);
    colliders.insert_with_parent(wall_collider, wall_handle_right, &mut bodies);

    // Create bottom bin dividers
    create_bins(&mut bodies, &mut colliders);

    // Create six individual Label objects for each prize bin
    // Choose a random prize value in the range 0..=3 for each bin and center the label
        let bin_count = 6usize;
        let _bin_width = 1024.0 / bin_count as f32;

        let mut lbl_pize1 = Label::new("Hello\nWorld", 50.0, 100.0, 30);
        lbl_pize1.with_colors(WHITE, Some(BLACK));
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
        // Check if the circle pegs map button was clicked
        if btn_circle_map.click() {
            // Reset physics managers
            pipeline = PhysicsPipeline::new();
            island_manager = IslandManager::new();
            broad_phase = BroadPhase::new();
            narrow_phase = NarrowPhase::new();
            ccd = CCDSolver::new();
            
            // Clear all pegs and dynamic objects but keep ground and walls
            bodies = RigidBodySet::new();
            colliders = ColliderSet::new();
            
            // Recreate ground
            let ground_body = RigidBodyBuilder::fixed()
                .translation(vector![432.0, 700.0])
                .build();
            let ground_collider = ColliderBuilder::cuboid(355.0, 20.0)
                .friction(0.4)
                .build();
            let ground_handle = bodies.insert(ground_body);
            colliders.insert_with_parent(ground_collider, ground_handle, &mut bodies);
            
            // Generate original circular peg map, then recreate walls and bins so they render on top
            create_circle_peg_map(&mut bodies, &mut colliders);

            // Recreate walls so they are above pegs
            let wall_body_left = RigidBodyBuilder::fixed()
                .translation(vector![70.0, 400.0])
                .build();
            let wall_body_right = RigidBodyBuilder::fixed()
                .translation(vector![780.0, 400.0])
                .build();
            let wall_collider = ColliderBuilder::cuboid(10.0, 400.0)
                .friction(0.4)
                .build();
            let wall_handle_left = bodies.insert(wall_body_left);
            let wall_handle_right = bodies.insert(wall_body_right);
            colliders.insert_with_parent(wall_collider.clone(), wall_handle_left, &mut bodies);
            colliders.insert_with_parent(wall_collider, wall_handle_right, &mut bodies);

            // Create bins once
            create_bins(&mut bodies, &mut colliders);
        }
if btn_square_map.click() {
    // Reset physics managers
    pipeline = PhysicsPipeline::new();
    island_manager = IslandManager::new();
    broad_phase = BroadPhase::new();
    narrow_phase = NarrowPhase::new();
    ccd = CCDSolver::new();

    bodies = RigidBodySet::new();
    colliders = ColliderSet::new();

    // Recreate ground
    let ground_body = RigidBodyBuilder::fixed()
        .translation(vector![432.0, 700.0])
        .build();
    let ground_collider = ColliderBuilder::cuboid(355.0, 20.0)
        .friction(0.4)
        .build();
    let ground_handle = bodies.insert(ground_body);
    colliders.insert_with_parent(ground_collider, ground_handle, &mut bodies);

    // Generate square peg map
    create_square_peg_map(&mut bodies, &mut colliders);

    // Recreate walls above pegs
    let wall_body_left = RigidBodyBuilder::fixed()
        .translation(vector![70.0, 400.0])
        .build();
    let wall_body_right = RigidBodyBuilder::fixed()
        .translation(vector![780.0, 400.0])
        .build();
    let wall_collider = ColliderBuilder::cuboid(10.0, 400.0)
        .friction(0.4)
        .build();
    let wall_handle_left = bodies.insert(wall_body_left);
    let wall_handle_right = bodies.insert(wall_body_right);
    colliders.insert_with_parent(wall_collider.clone(), wall_handle_left, &mut bodies);
    colliders.insert_with_parent(wall_collider, wall_handle_right, &mut bodies);

    // Bins
    create_bins(&mut bodies, &mut colliders);
}
        // ----- BUTTON INTERACTION LOGIC -----
        // Check if the triangle pegs map button was clicked
        if btn_triangle_map.click() {
            // Reset physics managers
            pipeline = PhysicsPipeline::new();
            island_manager = IslandManager::new();
            broad_phase = BroadPhase::new();
            narrow_phase = NarrowPhase::new();
            ccd = CCDSolver::new();
            
            // Clear all pegs and dynamic objects but keep ground and walls
            bodies = RigidBodySet::new();
            colliders = ColliderSet::new();
            
            // Recreate ground
            let ground_body = RigidBodyBuilder::fixed()
                .translation(vector![432.0, 700.0])
                .build();
            let ground_collider = ColliderBuilder::cuboid(355.0, 20.0)
                .friction(0.4)
                .build();
            let ground_handle = bodies.insert(ground_body);
            colliders.insert_with_parent(ground_collider, ground_handle, &mut bodies);
            
            // Generate triangle peg map, then recreate walls and bins so they render on top
            create_triangle_peg_map(&mut bodies, &mut colliders);

            // Recreate walls so they appear above pegs
            let wall_body_left = RigidBodyBuilder::fixed()
                .translation(vector![70.0, 400.0])
                .build();
            let wall_body_right = RigidBodyBuilder::fixed()
                .translation(vector![780.0, 400.0])
                .build();
            let wall_collider = ColliderBuilder::cuboid(10.0, 400.0)
                .friction(0.4)
                .build();
            let wall_handle_left = bodies.insert(wall_body_left);
            let wall_handle_right = bodies.insert(wall_body_right);
            colliders.insert_with_parent(wall_collider.clone(), wall_handle_left, &mut bodies);
            colliders.insert_with_parent(wall_collider, wall_handle_right, &mut bodies);

            // Create bins once
            create_bins(&mut bodies, &mut colliders);
        }

        // Check if the clear shapes button was clicked
        if btn_clear_shapes.click() {
            // Reset to initial state
            pipeline = PhysicsPipeline::new();
            island_manager = IslandManager::new();
            broad_phase = BroadPhase::new();
            narrow_phase = NarrowPhase::new();
            ccd = CCDSolver::new();
            
            bodies = RigidBodySet::new();
            colliders = ColliderSet::new();
            joints = ImpulseJointSet::new();
            multibody_joints = MultibodyJointSet::new();
            
            // Recreate ground
            let ground_body = RigidBodyBuilder::fixed()
                .translation(vector![432.0, 700.0])
                .build();
            let ground_collider = ColliderBuilder::cuboid(355.0, 20.0)
                .friction(0.4)
                .build();
            let ground_handle = bodies.insert(ground_body);
            colliders.insert_with_parent(ground_collider, ground_handle, &mut bodies);
            
            // Recreate circle pegs (same dense layout as initial map), constrained to walls
            let peg_radius = 8.0;
            let rows = 10;
            let cols = 15;
            let wall_inner_left = 70.0 + 10.0;
            let wall_inner_right = 780.0 - 10.0;
            let safety_inset = 4.0;
            let usable_left = wall_inner_left + peg_radius + safety_inset;
            let usable_right = wall_inner_right - peg_radius - safety_inset;
            let start_x = usable_left;
            let spacing = if cols > 1 { (usable_right - usable_left) / (cols as f32 - 1.0) } else { 0.0 };

            for row in 0..rows {
                let y = 120.0 + row as f32 * 40.0;
                for col in 0..cols {
                    let x_offset = if row % 2 == 0 { spacing / 2.0 } else { 0.0 };
                    let x = start_x + col as f32 * spacing + x_offset;
                    let peg_body = RigidBodyBuilder::fixed()
                        .translation(vector![x, y])
                        .build();
                    let peg_collider = ColliderBuilder::ball(peg_radius)
                        .restitution(0.5)
                        .build();
                    let ph = bodies.insert(peg_body);
                    colliders.insert_with_parent(peg_collider, ph, &mut bodies);
                }
            }
                // Extra left-side column for recreated pegs
                let x_extra_base = start_x - spacing;
                for row in 0..rows {
                    let y = 120.0 + row as f32 * 40.0;
                    let x_offset = if row % 2 == 0 { spacing / 2.0 } else { 0.0 };
                    let x = x_extra_base + x_offset;

                    let peg_body = RigidBodyBuilder::fixed()
                        .translation(vector![x, y])
                        .build();

                    let peg_collider = ColliderBuilder::ball(peg_radius)
                        .restitution(0.5)
                        .build();

                    let ph = bodies.insert(peg_body);
                    colliders.insert_with_parent(peg_collider, ph, &mut bodies);
                }
            
            // Recreate walls so they are above pegs
            let wall_body_left = RigidBodyBuilder::fixed()
                .translation(vector![70.0, 400.0])
                .build();
            let wall_body_right = RigidBodyBuilder::fixed()
                .translation(vector![780.0, 400.0])
                .build();
            let wall_collider = ColliderBuilder::cuboid(10.0, 400.0)
                .friction(0.4)
                .build();
            let wall_handle_left = bodies.insert(wall_body_left);
            let wall_handle_right = bodies.insert(wall_body_right);
            colliders.insert_with_parent(wall_collider.clone(), wall_handle_left, &mut bodies);
            colliders.insert_with_parent(wall_collider, wall_handle_right, &mut bodies);

            // Create bins once
            create_bins(&mut bodies, &mut colliders);
        }

        // Check if the spawn ball button was clicked by the player
        if btn_ball.click() {
            // Roll a random number 1-6 (like rolling a dice) to determine spawn position
            // This creates variety in where objects enter the game
            let dice = rand::gen_range(1, 7);
            // Map dice result to X coordinate: simulates random column selection
            // Results spread across six different horizontal positions: 201, 300, 400, 501, 600, 700
            place = match dice { 1 => 201, 2 => 300, 3 => 400, 4 => 501, 5 => 600, 6 => 690, _ => 400 };
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
            place = match dice { 1 => 201, 2 => 300, 3 => 400, 4 => 501, 5 => 600, 6 => 690, _ => 400 };
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
                    let color = if ball.radius > 100.0 {
                        ORANGE // Ground platform
                    } else if body.is_fixed() {
                        GREEN // Pegs are now green
                    } else {
                        YELLOW // Dynamic objects
                    };
                    draw_circle(pos.x, pos.y, ball.radius, color);
                }
                // ----- RENDER CUBOIDS -----
                // This handles rendering the ground platform and walls (cuboid/rectangle shapes)
                if let Some(cuboid) = shape.as_cuboid() {
                    // Get the half-extents (distance from center to edge)
                    let hx = cuboid.half_extents.x;
                    let hy = cuboid.half_extents.y;
                    
                    // Draw filled rectangle for the ground/walls in GREEN
                    draw_rectangle(pos.x - hx, pos.y - hy, hx * 2.0, hy * 2.0, GREEN);
                }

                // ----- RENDER POLYGONS -----
                // This conditional handles rendering of convex polygons (triangles and squares)
                if let Some(convex) = shape.as_convex_polygon() {
                    // Precompute cos and sin for this body's rotation to avoid repeated trig calls
                    let cos_r = rot.cos();
                    let sin_r = rot.sin();

                    // Transform vertices and draw lines without repeated trig evaluation
                    let pts = convex.points();
                    if !pts.is_empty() {
                        // Transform first point
                        let first = pts[0];
                        let mut prev_x = pos.x + (first.x * cos_r - first.y * sin_r);
                        let mut prev_y = pos.y + (first.x * sin_r + first.y * cos_r);

                        for v in pts.iter().skip(1) {
                            let x = pos.x + (v.x * cos_r - v.y * sin_r);
                            let y = pos.y + (v.x * sin_r + v.y * cos_r);
                            draw_line(prev_x, prev_y, x, y, 2.0, RED);
                            prev_x = x;
                            prev_y = y;
                        }

                        // Close the polygon (connect last to first)
                        let x0 = pos.x + (first.x * cos_r - first.y * sin_r);
                        let y0 = pos.y + (first.x * sin_r + first.y * cos_r);
                        draw_line(prev_x, prev_y, x0, y0, 2.0, RED);
                    }
                }
            }
        }

        lbl_pize1.draw();
        // Advance to the next frame and yield control back to the graphics system
        // The await keyword allows the async runtime to handle frame timing and input processing
        // The graphics system will display the rendered frame on the screen
        next_frame().await;
    }
}