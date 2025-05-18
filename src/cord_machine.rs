use reqwest::multipart::Form;
use reqwest::{self, Client};
use std::error::Error;
use std::sync::Arc;
use tokio::runtime::Runtime;
use tokio::task;
use urlencoding;

/// Controller for a wire-cutting/forming machine
pub struct CordMachine {
    base_url: String,
    client: Client,
    runtime: Arc<Runtime>,
}

impl CordMachine {
    pub fn new(http_host: &str) -> Result<Self, Box<dyn Error>> {
        let base_url = format!("http://{}", http_host);
        let client = Client::new();
        let runtime = Arc::new(Runtime::new()?);
        Ok(Self {
            base_url,
            client,
            runtime,
        })
    }

    pub fn wire_feed(&self, length: f32) -> Result<(), Box<dyn Error>> {
        let command = format!("G91\nG0 X{}\nG90", length);
        self.send_command(&command)
    }

    pub fn clamp_open(&self) -> Result<(), Box<dyn Error>> {
        self.send_command("G0 Z0")
    }

    pub fn clamp_close(&self) -> Result<(), Box<dyn Error>> {
        self.send_command("G0 Z1")
    }

    /// Send a G-code command to the machine
    pub fn send_command(&self, command: &str) -> Result<(), Box<dyn Error>> {
        let base_url = self.base_url.clone();
        let client = self.client.clone();
        let command = command.to_string();
        let runtime = self.runtime.clone();
        task::block_in_place(move || {
            runtime.block_on(async move {
                let url = format!(
                    "{}/command?commandText={}",
                    base_url,
                    urlencoding::encode(&command)
                );
                println!("Requesting {}", url);
                client.get(&url).send().await?;
                Ok::<(), Box<dyn Error>>(())
            })
        })?;
        Ok(())
    }

    /// Send a G-code command silently (without console response)
    pub fn send_command_silent(&self, command: &str) -> Result<(), Box<dyn Error>> {
        let base_url = self.base_url.clone();
        let client = self.client.clone();
        let runtime = self.runtime.clone();
        let command = command.to_string();
        task::block_in_place(move || {
            runtime.block_on(async move {
                let url = format!(
                    "{}/command_silent?commandText={}",
                    base_url,
                    urlencoding::encode(&command)
                );
                println!("Requesting {}", url);
                client.get(&url).send().await?;
                Ok::<(), Box<dyn Error>>(())
            })
        })?;
        Ok(())
    }

    /// Create and execute a G-code file to make a wire of the specified length
    pub fn make_wire(&self, length: f32) -> Result<(), Box<dyn Error>> {
        let base_url = self.base_url.clone();
        let client = self.client.clone();
        let gcode = self.template_make_wire(length);

        let runtime = self.runtime.clone();

        task::block_in_place(move || {
            runtime.block_on(async move {
                // Create form with file
                let form = Form::new().text("path", "/").part(
                    "myfile[]",
                    reqwest::multipart::Part::text(gcode)
                        .file_name("wire.g")
                        .mime_str("text/plain")?,
                );

                // Upload G-code file
                let upload_url = format!("{}/upload", base_url);
                println!("Uploading Gcode file to {}", upload_url);

                client.post(&upload_url).multipart(form).send().await?;

                // Execute the uploaded G-code
                let exec_url = format!(
                    "{}/command_silent?commandText={}",
                    base_url,
                    urlencoding::encode("M23 /WIREG~1.G\nM24")
                );

                println!("Requesting {}", exec_url);
                client.get(&exec_url).send().await?;

                Ok::<(), Box<dyn Error>>(())
            })
        })?;

        Ok(())
    }

    /// Generate a G-code template for making a wire
    fn template_make_wire(&self, wire_length: f32) -> String {
        format!(
            "M150 S0 R255 U255 B255 ; Set all lights to white\n\
            M150 I0 R255 U255 B255 ; Set backlight color\n\
            \n\
            ;First, make sure the clamp is open\n\
            ;The clamp motor is on the Z axis, and it's coordinates are defined as 2 for a whole rotation. That way, 0 is open (disengaged), and 1 is closed (engaged).\n\
            G90 ;Absolute movement mode\n\
            G0 Z0 F1000\n\
            \n\
            ;10mm is the width of the clamp, and this amount of thread is lost in the melting process.\n\
            G91 ;Relative movement mode\n\
            G0 X{} F5000 ; Move X axis {}mm\n\
            \n\
            ; Engage the clamp\n\
            G90 ;Absolute movement mode\n\
            G0 Z1 F1000\n\
            \n\
            M150 I0 R255 U0 B0 ; Set backlight color to red\n\
            \n\
            M150 I1 R255 U100 B0 ; Set button leds\n\
            M150 I2 R255 U100 B0 ; Set button leds\n\
            M0 Click to open clamp ; Pauses the program, shows message on display\n\
            M150 S0 R255 U255 B255 ; Set all lights to white\n\
            \n\
            ; Disengage the clamp\n\
            G90 ;Absolute movement mode\n\
            G0 Z0 F1000\n\
            M17 Z",
            wire_length, wire_length
        )
    }
}
