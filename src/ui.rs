use iced::widget::Text;
use iced_wgpu::Renderer;
use iced_winit::{Alignment, Color, Command, Element, Length, Program};
use iced_winit::widget::{Column, Row, slider, text_input};

pub struct Controls {
    measure_threshold: f32,
}

#[derive(Debug, Clone)]
pub enum Message {
    MeasureThresholdChanged(f32),
}

impl Controls {
    pub fn new() -> Self {
        Self {
            measure_threshold: 0.0,
        }
    }
}

impl Program for Controls {
    type Renderer = Renderer;
    type Message = Message;

    fn update(&mut self, message: Self::Message) -> Command<Self::Message> {
        match message {
            Message::MeasureThresholdChanged(new_threshold) => {
                self.measure_threshold = new_threshold;
            }
        }

        Command::none()
    }

    fn view(&self) -> Element<'_, Self::Message, Self::Renderer> {
        let sliders =
            Row::new()
                .width(Length::Units(500))
                .push(
                    slider(0.0f32..=1.0, self.measure_threshold, move |new_threshold| {
                        Message::MeasureThresholdChanged(new_threshold)
                    })
                        .step(0.01)
                );

        Row::new()
            .width(Length::Fill)
            .height(Length::Fill)
            .align_items(Alignment::End)
            .push(
                Column::new()
                    .width(Length::Fill)
                    .align_items(Alignment::End)
                    .push(
                        Column::new()
                            .padding(10)
                            .spacing(10)
                            .push(
                                Text::new("Background color")
                                    .style(Color::WHITE),
                            )
                            .push(sliders)
                    ),
            )
            .into()
    }
}
