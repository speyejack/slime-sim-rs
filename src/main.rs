use std::f32::consts::PI;
use std::time::{Duration, Instant};

use iced::widget::canvas::{self, Cache, Canvas, Cursor, Fill, Frame, Geometry, Path, Program};
use iced::widget::{container, slider, Slider};
use iced::{executor, Application, Color, Command, Element, Length, Rectangle, Settings, Theme};
use iced_native::column;
use iced_native::widget::Widget;
use vector2d::Vector2D;
use xorshift::{Rng, SeedableRng, Xorshift128};

struct SlimeSim {
    sim: Sim,
}

#[derive(Debug, Clone)]
enum Message {
    Tick,
    SteerUpdate(f32),
}

impl Application for SlimeSim {
    type Executor = executor::Default;

    type Message = Message;

    type Theme = Theme;

    type Flags = ();

    fn new(_: Self::Flags) -> (Self, iced::Command<Self::Message>) {
        let mut rng = Xorshift128::from_seed(&[2, 3]);
        let agents = 2000;
        let sim = Sim {
            scale: 3.0,
            agents: (0..agents)
                // .map(|a| a as f32 / (agents as f32) * 2.0 * PI)
                .map(|a| {
                    (
                        rng.next_f32() * 2.0 * PI,
                        (2.0 * rng.next_f32() - 1.0) * 30.0 + 128.0,
                        (2.0 * rng.next_f32() - 1.0) * 30.0 + 128.0,
                    )
                })
                .map(|(a, x, y)| Agent {
                    pos: Vector2D::new(x, y),
                    dir: from_angle(a),
                })
                .collect(),
            pixs: (0..(256 * 256)).map(|_| 0).collect(),
            evap: 10,
            wander: 3.0,
            steer: 05.0,
            turn_speed: 1.0,
            speed: 60.0,
            rng,
            last_update: Instant::now(),
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
            Message::SteerUpdate(u) => {
                self.sim.steer = u;
                Command::none()
            }
        }
    }

    fn view(&self) -> iced::Element<'_, Self::Message, iced::Renderer<Self::Theme>> {
        let slider = container(slider(0.0..=2.0, self.sim.steer, Message::SteerUpdate)).width(250);

        let sim_view = self.sim.view();
        container(column!(sim_view, slider))
            // container(column!(container(slider).width(Length::Fill).center_x()).spacing(0.25))
            .height(Length::Fill)
            .width(Length::Fill)
            .center_x()
            .center_y()
            .into()
    }
}

// First, we define the data we need for drawing
struct Sim {
    scale: f32,
    agents: Vec<Agent>,
    pixs: Vec<u8>,
    evap: u8,
    wander: f32,
    steer: f32,
    turn_speed: f32,
    speed: f32,
    rng: Xorshift128,
    last_update: Instant,
}

#[derive(Debug)]
struct Agent {
    dir: Vector2D<f32>,
    pos: Vector2D<f32>,
}

impl Sim {
    fn get_avg_neighbors(pixels: &[u8], ind: usize) -> u8 {
        let mut new_val = 0;
        for i in -1..=1 {
            for j in -1..=1 {
                let (x, y) = (((ind % 256) as i32) + i, (ind / 256) as i32 + j);
                let (x, y) = (x.max(0).min(255), y.max(0).min(255));
                let index = 256 * y + x;
                new_val += pixels[index as usize] as u32;
            }
        }

        ((new_val) / 9) as u8
    }
    fn update_canvas(&mut self) {
        let old_pixels = self.pixs.clone();
        for (ind, pix) in self.pixs.iter_mut().enumerate() {
            let new_val = Self::get_avg_neighbors(&old_pixels, ind).saturating_sub(self.evap);
            *pix = new_val
        }

        let since = self.last_update.elapsed().as_secs_f32();

        for ag in self.agents.iter_mut() {
            let mut pos = ag.pos;
            let dir = ag.dir;

            let mut steer_dir: Vector2D<f32> = Vector2D::new(0.0, 0.0);
            let ang = dir.angle();
            for off_angle in [-PI / 6.0, 0.0, PI / 6.0] {
                let probe_ang = ang + off_angle;
                let probe = from_angle(probe_ang) * 3.0;

                let index = get_index(pos + probe);
                let avg_val = Self::get_avg_neighbors(&self.pixs, index) as f32;
                steer_dir += probe * avg_val;
            }
            let steer_dir = steer_dir.normalise();

            // let dx = ag.a.cos();
            // let dy = ag.a.sin();

            let rn_angle = (self.rng.next_f32() * 2.0 - 1.0) * PI / 100.0;
            let rn_dir = from_angle(rn_angle + ang) * self.wander;
            // pos += da + steer_dir;
            // ag.a += (da + steer_angle) * self.turn_speed;
            let expected_step =
                dir + (rn_dir * self.wander + steer_dir * self.steer) * self.turn_speed;

            let step_dir = expected_step.normalise();
            let step_mag = since * self.speed;
            let step = step_dir * step_mag;

            pos += step;
            let mut dir = step_dir;

            if pos.x < 0.0 {
                pos.x = -pos.x;
                dir.x = -dir.x;
            }
            if pos.x > 256.0 {
                pos.x = 2.0 * 255.0 - pos.x;
                dir.x = -dir.x;
            }
            if pos.y < 0.0 {
                pos.y = -pos.y;
                dir.y = -dir.y;
            }
            if pos.y > 256.0 {
                pos.y = 2.0 * 255.0 - pos.y;
                dir.y = -dir.y;
            }

            let mut i = 0.0;
            while i < step_mag {
                let partial_step = step_dir * i;
                let index = get_index(ag.pos + partial_step);
                self.pixs[index] = 255;
                i += 0.5;
            }

            ag.pos = pos;
            ag.dir = dir;
        }
        self.last_update = Instant::now();
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
    SlimeSim::run(Settings {
        antialiasing: true,
        ..Settings::default()
    })
    .unwrap();
}

fn get_index(pos: Vector2D<f32>) -> usize {
    let x = (pos.x as usize).min(255);
    let y = (pos.y as usize).min(255);
    let index = (x * 256) + y;
    index
}

fn from_angle(angle: f32) -> Vector2D<f32> {
    Vector2D::new(angle.cos(), angle.sin())
}
