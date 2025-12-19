use crate::{LabEvent, PickIntent, PointerChange, Radio};
use std::collections::HashMap;
use winit::dpi::PhysicalPosition;
use winit::event::{ElementState, MouseButton, MouseScrollDelta, Touch, TouchPhase, WindowEvent};

/// Handles all pointer events (mouse, touch, wheel) and converts them to lab events
pub struct PointerHandler {
    // Store active touches by their id
    active_touches: HashMap<u64, PhysicalPosition<f64>>,
    // Track the last distance between two touch points for pinch detection
    last_pinch_distance: Option<f64>,
    // Track current mouse position
    mouse_position: Option<PhysicalPosition<f64>>,
    // Track active touch count
    active_touch_count: usize,
    // Radio for sending events
    radio: Radio,
}

impl PointerHandler {
    pub fn new(radio: Radio) -> Self {
        Self {
            active_touches: HashMap::new(),
            last_pinch_distance: None,
            mouse_position: None,
            active_touch_count: 0,
            radio,
        }
    }

    /// Process window events and convert them to lab events
    pub fn process_window_event(&mut self, event: &WindowEvent) -> bool {
        match event {
            WindowEvent::Touch(touch_event) => {
                self.handle_touch_event(touch_event);
                true
            },
            WindowEvent::CursorMoved { position, .. } => {
                self.handle_cursor_moved(*position);
                true
            },
            WindowEvent::MouseInput { state, button, .. } => {
                self.handle_mouse_input(*state, *button);
                true
            },
            WindowEvent::MouseWheel { delta, .. } => {
                self.handle_mouse_wheel(*delta);
                true
            },
            WindowEvent::CursorLeft { .. } => {
                // Cursor left the window - release any active drag
                LabEvent::PointerChanged(PointerChange::Released(PickIntent::Reset)).send(&self.radio);
                true
            },
            _ => false, // Event not handled by this handler
        }
    }

    /// Handle touch events
    fn handle_touch_event(&mut self, touch: &Touch) {
        // Update active touch count
        match touch.phase {
            TouchPhase::Started => {
                self.active_touch_count += 1;
                self.active_touches.insert(touch.id, touch.location);
            },
            TouchPhase::Ended | TouchPhase::Cancelled => {
                if self.active_touch_count > 0 {
                    self.active_touch_count -= 1;
                }
                self.active_touches.remove(&touch.id);
                
                // Reset pinch state if all touches are gone
                if self.active_touches.is_empty() {
                    self.last_pinch_distance = None;
                }
            },
            TouchPhase::Moved => {
                self.active_touches.insert(touch.id, touch.location);
            },
        }

        // Detect pinch gestures and convert to zoom events
        if let Some(zoom_amount) = self.detect_pinch() {
            // If we detected a pinch, send a zoom event
            LabEvent::PointerChanged(PointerChange::Zoomed(zoom_amount)).send(&self.radio);
            return; // We've handled the pinch, no need to process further
        }
        
        // For non-pinch touches, only process if there's exactly one touch
        if self.active_touch_count != 1 {
            return; // Skip multi-touch events that aren't pinches
        }

        // Handle single touch events
        match touch.phase {
            TouchPhase::Started => {
                // Use the special TouchPressed variant that includes the position
                LabEvent::PointerChanged(PointerChange::TouchPressed(touch.location)).send(&self.radio);
            },
            TouchPhase::Moved => {
                LabEvent::PointerChanged(PointerChange::Moved(touch.location)).send(&self.radio);
            },
            TouchPhase::Ended | TouchPhase::Cancelled => {
                // Use the special TouchReleased variant
                LabEvent::PointerChanged(PointerChange::TouchReleased(PickIntent::Reset)).send(&self.radio);
            }
        }
    }

    /// Handle cursor moved events
    fn handle_cursor_moved(&mut self, position: PhysicalPosition<f64>) {
        self.mouse_position = Some(position);
        LabEvent::PointerChanged(PointerChange::Moved(position)).send(&self.radio);
    }

    /// Handle mouse input events
    fn handle_mouse_input(&mut self, state: ElementState, button: MouseButton) {
        let change = match state {
            ElementState::Pressed => PointerChange::Pressed,
            ElementState::Released => {
                let pick_intent = match button {
                    MouseButton::Right => PickIntent::Traverse,
                    _ => PickIntent::Select,
                };
                PointerChange::Released(pick_intent)
            }
        };
        
        LabEvent::PointerChanged(change).send(&self.radio);
    }

    /// Handle mouse wheel events
    fn handle_mouse_wheel(&mut self, delta: MouseScrollDelta) {
        let change = match delta {
            MouseScrollDelta::LineDelta(_, y) => PointerChange::Zoomed(y * 0.05),
            MouseScrollDelta::PixelDelta(position) => {
                PointerChange::Zoomed(position.y as f32 * 0.005)
            }
        };
        
        LabEvent::PointerChanged(change).send(&self.radio);
    }

    /// Detect pinch gesture and calculate zoom factor
    fn detect_pinch(&mut self) -> Option<f32> {
        // Only detect pinch if we have exactly 2 touches
        if self.active_touches.len() != 2 {
            return None;
        }
        
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
                
                // Return a small zoom amount to make it gradual and stable
                let zoom_amount = if scale_factor > 1.0 {
                    4.5 // Zoom in (fingers moving apart)
                } else {
                    -4.5 // Zoom out (fingers moving together)
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
