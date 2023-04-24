use std::f32::consts::PI;
use std::time::Duration;

use iced::widget::canvas::{self, Cache, Canvas, Cursor, Fill, Frame, Geometry, Path, Program};
use iced::{executor, Application, Color, Command, Element, Length, Rectangle, Settings, Theme};
use iced_native::widget::Widget;
use xorshift::{Rng, SeedableRng, Xorshift128};

struct SlimeSim {
    sim: Sim,
}

#[derive(Debug)]
enum Message {
    Tick,
}

impl Application for SlimeSim {
    type Executor = executor::Default;

    type Message = Message;

    type Theme = Theme;

    type Flags = ();

    fn new(flags: Self::Flags) -> (Self, iced::Command<Self::Message>) {
        let agents = 10000;
        let sim = Sim {
            scale: 3.0,
            agents: (0..agents)
                .map(|a| a as f32 / (agents as f32) * 2.0 * PI)
                .map(|a| Agent {
                    x: 128.0,
                    y: 128.0,
                    a,
                })
                .collect(),
            pixs: (0..(256 * 256)).map(|_| 0).collect(),
            evap: 10,
            wander: 0.1,
            rng: Xorshift128::from_seed(&[2, 3]),
        };
        (Self { sim }, Command::none())
    }

    fn title(&self) -> String {
        String::from("My test prog")
    }

    fn subscription(&self) -> iced::Subscription<Self::Message> {
        iced::time::every(Duration::from_millis(1000 / 60)).map(|_| Message::Tick)
    }

    fn update(&mut self, message: Self::Message) -> iced::Command<Self::Message> {
        match message {
            Message::Tick => {
                self.sim.update_canvas();
                Command::none()
            }
        }
    }

    fn view(&self) -> iced::Element<'_, Self::Message, iced::Renderer<Self::Theme>> {
        self.sim.view()
    }
}

// First, we define the data we need for drawing
struct Sim {
    scale: f32,
    agents: Vec<Agent>,
    pixs: Vec<u8>,
    evap: u8,
    wander: f32,
    rng: Xorshift128,
}

#[derive(Debug)]
struct Agent {
    x: f32,
    y: f32,
    a: f32,
}

impl Sim {
    fn update_canvas(&mut self) {
        for pix in self.pixs.iter_mut() {
            *pix = pix.saturating_sub(self.evap);
        }

        for ag in self.agents.iter_mut() {
            let da = (self.rng.next_f32() * 2.0 - 1.0) * self.wander;
            let dx = ag.a.cos();
            let dy = ag.a.sin();
            ag.x += dx;
            ag.y += dy;
            ag.a += da;

            // TODO Fix reflections
            if ag.x < 0.0 {
                ag.x = -ag.x;
                ag.a = (180.0 - ag.a) % 360.0;
            }
            if ag.x > 256.0 {
                ag.x = 2.0 * 255.0 - ag.x;
                ag.a = (180.0 - ag.a) % 360.0;
            }
            if ag.y < 0.0 {
                ag.y = -ag.y;
                ag.a = (0.0 - ag.a) % 360.0;
            }
            if ag.y > 256.0 {
                ag.y = 2.0 * 255.0 - ag.y;
                ag.a = (0.0 - ag.a) % 360.0;
            }

            let x = (ag.x as usize).min(255);
            let y = (ag.y as usize).min(255);
            let index = (x * 256) + y;
            self.pixs[index] = 255;
        }
    }

    fn view(&self) -> Element<Message> {
        Canvas::new(self)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }
}

// Then, we implement the `Program` trait
impl Program<Message> for Sim {
    type State = ();

    fn draw(
        &self,
        _state: &(),
        _theme: &Theme,
        bounds: Rectangle,
        _cursor: Cursor,
    ) -> Vec<Geometry> {
        // We prepare a new `Frame`
        let mut frame = Frame::new(bounds.size());
        frame.fill_rectangle(
            [0, 0].into(),
            [256.0 * self.scale, 256.0 * self.scale].into(),
            Color::BLACK,
        );

        for (i, pix) in self.pixs.iter().enumerate() {
            let pos = [(i / 256) as f32 * self.scale, (i % 256) as f32 * self.scale];

            let pix_val: f32 = *pix as f32 / 256.0;
            // let pix_val: f32 = 0.5;

            frame.fill_rectangle(
                pos.into(),
                [self.scale, self.scale].into(),
                Color::new(1.0, 1.0, 1.0, pix_val),
            )
        }

        // Finally, we produce the geometry
        vec![frame.into_geometry()]
    }
}

fn main() {
    // Finally, we simply use our `Circle` to create the `Canvas`!
    println!("Hello, world!");
    SlimeSim::run(Settings {
        antialiasing: true,
        ..Settings::default()
    })
    .unwrap();
}
