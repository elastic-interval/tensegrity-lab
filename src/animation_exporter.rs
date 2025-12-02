/*
 * Copyright (c) 2020. Beautiful Code BV, Rotterdam, Netherlands
 * Licensed under GNU GENERAL PUBLIC LICENSE Version 3.
 */

use crate::fabric::Fabric;
use cgmath::Point3;
use std::fs::File;
use std::io::{self, Write};
use std::path::PathBuf;
use zip::{write::FileOptions, CompressionMethod, ZipWriter};

/// Manages the export of animation frames to USD format for Blender
pub struct AnimationExporter {
    output_dir: PathBuf,
    frame_count: usize,
    fps: f64,
    capture_interval: usize,
    iteration_count: usize,
    enabled: bool,
    frame_data: Vec<(usize, String)>,
}

impl AnimationExporter {
    /// Create a new animation exporter
    pub fn new<P: Into<PathBuf>>(output_dir: P, fps: f64, capture_interval: usize) -> Self {
        Self {
            output_dir: output_dir.into(),
            frame_count: 0,
            fps,
            capture_interval,
            iteration_count: 0,
            enabled: false,
            frame_data: Vec::new(),
        }
    }

    /// Start capturing frames
    pub fn start(&mut self) {
        self.enabled = true;
        self.frame_count = 0;
        self.iteration_count = 0;
        self.frame_data.clear();
        println!("Animation export started (will save to ZIP when stopped)");
    }

    /// Stop capturing frames and finalize the animation
    pub fn stop(&mut self) -> io::Result<()> {
        if !self.enabled {
            return Ok(());
        }

        self.enabled = false;

        if self.frame_count == 0 {
            println!("No frames captured");
            return Ok(());
        }

        println!("Creating ZIP archive with {} frames...", self.frame_count);

        // Create ZIP file with timestamp
        let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S");
        let zip_path = self.output_dir.with_file_name(format!(
            "animation_export_{}.zip",
            timestamp
        ));
        let zip_file = File::create(&zip_path)?;
        let mut zip = ZipWriter::new(zip_file);

        let options: FileOptions<'_, ()> = FileOptions::default()
            .compression_method(CompressionMethod::Deflated);

        // Write all frame files to ZIP
        for (frame_num, content) in &self.frame_data {
            let filename = format!("frame_{:06}.usda", frame_num);
            zip.start_file(filename, options)?;
            zip.write_all(content.as_bytes())?;
        }

        // Create and write the main animation USD file
        let main_usd = Self::create_animation_sequence_usd(self.frame_count, self.fps);
        zip.start_file("animation.usda", options)?;
        zip.write_all(main_usd.as_bytes())?;

        // Add README
        let readme = format!(
            "USD Animation Export\n\
             ===================\n\n\
             Frames: {}\n\
             FPS: {}\n\n\
             To use in Blender:\n\
             1. Extract this ZIP file\n\
             2. File -> Import -> Universal Scene Description\n\
             3. Select animation.usda\n\n\
             Each frame includes camera position and orientation.\n",
            self.frame_count, self.fps
        );
        zip.start_file("README.txt", options)?;
        zip.write_all(readme.as_bytes())?;

        zip.finish()?;

        // Clear frame data from memory
        self.frame_data.clear();

        println!("Animation export completed!");
        println!("ZIP file: {:?}", zip_path);
        println!("Frames: {}", self.frame_count);
        println!("\nTo use in Blender:");
        println!("  1. Extract the ZIP file");
        println!("  2. File -> Import -> Universal Scene Description");
        println!("  3. Select animation.usda");

        Ok(())
    }

    /// Create the main animation USD file content
    fn create_animation_sequence_usd(frame_count: usize, fps: f64) -> String {
        let mut output = String::new();

        output.push_str("#usda 1.0\n");
        output.push_str("(\n");
        output.push_str("    defaultPrim = \"Animation\"\n");
        output.push_str("    metersPerUnit = 0.001\n");
        output.push_str("    upAxis = \"Y\"\n");
        output.push_str("    startTimeCode = 0\n");
        output.push_str(&format!("    endTimeCode = {}\n", frame_count.saturating_sub(1)));
        output.push_str(&format!("    timeCodesPerSecond = {}\n", fps));
        output.push_str(&format!("    framesPerSecond = {}\n", fps));
        output.push_str(")\n\n");
        output.push_str("def Xform \"Animation\"\n");
        output.push_str("{\n");

        // Create references to frame files
        for frame in 0..frame_count {
            output.push_str(&format!("    def \"Frame_{}\" (\n", frame));
            output.push_str(&format!(
                "        prepend references = @./frame_{:06}.usda@</Fabric>\n",
                frame
            ));
            output.push_str("    )\n");
            output.push_str("    {\n");
            output.push_str("    }\n");
        }

        output.push_str("}\n");

        output
    }

    /// Check if we should capture this iteration
    fn should_capture(&self) -> bool {
        self.enabled && (self.iteration_count % self.capture_interval == 0)
    }

    /// Capture a frame from the current fabric state with optional camera
    pub fn capture_frame(
        &mut self,
        fabric: &Fabric,
        camera_pos: Option<Point3<f32>>,
        camera_target: Option<Point3<f32>>,
    ) -> io::Result<()> {
        if !self.enabled {
            return Ok(());
        }

        self.iteration_count += 1;

        if self.should_capture() {
            // Generate USD content and store in memory
            let usd_content = fabric.to_usda_with_camera(camera_pos, camera_target)?;
            self.frame_data.push((self.frame_count, usd_content));
            self.frame_count += 1;

            if self.frame_count % 10 == 0 {
                println!("Captured {} frames...", self.frame_count);
            }
        }

        Ok(())
    }

    /// Get the current frame count
    pub fn frame_count(&self) -> usize {
        self.frame_count
    }

    /// Check if export is enabled
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Toggle export on/off, returning new state
    pub fn toggle(&mut self) -> io::Result<bool> {
        if self.enabled {
            self.stop()?;
            Ok(false)
        } else {
            self.start();
            Ok(true)
        }
    }
}
