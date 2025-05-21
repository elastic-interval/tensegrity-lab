use std::collections::HashMap;
use winit::dpi::PhysicalPosition;
use winit::event::Touch;

/// Simple utility for detecting pinch gestures and calculating zoom factors
pub struct PinchDetector {
    // Store active touches by their id
    active_touches: HashMap<u64, PhysicalPosition<f64>>,
    // Track the last distance between two touch points for pinch detection
    last_pinch_distance: Option<f64>,
}

impl PinchDetector {
    pub fn new() -> Self {
        Self {
            active_touches: HashMap::new(),
            last_pinch_distance: None,
        }
    }

    /// Process a touch event and return a zoom factor if a pinch is detected
    /// Returns None if no pinch is detected or if there's only one touch
    /// Returns Some(factor) if a pinch is detected, where:
    /// - factor > 1.0 means zoom in (fingers moving apart)
    /// - factor < 1.0 means zoom out (fingers moving together)
    pub fn process_touch(&mut self, touch: &Touch) -> Option<f32> {
        match touch.phase {
            winit::event::TouchPhase::Started => {
                // Add this touch to our active touches
                self.active_touches.insert(touch.id, touch.location);
                None
            },
            winit::event::TouchPhase::Moved => {
                // Update this touch's position
                self.active_touches.insert(touch.id, touch.location);
                
                // Only detect pinch if we have exactly 2 touches
                if self.active_touches.len() == 2 {
                    self.detect_pinch()
                } else {
                    None
                }
            },
            winit::event::TouchPhase::Ended | winit::event::TouchPhase::Cancelled => {
                // Remove this touch
                self.active_touches.remove(&touch.id);
                
                // Reset pinch state if all touches are gone
                if self.active_touches.is_empty() {
                    self.last_pinch_distance = None;
                }
                
                None
            }
        }
    }
    
    /// Detect pinch gesture and calculate zoom factor
    fn detect_pinch(&mut self) -> Option<f32> {
        // Get the two touch points
        let touch_points: Vec<PhysicalPosition<f64>> = self.active_touches.values().cloned().collect();
        
        // Calculate current distance between touch points
        let current_distance = calculate_distance(touch_points[0], touch_points[1]);
        
        // Calculate zoom factor if we have a previous distance
        if let Some(last_distance) = self.last_pinch_distance {
            let scale_factor = (current_distance / last_distance) as f32;
            
            // Only register significant changes to avoid jitter
            // Use a higher threshold to prevent too many zoom events
            if (scale_factor - 1.0).abs() > 0.03 {
                // Update last distance for next time
                self.last_pinch_distance = Some(current_distance);
                
                // Return a very small zoom amount to make it gradual and stable
                // This prevents the black screen issue
                // Double the zoom rate while maintaining stability
                let zoom_amount = if scale_factor > 1.0 {
                    0.16 // Zoom in (fingers moving apart)
                } else {
                    -0.16 // Zoom out (fingers moving together)
                };
                
                return Some(zoom_amount);
            }
        } else {
            // Initialize pinch detection
            self.last_pinch_distance = Some(current_distance);
        }
        
        None
    }
}

/// Calculate distance between two points
fn calculate_distance(p1: PhysicalPosition<f64>, p2: PhysicalPosition<f64>) -> f64 {
    let dx = p2.x - p1.x;
    let dy = p2.y - p1.y;
    (dx * dx + dy * dy).sqrt()
}
