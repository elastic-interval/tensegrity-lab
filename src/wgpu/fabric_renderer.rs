use crate::camera::Pick;
use crate::fabric::interval::Role;
use crate::fabric::material::Material;
use crate::fabric::{Fabric, UniqueId};
use crate::wgpu::Wgpu;
use crate::wgpu::cylinder_renderer::{CylinderInstance, CylinderRenderer};
use crate::wgpu::joint_renderer::JointRenderer;
use crate::{Appearance, AppearanceMode, IntervalDetails, JointDetails, RenderStyle};

pub struct FabricRenderer {
    // Cylinder renderer for intervals
    cylinder_renderer: CylinderRenderer,
    
    // Joint renderer for selected joints
    joint_renderer: JointRenderer,
}

impl FabricRenderer {
    pub fn new(wgpu: &Wgpu) -> Self {
        // Create the cylinder renderer for intervals
        let cylinder_renderer = CylinderRenderer::new(wgpu);
        
        // Create the joint renderer for selected joints
        let joint_renderer = JointRenderer::new(wgpu);
        
        Self {
            cylinder_renderer,
            joint_renderer,
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
        
        // Update the joint renderer to show selected joints
        self.joint_renderer.update(wgpu, fabric, pick);
    }

    pub fn render<'a>(
        &'a self,
        render_pass: &mut wgpu::RenderPass<'a>,
        bind_group: &'a wgpu::BindGroup,
    ) {
        // Render the cylinders for intervals
        self.cylinder_renderer.render(render_pass, bind_group);
        
        // Render the joint markers for selected joints
        self.joint_renderer.render(render_pass, bind_group);
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
                            // Use the Highlighted mode for intervals touching the selected joint
                            role_appearance.apply_mode(AppearanceMode::Highlighted)
                        } else {
                            // Use the Faded mode for intervals not touching the selected joint
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
                            // Use the appropriate mode based on the interval role
                            if interval.material.properties().role == Role::Pushing {
                                role_appearance.apply_mode(AppearanceMode::SelectedPush)
                            } else {
                                role_appearance.apply_mode(AppearanceMode::SelectedPull)
                            }
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
                                    // Use the Highlighted mode for adjacent intervals
                                    role_appearance.apply_mode(AppearanceMode::Highlighted)
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
                                // Use the Highlighted mode for adjacent intervals
                                role_appearance.apply_mode(AppearanceMode::Highlighted)
                            } else {
                                // Use the Faded mode for non-adjacent intervals
                                role_appearance.apply_mode(AppearanceMode::Faded)
                            }
                        }
                    }
                };
                instances.push(CylinderInstance {
                    start: [start.x, start.y, start.z],
                    radius_factor: appearance.radius,
                    end: [end.x, end.y, end.z],
                    material_type: material.role as u32,
                    color: appearance.color,
                });
            }
        }

        instances
    }
}
