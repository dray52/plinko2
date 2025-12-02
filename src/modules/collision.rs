/*
Made by: Mathew Dusome
Lets us check for collisions with pixels.  One version for web one for native 
linux and windows

Must add the following to Cargo.toml

# Conditionally include Rayon only for native platforms (not Wasm)
rayon = { version = "1.7", optional = true }
[features]
default = ["native"]  # Default feature includes "native"
native = ["rayon"]    # The "native" feature enables Rayon
[target.'cfg(not(target_arch = "wasm32"))'.dependencies]
rayon = "1.7"  # Rayon is only included for native builds

In your mod.rs file located in the modules folder add the following to the end of the file:
    pub mod collision;
Then in with the other use command add:

use crate::modules::collision::check_collision;
 
Then in the loop you would use the follow to check if two images hit: 
let collision = check_collision(&img1, &img2, 1); //Where 1 is the number of pixels to skip
    if collision {
        println!("Collision detected!");
    } else {
        println!("No collision.");
    }
*/

use macroquad::prelude::Vec2;

#[cfg(not(target_arch = "wasm32"))]
use rayon::prelude::*;

// Define the Collidable trait
pub trait Collidable {
    fn pos(&self) -> Vec2;
    fn size(&self) -> Vec2;
    fn texture_size(&self) -> Vec2;
    fn get_mask(&self) -> Option<Vec<u8>>;
    fn get_angle(&self) -> f32; // New method to get rotation angle
}
use crate::modules::still_image::StillImage;
// Implement for StillImage
impl Collidable for StillImage {
    fn pos(&self) -> Vec2 {
        self.pos()
    }
    
    fn size(&self) -> Vec2 {
        self.size()
    }
    
    fn texture_size(&self) -> Vec2 {
        self.texture_size()
    }
    
    fn get_mask(&self) -> Option<Vec<u8>> {
        self.get_mask()
    }
    
    fn get_angle(&self) -> f32 {
        self.get_angle()
    }
}
/* 
use crate::modules::animated_image::AnimatedImage;
// Implement for AnimatedImage
impl Collidable for AnimatedImage {
    fn pos(&self) -> Vec2 {
        self.pos()
    }
    
    fn size(&self) -> Vec2 {
        self.size()
    }
    
    fn texture_size(&self) -> Vec2 {
        self.texture_size()
    }
    
    fn get_mask(&self) -> Option<Vec<u8>> {
        self.get_mask()
    }
    
    fn get_angle(&self) -> f32 {
        self.get_angle()
    }
}
*/

// Utility function to calculate texture coordinates safely
#[inline]
fn calc_tex_coord(point: Vec2, pos: Vec2, size: Vec2, tex_size: Vec2) -> (usize, usize) {
    let safe_size_x = if size.x < 0.001 { 0.001 } else { size.x };
    let safe_size_y = if size.y < 0.001 { 0.001 } else { size.y };
    
    let norm_x = (point.x - pos.x) / safe_size_x;
    let norm_y = (point.y - pos.y) / safe_size_y;
    
    let clamped_x = norm_x.max(0.0).min(0.999);
    let clamped_y = norm_y.max(0.0).min(0.999);
    
    let tx = (clamped_x * tex_size.x) as usize;
    let ty = (clamped_y * tex_size.y) as usize;
    
    (tx, ty)
}

// Utility function to check if a mask bit is set (1 = opaque)
#[inline]
fn is_mask_bit_set(mask: &[u8], idx: usize) -> Option<bool> {
    if idx / 8 >= mask.len() {
        None
    } else {
        let mask_byte = mask[idx / 8];
        let mask_bit = (mask_byte >> (7 - (idx % 8))) & 1;
        Some(mask_bit == 1)
    }
}

// Utility function to check if a point is within rectangle bounds
#[inline]
fn is_point_in_bounds(point: Vec2, pos: Vec2, size: Vec2) -> bool {
    point.x >= pos.x && point.x < pos.x + size.x &&
    point.y >= pos.y && point.y < pos.y + size.y
}

// Generic collision detection function that works with anything implementing Collidable
pub fn check_collision<T, U>(obj1: &T, obj2: &U, skip_pixels: usize) -> bool
where
    T: Collidable,
    U: Collidable,
{
    let pos1 = obj1.pos();
    let size1 = obj1.size();
    let mask1_opt = obj1.get_mask();
    let texture1_size = obj1.texture_size();
    let angle1 = obj1.get_angle();

    let pos2 = obj2.pos();
    let size2 = obj2.size();
    let mask2_opt = obj2.get_mask();
    let texture2_size = obj2.texture_size();
    let angle2 = obj2.get_angle();
    
    // If objects are rotated, calculate rotated bounding boxes
    let (rot_pos1, rot_size1) = calculate_rotated_bounding_box(pos1, size1, angle1);
    let (rot_pos2, rot_size2) = calculate_rotated_bounding_box(pos2, size2, angle2);
    
    // Calculate bounding box overlap using rotated bounding boxes
    let overlap_x = rot_pos1.x.max(rot_pos2.x);
    let overlap_y = rot_pos1.y.max(rot_pos2.y);
    let overlap_w = (rot_pos1.x + rot_size1.x).min(rot_pos2.x + rot_size2.x) - overlap_x;
    let overlap_h = (rot_pos1.y + rot_size1.y).min(rot_pos2.y + rot_size2.y) - overlap_y;
    
    // Quick early exit if no bounding box overlap
    if overlap_w <= 0.0 || overlap_h <= 0.0 {
        return false; // No overlap
    }
    
    // If both masks are None, use simple bounding box collision
    if mask1_opt.is_none() && mask2_opt.is_none() {
        // If both are rotated but without transparency, use SAT algorithm
        if angle1 != 0.0 || angle2 != 0.0 {
            return check_rotated_rectangle_collision(
                pos1, size1, angle1,
                pos2, size2, angle2
            );
        }
        return true; // Bounding boxes overlap
    }
    
    // If at least one object has rotation, we need to use the rotation-aware collision code
    if angle1 != 0.0 || angle2 != 0.0 {
        return check_rotated_pixel_collision(
            obj1, obj2,
            &overlap_x, &overlap_y, &overlap_w, &overlap_h,
            skip_pixels
        );
    }
    
    // Handle case where only one mask is available (mixed case: one has transparency, one doesn't)
    if mask1_opt.is_some() && mask2_opt.is_none() {
        // Only obj1 has a mask
        return check_one_masked_collision(
            &pos1, &size1, &texture1_size, &mask1_opt.unwrap(),
            &pos2, &size2,
            &overlap_x, &overlap_y, &overlap_w, &overlap_h,
            skip_pixels
        );
    }
    
    if mask1_opt.is_none() && mask2_opt.is_some() {
        // Only obj2 has a mask
        return check_one_masked_collision(
            &pos2, &size2, &texture2_size, &mask2_opt.unwrap(),
            &pos1, &size1,
            &overlap_x, &overlap_y, &overlap_w, &overlap_h,
            skip_pixels
        );
    }
    
    // If we get here, both objects have masks but no rotation
    let mask1 = mask1_opt.unwrap();
    let mask2 = mask2_opt.unwrap();

    #[cfg(not(target_arch = "wasm32"))]
    {
        // Parallel processing (Rayon) on Linux/Windows
        return (0..overlap_h as usize).into_par_iter().step_by(skip_pixels).any(|y| {
            (0..overlap_w as usize).into_par_iter().step_by(skip_pixels).any(|x| {
                let world_point = Vec2::new(overlap_x + x as f32, overlap_y + y as f32);
                
                // Calculate texture coordinates for both objects
                let (tx1, ty1) = calc_tex_coord(world_point, pos1, size1, texture1_size);
                let (tx2, ty2) = calc_tex_coord(world_point, pos2, size2, texture2_size);
                
                // Calculate indices in mask arrays
                let idx1 = ty1 * texture1_size.x as usize + tx1;
                let idx2 = ty2 * texture2_size.x as usize + tx2;
                
                // Check both mask bits
                let mask1_bit = is_mask_bit_set(&mask1, idx1);
                let mask2_bit = is_mask_bit_set(&mask2, idx2);
                
                // If either mask check failed or one of the bits is not set, no collision
                mask1_bit.unwrap_or(false) && mask2_bit.unwrap_or(false)
            })
        });
    }

    #[cfg(target_arch = "wasm32")]
    {
        // Sequential for Web (WASM)
        for y in (0..overlap_h as usize).step_by(skip_pixels) {
            for x in (0..overlap_w as usize).step_by(skip_pixels) {
                let world_point = Vec2::new(overlap_x + x as f32, overlap_y + y as f32);
                
                // Calculate texture coordinates for both objects
                let (tx1, ty1) = calc_tex_coord(world_point, pos1, size1, texture1_size);
                let (tx2, ty2) = calc_tex_coord(world_point, pos2, size2, texture2_size);
                
                // Calculate indices in mask arrays
                let idx1 = ty1 * texture1_size.x as usize + tx1;
                let idx2 = ty2 * texture2_size.x as usize + tx2;
                
                // Check both mask bits
                let mask1_bit = is_mask_bit_set(&mask1, idx1);
                let mask2_bit = is_mask_bit_set(&mask2, idx2);
                
                // If both bits are set, we have a collision
                if mask1_bit.unwrap_or(false) && mask2_bit.unwrap_or(false) {
                    return true;
                }
            }
        }
        false
    }
}

// Helper function for collision detection when only one object has a mask
#[inline]
fn check_one_masked_collision(
    masked_pos: &Vec2,
    masked_size: &Vec2,
    masked_tex_size: &Vec2,
    mask: &Vec<u8>,
    other_pos: &Vec2,
    other_size: &Vec2,
    overlap_x: &f32,
    overlap_y: &f32,
    overlap_w: &f32,
    overlap_h: &f32,
    skip_pixels: usize
) -> bool {
    #[cfg(not(target_arch = "wasm32"))]
    {
        // Parallel processing for Linux/Windows
        return (0..*overlap_h as usize).into_par_iter().step_by(skip_pixels).any(|y| {
            (0..*overlap_w as usize).into_par_iter().step_by(skip_pixels).any(|x| {
                let world_point = Vec2::new(*overlap_x + x as f32, *overlap_y + y as f32);
                
                // Calculate texture coordinates for masked object
                let (tx, ty) = calc_tex_coord(world_point, *masked_pos, *masked_size, *masked_tex_size);
                
                // Calculate index in mask array
                let idx = ty * masked_tex_size.x as usize + tx;
                
                // Check mask bit
                let is_opaque = is_mask_bit_set(mask, idx).unwrap_or(false);
                
                // Check if point is in other object's bounds
                let in_other_bounds = is_point_in_bounds(world_point, *other_pos, *other_size);
                
                // Collision occurs if point is opaque in the masked object and also within the other object
                is_opaque && in_other_bounds
            })
        });
    }
    
    #[cfg(target_arch = "wasm32")]
    {
        // Sequential for Web (WASM)
        for y in (0..*overlap_h as usize).step_by(skip_pixels) {
            for x in (0..*overlap_w as usize).step_by(skip_pixels) {
                let world_point = Vec2::new(*overlap_x + x as f32, *overlap_y + y as f32);
                
                // Calculate texture coordinates for masked object
                let (tx, ty) = calc_tex_coord(world_point, *masked_pos, *masked_size, *masked_tex_size);
                
                // Calculate index in mask array
                let idx = ty * masked_tex_size.x as usize + tx;
                
                // Check mask bit
                let is_opaque = is_mask_bit_set(mask, idx).unwrap_or(false);
                
                // Check if point is in other object's bounds
                let in_other_bounds = is_point_in_bounds(world_point, *other_pos, *other_size);
                
                if is_opaque && in_other_bounds {
                    return true;
                }
            }
        }
        false
    }
}

// Helper function for collision detection when only one rotated object has a transparency mask
#[inline]
fn check_one_rotated_masked_collision(
    masked_pos: Vec2,
    masked_size: Vec2,
    masked_tex_size: Vec2,
    mask: Vec<u8>,
    masked_angle: f32,
    masked_center: Vec2,
    other_pos: Vec2,
    other_size: Vec2,
    other_angle: f32,
    other_center: Vec2,
    overlap_x: &f32,
    overlap_y: &f32,
    overlap_w: &f32,
    overlap_h: &f32,
    skip_pixels: usize
) -> bool {
    #[cfg(not(target_arch = "wasm32"))]
    {
        // Parallel processing for Linux/Windows
        return (0..*overlap_h as usize).into_par_iter().step_by(skip_pixels).any(|y| {
            (0..*overlap_w as usize).into_par_iter().step_by(skip_pixels).any(|x| {
                // For each pixel in the overlap region
                let world_point = Vec2::new(*overlap_x + x as f32, *overlap_y + y as f32);
                
                // Transform to masked object's local space (accounting for rotation)
                let local_masked_point = rotate_point(world_point, masked_center, -masked_angle);
                
                // Check if point is within masked object's bounds
                if !is_point_in_bounds(local_masked_point, masked_pos, masked_size) {
                    return false;
                }
                
                // Calculate texture coordinates for masked object
                let (tx, ty) = calc_tex_coord(local_masked_point, masked_pos, masked_size, masked_tex_size);
                
                // Check if pixel is opaque in masked object
                let idx = ty * masked_tex_size.x as usize + tx;
                let is_opaque = is_mask_bit_set(&mask, idx).unwrap_or(false);
                
                if !is_opaque {
                    return false;
                }
                
                // For the solid object, check if the point is inside its rotated bounds
                let local_other_point = rotate_point(world_point, other_center, -other_angle);
                let in_other_bounds = is_point_in_bounds(local_other_point, other_pos, other_size);
                
                is_opaque && in_other_bounds
            })
        });
    }
    
    #[cfg(target_arch = "wasm32")]
    {
        // Sequential for Web (WASM)
        for y in (0..*overlap_h as usize).step_by(skip_pixels) {
            for x in (0..*overlap_w as usize).step_by(skip_pixels) {
                // For each pixel in the overlap region
                let world_point = Vec2::new(*overlap_x + x as f32, *overlap_y + y as f32);
                
                // Transform to masked object's local space (accounting for rotation)
                let local_masked_point = rotate_point(world_point, masked_center, -masked_angle);
                
                // Check if point is within masked object's bounds
                if !is_point_in_bounds(local_masked_point, masked_pos, masked_size) {
                    continue;
                }
                
                // Calculate texture coordinates for masked object
                let (tx, ty) = calc_tex_coord(local_masked_point, masked_pos, masked_size, masked_tex_size);
                
                // Check if pixel is opaque in masked object
                let idx = ty * masked_tex_size.x as usize + tx;
                let is_opaque = is_mask_bit_set(&mask, idx).unwrap_or(false);
                
                if !is_opaque {
                    continue;
                }
                
                // For the solid object, check if the point is inside its rotated bounds
                let local_other_point = rotate_point(world_point, other_center, -other_angle);
                let in_other_bounds = is_point_in_bounds(local_other_point, other_pos, other_size);
                
                if is_opaque && in_other_bounds {
                    return true;
                }
            }
        }
        false
    }
}

// Helper function to handle pixel-perfect collision for rotated objects
fn check_rotated_pixel_collision<T, U>(
    obj1: &T,
    obj2: &U,
    overlap_x: &f32,
    overlap_y: &f32,
    overlap_w: &f32,
    overlap_h: &f32,
    skip_pixels: usize
) -> bool
where
    T: Collidable,
    U: Collidable,
{
    let pos1 = obj1.pos();
    let size1 = obj1.size();
    let mask1_opt = obj1.get_mask();
    let texture1_size = obj1.texture_size();
    let angle1 = obj1.get_angle();
    let center1 = Vec2::new(pos1.x + size1.x / 2.0, pos1.y + size1.y / 2.0);

    let pos2 = obj2.pos();
    let size2 = obj2.size();
    let mask2_opt = obj2.get_mask();
    let texture2_size = obj2.texture_size();
    let angle2 = obj2.get_angle();
    let center2 = Vec2::new(pos2.x + size2.x / 2.0, pos2.y + size2.y / 2.0);
    
    // Mixed case: Only one image has transparency
    if mask1_opt.is_some() && mask2_opt.is_none() {
        // Object 1 has transparency, object 2 doesn't
        return check_one_rotated_masked_collision(
            pos1, size1, texture1_size, mask1_opt.unwrap(), angle1, center1,
            pos2, size2, angle2, center2,
            overlap_x, overlap_y, overlap_w, overlap_h,
            skip_pixels
        );
    }
    
    if mask1_opt.is_none() && mask2_opt.is_some() {
        // Object 2 has transparency, object 1 doesn't
        return check_one_rotated_masked_collision(
            pos2, size2, texture2_size, mask2_opt.unwrap(), angle2, center2,
            pos1, size1, angle1, center1,
            overlap_x, overlap_y, overlap_w, overlap_h,
            skip_pixels
        );
    }
    
    // If both objects lack masks, use the SAT algorithm
    if mask1_opt.is_none() && mask2_opt.is_none() {
        return check_rotated_rectangle_collision(
            pos1, size1, angle1,
            pos2, size2, angle2
        );
    }
    
    // Both objects have transparency masks - use full pixel-perfect collision
    let mask1 = mask1_opt.unwrap();
    let mask2 = mask2_opt.unwrap();
    
    #[cfg(not(target_arch = "wasm32"))]
    {
        // Parallel processing (Rayon) for Linux/Windows
        return (0..*overlap_h as usize).into_par_iter().step_by(skip_pixels).any(|y| {
            (0..*overlap_w as usize).into_par_iter().step_by(skip_pixels).any(|x| {
                // For each pixel in the overlap region
                let world_point = Vec2::new(*overlap_x + x as f32, *overlap_y + y as f32);
                
                // Find the corresponding point in obj1's local space (accounting for rotation)
                let local_point1 = rotate_point(world_point, center1, -angle1); // Negative angle to reverse rotation
                
                // Find the corresponding point in obj2's local space (accounting for rotation)
                let local_point2 = rotate_point(world_point, center2, -angle2); // Negative angle to reverse rotation
                
                // Check if the point is inside both objects' bounds
                if !is_point_in_bounds(local_point1, pos1, size1) || 
                   !is_point_in_bounds(local_point2, pos2, size2) {
                    return false;
                }
                
                // Calculate texture coordinates for both objects
                let (tx1, ty1) = calc_tex_coord(local_point1, pos1, size1, texture1_size);
                let (tx2, ty2) = calc_tex_coord(local_point2, pos2, size2, texture2_size);
                
                // Calculate indices in mask arrays
                let idx1 = ty1 * texture1_size.x as usize + tx1;
                let idx2 = ty2 * texture2_size.x as usize + tx2;
                
                // Check both mask bits
                let mask1_bit = is_mask_bit_set(&mask1, idx1).unwrap_or(false);
                let mask2_bit = is_mask_bit_set(&mask2, idx2).unwrap_or(false);
                
                // If both bits are set, we have a collision at this pixel
                mask1_bit && mask2_bit
            })
        });
    }
    
    #[cfg(target_arch = "wasm32")]
    {
        // Sequential for Web (WASM)
        for y in (0..*overlap_h as usize).step_by(skip_pixels) {
            for x in (0..*overlap_w as usize).step_by(skip_pixels) {
                // For each pixel in the overlap region
                let world_point = Vec2::new(*overlap_x + x as f32, *overlap_y + y as f32);
                
                // Find the corresponding point in obj1's local space (accounting for rotation)
                let local_point1 = rotate_point(world_point, center1, -angle1); // Negative angle to reverse rotation
                
                // Find the corresponding point in obj2's local space (accounting for rotation)
                let local_point2 = rotate_point(world_point, center2, -angle2); // Negative angle to reverse rotation
                
                // Check if the point is inside both objects' bounds
                if !is_point_in_bounds(local_point1, pos1, size1) || 
                   !is_point_in_bounds(local_point2, pos2, size2) {
                    continue;
                }
                
                // Calculate texture coordinates for both objects
                let (tx1, ty1) = calc_tex_coord(local_point1, pos1, size1, texture1_size);
                let (tx2, ty2) = calc_tex_coord(local_point2, pos2, size2, texture2_size);
                
                // Calculate indices in mask arrays
                let idx1 = ty1 * texture1_size.x as usize + tx1;
                let idx2 = ty2 * texture2_size.x as usize + tx2;
                
                // Check both mask bits
                let mask1_bit = is_mask_bit_set(&mask1, idx1).unwrap_or(false);
                let mask2_bit = is_mask_bit_set(&mask2, idx2).unwrap_or(false);
                
                // If both bits are set, we have a collision at this pixel
                if mask1_bit && mask2_bit {
                    return true;
                }
            }
        }
        false
    }
}

// New function to check collision between two rotated rectangles
// This is much more efficient than pixel-perfect collision for solid images
fn check_rotated_rectangle_collision(
    pos1: Vec2, size1: Vec2, angle1: f32,
    pos2: Vec2, size2: Vec2, angle2: f32
) -> bool {
    // For very small angles, use regular AABB collision for better performance
    let small_angle_threshold = 0.05; // About 3 degrees
    if angle1.abs() < small_angle_threshold && angle2.abs() < small_angle_threshold {
        // Use regular AABB collision for performance
        return pos1.x < pos2.x + size2.x &&
               pos1.x + size1.x > pos2.x &&
               pos1.y < pos2.y + size2.y &&
               pos1.y + size1.y > pos2.y;
    }
    
    // For simplicity, we'll use the Separating Axis Theorem (SAT)
    // This is a common algorithm for detecting collision between convex polygons
    
    // Get the corners of both rectangles
    let center1 = Vec2::new(pos1.x + size1.x / 2.0, pos1.y + size1.y / 2.0);
    let center2 = Vec2::new(pos2.x + size2.x / 2.0, pos2.y + size2.y / 2.0);
    
    // Calculate half-widths and half-heights
    let half_width1 = size1.x / 2.0;
    let half_height1 = size1.y / 2.0;
    let half_width2 = size2.x / 2.0;
    let half_height2 = size2.y / 2.0;
    
    // Calculate the four corners of both rectangles
    let corners1 = [
        rotate_point(Vec2::new(center1.x - half_width1, center1.y - half_height1), center1, angle1),
        rotate_point(Vec2::new(center1.x + half_width1, center1.y - half_height1), center1, angle1),
        rotate_point(Vec2::new(center1.x + half_width1, center1.y + half_height1), center1, angle1),
        rotate_point(Vec2::new(center1.x - half_width1, center1.y + half_height1), center1, angle1)
    ];
    
    let corners2 = [
        rotate_point(Vec2::new(center2.x - half_width2, center2.y - half_height2), center2, angle2),
        rotate_point(Vec2::new(center2.x + half_width2, center2.y - half_height2), center2, angle2),
        rotate_point(Vec2::new(center2.x + half_width2, center2.y + half_height2), center2, angle2),
        rotate_point(Vec2::new(center2.x - half_width2, center2.y + half_height2), center2, angle2)
    ];
    
    // Calculate the edges of both rectangles
    let edges1 = [
        Vec2::new(corners1[1].x - corners1[0].x, corners1[1].y - corners1[0].y),
        Vec2::new(corners1[2].x - corners1[1].x, corners1[2].y - corners1[1].y),
        Vec2::new(corners1[3].x - corners1[2].x, corners1[3].y - corners1[2].y),
        Vec2::new(corners1[0].x - corners1[3].x, corners1[0].y - corners1[3].y)
    ];
    
    let edges2 = [
        Vec2::new(corners2[1].x - corners2[0].x, corners2[1].y - corners2[0].y),
        Vec2::new(corners2[2].x - corners2[1].x, corners2[2].y - corners2[1].y),
        Vec2::new(corners2[3].x - corners2[2].x, corners2[3].y - corners2[2].y),
        Vec2::new(corners2[0].x - corners2[3].x, corners2[0].y - corners2[3].y)
    ];
    
    // Collect all axes to test (perpendicular to edges)
    let mut axes = Vec::with_capacity(8);
    for edge in &edges1 {
        // Perpendicular vector, normalize only if length is significant
        let perp = Vec2::new(-edge.y, edge.x);
        let length = (perp.x * perp.x + perp.y * perp.y).sqrt();
        
        if length > 0.0001 {
            axes.push(Vec2::new(perp.x / length, perp.y / length));
        }
    }
    for edge in &edges2 {
        // Perpendicular vector, normalize only if length is significant
        let perp = Vec2::new(-edge.y, edge.x);
        let length = (perp.x * perp.x + perp.y * perp.y).sqrt();
        
        if length > 0.0001 {
            axes.push(Vec2::new(perp.x / length, perp.y / length));
        }
    }
    
    // Test all axes
    for axis in &axes {
        // Project corners onto axis
        let mut min1 = f32::MAX;
        let mut max1 = f32::MIN;
        let mut min2 = f32::MAX;
        let mut max2 = f32::MIN;
        
        for corner in &corners1 {
            let projection = corner.x * axis.x + corner.y * axis.y;
            min1 = min1.min(projection);
            max1 = max1.max(projection);
        }
        
        for corner in &corners2 {
            let projection = corner.x * axis.x + corner.y * axis.y;
            min2 = min2.min(projection);
            max2 = max2.max(projection);
        }
        
        // Check for gap
        if min1 > max2 || min2 > max1 {
            return false; // Gap found, no collision
        }
    }
    
    // No gap found on any axis, rectangles are colliding
    true
}

// Helper function to rotate a point around a center point
fn rotate_point(point: Vec2, center: Vec2, angle: f32) -> Vec2 {
    // Early return for zero angle to avoid unnecessary calculations
    if angle == 0.0 {
        return point;
    }
    
    // Translate point to origin
    let translated = Vec2::new(point.x - center.x, point.y - center.y);
    
    // Normalize angle to be between -π and π for better numerical stability
    let normalized_angle = {
        let mut a = angle % (2.0 * std::f32::consts::PI);
        if a > std::f32::consts::PI {
            a -= 2.0 * std::f32::consts::PI;
        } else if a < -std::f32::consts::PI {
            a += 2.0 * std::f32::consts::PI;
        }
        a
    };
    
    // Rotate - Note: positive angle is counter-clockwise
    let cos_angle = normalized_angle.cos();
    let sin_angle = normalized_angle.sin();
    
    // Standard 2D rotation formula
    let rotated_x = translated.x * cos_angle - translated.y * sin_angle;
    let rotated_y = translated.x * sin_angle + translated.y * cos_angle;
    
    // Translate back
    Vec2::new(rotated_x + center.x, rotated_y + center.y)
}

// Calculate the rotated bounding box dimensions
fn calculate_rotated_bounding_box(pos: Vec2, size: Vec2, angle: f32) -> (Vec2, Vec2) {
    if angle == 0.0 {
        return (pos, size);
    }
    
    let center = Vec2::new(pos.x + size.x / 2.0, pos.y + size.y / 2.0);
    
    // Calculate the four corners of the original rectangle
    let top_left = pos;
    let top_right = Vec2::new(pos.x + size.x, pos.y);
    let bottom_left = Vec2::new(pos.x, pos.y + size.y);
    let bottom_right = Vec2::new(pos.x + size.x, pos.y + size.y);
    
    // Rotate each corner around the center
    let rotated_tl = rotate_point(top_left, center, angle);
    let rotated_tr = rotate_point(top_right, center, angle);
    let rotated_bl = rotate_point(bottom_left, center, angle);
    let rotated_br = rotate_point(bottom_right, center, angle);
    
    // Find the min and max x,y coordinates to form the bounding box
    let min_x = rotated_tl.x.min(rotated_tr.x).min(rotated_bl.x).min(rotated_br.x);
    let min_y = rotated_tl.y.min(rotated_tr.y).min(rotated_bl.y).min(rotated_br.y);
    let max_x = rotated_tl.x.max(rotated_tr.x).max(rotated_bl.x).max(rotated_br.x);
    let max_y = rotated_tl.y.max(rotated_tr.y).max(rotated_bl.y).max(rotated_br.y);
    
    // Add a small margin to avoid missing edge collisions due to rounding errors
    // The margin is a small fraction of the object's size
    let margin_x = size.x * 0.02; // 2% of width
    let margin_y = size.y * 0.02; // 2% of height
    
    // Return the new position and size with margin
    (
        Vec2::new(min_x - margin_x, min_y - margin_y),
        Vec2::new(max_x - min_x + 2.0 * margin_x, max_y - min_y + 2.0 * margin_y)
    )
}
