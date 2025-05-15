use crate::camera::Pick;
use crate::fabric::interval::Role;
use crate::fabric::material::Material;
use crate::fabric::{Fabric, UniqueId};
use crate::wgpu::attachment_renderer::AttachmentRenderer;
use crate::wgpu::cylinder_renderer::{CylinderInstance, CylinderRenderer};
use crate::wgpu::joint_renderer::JointRenderer; // Keep for now to avoid compilation errors
use crate::wgpu::Wgpu;
use crate::{Appearance, AppearanceMode, IntervalDetails, JointDetails, RenderStyle};

pub struct FabricRenderer {
    // Cylinder renderer for intervals
    cylinder_renderer: CylinderRenderer,
    
    // Joint renderer for selected joints
    joint_renderer: JointRenderer,
    
    // Attachment point renderer for push intervals
    attachment_renderer: AttachmentRenderer,
}

impl FabricRenderer {
    pub fn new(wgpu: &Wgpu) -> Self {
        // Create the cylinder renderer for intervals
        let cylinder_renderer = CylinderRenderer::new(wgpu);
        
        // Create the joint renderer for selected joints
        let joint_renderer = JointRenderer::new(wgpu);
        
        // Create the attachment point renderer for push intervals
        let attachment_renderer = AttachmentRenderer::new(wgpu);
        
        Self {
            cylinder_renderer,
            joint_renderer,
            attachment_renderer,
        }
    }

    pub fn update_from_fabric(
        &mut self,
        wgpu: &Wgpu,
        fabric: &Fabric,
        pick: &Pick,
        render_style: &mut RenderStyle,
    ) {
        // Create instances from fabric data
        let instances = self.create_instances_from_fabric(fabric, pick, render_style);
        
        // Update the cylinder renderer with the new instances
        self.cylinder_renderer.update(wgpu, &instances);
        
        // Enable joint renderer for joints that don't have connected push intervals
        self.joint_renderer.update(wgpu, fabric, pick);
        
        // Update the attachment point renderer to show attachment points on selected push intervals and joints
        self.attachment_renderer.update(wgpu, fabric, pick);
    }

    pub fn render<'a>(
        &'a self,
        render_pass: &mut wgpu::RenderPass<'a>,
        bind_group: &'a wgpu::BindGroup,
    ) {
        // Render the cylinders for intervals
        self.cylinder_renderer.render(render_pass, bind_group);
        
        // Render joint markers for selected joints without push intervals
        self.joint_renderer.render(render_pass, bind_group);
        
        // Render the attachment points for selected push intervals and joints
        self.attachment_renderer.render(render_pass, bind_group);
    }

    // Create instances from fabric intervals - minimal CPU processing
    fn create_instances_from_fabric(
        &self,
        fabric: &Fabric,
        pick: &Pick,
        render_style: &RenderStyle,
    ) -> Vec<CylinderInstance> {
        use RenderStyle::*;
        let mut instances = Vec::with_capacity(fabric.intervals.len());
        for (index, interval_opt) in fabric.intervals.iter().enumerate() {
            if let Some(interval) = interval_opt {
                let interval_id = UniqueId(index);
                let push = interval.material == Material::Push;
                match render_style {
                    WithPullMap(_) if push => continue,
                    WithPushMap(_) if !push => continue,
                    _ => {}
                }
                let (alpha, omega) = (interval.alpha_index, interval.omega_index);
                let start = fabric.joints[alpha].location;
                let end = fabric.joints[omega].location;
                let material = interval.material.properties();
                let role_appearance = material.role.appearance();
                let appearance = match pick {
                    Pick::Nothing => match render_style {
                        Normal => role_appearance,
                        WithAppearanceFunction(appearance) => {
                            appearance(interval).unwrap_or(role_appearance)
                        }
                        WithPullMap(color_map) | WithPushMap(color_map) => {
                            let key = interval.key();
                            match color_map.get(&key) {
                                None => role_appearance.apply_mode(AppearanceMode::Faded),
                                Some(color) => Appearance {
                                    color: *color,
                                    radius: role_appearance.radius * 2.0, // Double thickness for colored intervals
                                },
                            }
                        }
                    },
                    Pick::Joint(JointDetails { index, .. }) => {
                        if interval.touches(*index) {
                            role_appearance.highlighted_for_role(interval.material.properties().role)
                        } else {
                            role_appearance.apply_mode(AppearanceMode::Faded)
                        }
                    }
                    Pick::Interval(IntervalDetails {
                        near_joint,
                        far_joint,
                        id,
                        role,
                        original_interval_id,
                        ..
                    }) => {
                        // If this is the currently selected interval, highlight it based on its type
                        if *id == interval_id {
                            // Use the appropriate selected mode based on interval role
                            role_appearance.selected_for_role(interval.material.properties().role)
                        } 
                        // We no longer need to handle the originally selected push interval separately
                        // This was causing push intervals to remain purple after being deselected
                        else if let Some(orig_id) = original_interval_id {
                            // Only highlight the interval if it's currently selected
                            if *orig_id == interval_id && *id == interval_id && interval.material.properties().role == Role::Pushing {
                                role_appearance.apply_mode(AppearanceMode::SelectedPush)
                            } else {
                                // For all other intervals, check if they're adjacent to either joint
                                // When a push interval is selected, we want to keep intervals on both near and far joints highlighted
                                // When an interval from the far joint is selected, we still want to highlight intervals on the near joint
                                let active = match role {
                                    Role::Pushing => {
                                        // For push intervals, consider intervals adjacent to both near and far joints
                                        // Also check if the interval touches the original interval's near or far joint
                                        if let Some(orig_id) = original_interval_id {
                                            // Get the original interval's near joint
                                            let orig_interval = fabric.interval(*orig_id);
                                            let orig_near = orig_interval.alpha_index;
                                            
                                            // Check if the interval touches any of the relevant joints
                                            interval.touches(*near_joint) || interval.touches(*far_joint) || 
                                            interval.touches(orig_near)
                                        } else {
                                            interval.touches(*near_joint) || interval.touches(*far_joint)
                                        }
                                    }
                                    Role::Pulling => interval.touches(*near_joint),
                                    Role::Springy => false,
                                };
                                if active {
                                    // Use the appropriate highlighted mode based on interval role
                                    role_appearance.highlighted_for_role(interval.material.properties().role)
                                } else {
                                    // Use the Faded mode for non-adjacent intervals
                                    role_appearance.apply_mode(AppearanceMode::Faded)
                                }
                            }
                        } else {
                            // For intervals without an original_interval_id
                            // This case is less common but we should maintain consistent behavior
                            let active = match role {
                                Role::Pushing => {
                                    // For push intervals, consider intervals adjacent to both near and far joints
                                    interval.touches(*near_joint) || interval.touches(*far_joint)
                                }
                                Role::Pulling => interval.touches(*near_joint),
                                Role::Springy => false,
                            };
                            if active {
                                // Use the appropriate highlighted mode based on interval role
                                role_appearance.highlighted_for_role(interval.material.properties().role)
                            } else {
                                // Use the Faded mode for non-adjacent intervals
                                role_appearance.apply_mode(AppearanceMode::Faded)
                            }
                        }
                    }
                };
                // Check if this is a pull interval connected to a selected push interval or joint
                let mut modified_start = start;
                let mut modified_end = end;
                
                // For pull intervals, we need to connect them to attachment points on push intervals
                if interval.material.properties().role == Role::Pulling {
                    // Regardless of what's selected, we need to check both ends of the pull interval
                    // and connect them to attachment points if they're connected to a push interval
                    
                    // Process both ends of the pull interval
                    let joint_indices = [interval.alpha_index, interval.omega_index];
                    let modified_points = [&mut modified_start, &mut modified_end];
                    
                    // For each end of the pull interval
                    for (i, joint_index) in joint_indices.iter().enumerate() {
                        // Find all push intervals connected to this joint
                        for push_opt in fabric.intervals.iter() {
                            if let Some(push_interval) = push_opt {
                                // Only consider push intervals
                                if push_interval.material.properties().role == Role::Pushing {
                                    // Check if this push interval is connected to the current joint
                                    if push_interval.touches(*joint_index) {
                                        // Get attachment points for this push interval
                                        if let Ok((alpha_points, omega_points)) = push_interval.attachment_points(&fabric.joints) {
                                            let pull_midpoint = interval.midpoint(&fabric.joints);
                                            
                                            // Determine which end of the push interval is connected to the joint
                                            if push_interval.alpha_index == *joint_index {
                                                // The push interval's alpha end is connected to the joint
                                                let (nearest_idx, _) = push_interval.find_nearest_point(&alpha_points, pull_midpoint);
                                                *modified_points[i] = alpha_points[nearest_idx].position;
                                            } else if push_interval.omega_index == *joint_index {
                                                // The push interval's omega end is connected to the joint
                                                let (nearest_idx, _) = push_interval.find_nearest_point(&omega_points, pull_midpoint);
                                                *modified_points[i] = omega_points[nearest_idx].position;
                                            }
                                            
                                            // We found a push interval for this joint, no need to check others
                                            break;
                                        }
                                    }
                                }
                            }
                        }
                    }
                    
                    // Additional processing for selected elements
                    match pick {
                        // If a push interval is selected, we want to ensure that pull intervals
                        // connected to it are properly visualized
                        Pick::Interval(IntervalDetails { id, .. }) => {
                            if let Some(push_interval) = fabric.intervals[id.0].as_ref() {
                                if push_interval.material.properties().role == Role::Pushing {
                                    // We've already handled the basic case above, but we might need
                                    // additional logic for selected push intervals if needed
                                }
                            }
                        },
                        // If a joint is selected, we've already handled it in the general case above
                        Pick::Joint(_) => {},
                        _ => {}
                    }
                }
                
                instances.push(CylinderInstance {
                    start: [modified_start.x, modified_start.y, modified_start.z],
                    radius_factor: appearance.radius,
                    end: [modified_end.x, modified_end.y, modified_end.z],
                    material_type: material.role as u32,
                    color: appearance.color,
                });
            }
        }

        instances
    }
}
