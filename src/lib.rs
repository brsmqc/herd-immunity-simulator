#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use eframe::{
    egui,
    egui::{Color32, RichText},
};
use instant::Instant;
use rand::Rng;
use std::cmp::{max, min};
use std::time::Duration;
// use std::time::{Duration, Instant};
// #[cfg(target_arch = "wasm32")]

const X_SIZE: usize = 37;
const Y_SIZE: usize = 33;

#[derive(Clone, Copy, Debug)]
struct Cell {
    vaccinated: bool,
    infected: bool,
}

#[derive(Clone, Copy, Debug)]
struct Params {
    vac_left: i32,  // 0..100
    vac_right: i32, // 0..100
    right_same: bool,
    inf_rate_nonvac: i32, // 0..100
    inf_rate_vac: i32,    // 0..100
    inf_speed: f32,       // 0.1..10+ (multiplier)
}

impl Default for Params {
    fn default() -> Self {
        Self {
            vac_left: 50,
            vac_right: 50,
            right_same: true,
            inf_rate_nonvac: 90,
            inf_rate_vac: 10,
            inf_speed: 5.0,
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct ScheduledInfection {
    x: usize,
    y: usize,
    trigger_at: Instant,
}

pub struct App {
    grid: Vec<Cell>,
    params: Params,
    scheduled: Vec<ScheduledInfection>,
    total_vaccinated: usize,
    // cached colors (to mirror the JS idea of color-coding)
    color_vax: Color32,
    color_unvax: Color32,
    color_infected: Color32,
}

impl App {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        let mut app = Self {
            grid: vec![
                Cell {
                    vaccinated: false,
                    infected: false
                };
                X_SIZE * Y_SIZE
            ],
            params: Params::default(),
            scheduled: Vec::new(),
            total_vaccinated: 0,
            color_vax: Color32::from_hex("#8babf1").unwrap(),
            // Blue options: 8F7DE8 (darker), ADA0EE (lighter)
            color_unvax: Color32::from_hex("#c44601").unwrap(),
            color_infected: Color32::from_hex("#200024").unwrap(),
        };
        app.populate();
        app
    }

    fn idx(x: usize, y: usize) -> usize {
        y * X_SIZE + x
    }

    fn populate(&mut self) {
        self.scheduled.clear();
        let mut rng = rand::rng();
        let vac_left = self.params.vac_left as f64 / 100.0;
        let vac_right = if self.params.right_same {
            self.params.vac_left as f64 / 100.0
        } else {
            self.params.vac_right as f64 / 100.0
        };

        self.total_vaccinated = 0;
        for x in 0..X_SIZE {
            for y in 0..Y_SIZE {
                let vaccinated = if x < X_SIZE / 2 {
                    rng.random::<f64>() < vac_left
                } else {
                    rng.random::<f64>() < vac_right
                };
                let idx = Self::idx(x, y);
                self.grid[idx] = Cell {
                    vaccinated,
                    infected: false,
                };
                if vaccinated {
                    self.total_vaccinated += 1;
                }
            }
        }
    }

    fn stats(&self) -> (usize, usize, usize, f32, f32, f32, f32) {
        let total_pop = (X_SIZE * Y_SIZE) as f32;
        let total_vax = self.total_vaccinated as f32;
        let total_unvax = total_pop - total_vax;
        let num_infected = self.grid.iter().filter(|c| c.infected).count() as f32;
        let num_vax_infected = self
            .grid
            .iter()
            .filter(|c| c.infected && c.vaccinated)
            .count() as f32;
        let num_unvax_infected = num_infected - num_vax_infected;

        let percent_infected = if total_pop > 0.0 {
            100.0 * num_infected / total_pop
        } else {
            0.0
        };
        let percent_vax_pop_infected = if total_vax > 0.0 {
            100.0 * num_vax_infected / total_vax
        } else {
            0.0
        };
        let percent_unvax_pop_infected = if total_unvax > 0.0 {
            100.0 * num_unvax_infected / total_unvax
        } else {
            0.0
        };
        let percent_infected_vax = if num_infected > 0.0 {
            100.0 * num_vax_infected / num_infected
        } else {
            0.0
        };

        (
            num_infected as usize,
            num_vax_infected as usize,
            num_unvax_infected as usize,
            percent_infected,
            percent_vax_pop_infected,
            percent_unvax_pop_infected,
            percent_infected_vax,
        )
    }

    fn schedule_infection(&mut self, x: usize, y: usize, delay_ms: u64) {
        let trigger_at = Instant::now() + Duration::from_millis(delay_ms);
        self.scheduled.push(ScheduledInfection { x, y, trigger_at });
    }

    fn try_infect(&mut self, x: usize, y: usize) {
        let idx = Self::idx(x, y);
        if self.grid[idx].infected {
            return;
        }
        self.grid[idx].infected = true;

        // After infecting, consider neighbors with probability depending on vaccination status
        let (sx, ex) = (
            max(0, x as isize - 1) as usize,
            min(X_SIZE as isize - 1, x as isize + 1) as usize,
        );
        let (sy, ey) = (
            max(0, y as isize - 1) as usize,
            min(Y_SIZE as isize - 1, y as isize + 1) as usize,
        );
        let mut rng = rand::rng();
        for ix in sx..=ex {
            for iy in sy..=ey {
                let ii = Self::idx(ix, iy);
                if self.grid[ii].infected {
                    continue;
                }
                let chance = if self.grid[ii].vaccinated {
                    self.params.inf_rate_vac as f64 / 100.0
                } else {
                    self.params.inf_rate_nonvac as f64 / 100.0
                } as f64;
                if rng.random::<f64>() < chance {
                    let base_ms: f32 = 500.0 + 5000.0 * rng.random::<f32>();
                    let speed = self.params.inf_speed.max(0.01);
                    let delay = (base_ms / speed) as u64;
                    self.schedule_infection(ix, iy, delay);
                }
            }
        }
    }

    fn update_scheduled(&mut self) {
        let now = Instant::now();
        // Partition into ready and pending
        let mut i = 0;
        while i < self.scheduled.len() {
            if self.scheduled[i].trigger_at <= now {
                let s = self.scheduled.remove(i);
                self.try_infect(s.x, s.y);
            } else {
                i += 1;
            }
        }
    }

    fn draw_grid(&mut self, ui: &mut egui::Ui) {
        // Draw as a grid of clickable squares
        let cell_size = 15.0; // pixel size of each cell
        let border = 1.0;
        let full_size = cell_size + 2.0 * border;

        // Reserve the whole grid rect
        let desired_size = egui::vec2(full_size * X_SIZE as f32, full_size * Y_SIZE as f32);
        let (rect, _response) = ui.allocate_exact_size(desired_size, egui::Sense::hover());

        let painter = ui.painter_at(rect);

        for y in 0..Y_SIZE {
            for x in 0..X_SIZE {
                let idx = Self::idx(x, y);
                let cell = self.grid[idx];

                // Cell color
                let mut fill_color = if cell.vaccinated {
                    self.color_vax
                } else {
                    self.color_unvax
                };
                if cell.infected {
                    fill_color = self.color_infected;
                }

                // Compute rect for this cell
                let min = rect.min + egui::vec2(x as f32 * full_size, y as f32 * full_size);
                let max = min + egui::vec2(full_size, full_size);
                let r = egui::Rect::from_min_max(min, max);

                // Inner rect (inside border)
                let inner = r.shrink(border);

                // Interaction region
                let id = egui::Id::new((x, y));
                let response = ui.interact(r, id, egui::Sense::click_and_drag());

                // Fill cell
                painter.rect_filled(inner, 0.0, fill_color);

                // Hover overaly (semi-transparent white on top of the inner rect)
                if response.hovered() {
                    painter.rect_filled(inner, 0.0, Color32::from_white_alpha(40));
                }

                // Stroke border (always on top)
                painter.rect_stroke(r, 0.0, (border, Color32::BLACK), egui::StrokeKind::Inside);

                // Infect if clicked
                if response.clicked() {
                    self.try_infect(x, y);
                }
            }
        }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Set custom visuals
        let mut style = (*ctx.style()).clone();

        // Text color
        style.visuals.override_text_color = Some(Color32::BLACK);

        ctx.set_style(style);
        //ctx.set_visuals(egui::Visuals::dark());

        // progress scheduled infections
        self.update_scheduled();

        egui::TopBottomPanel::top("top").show(ctx, |ui| {
            ui.heading("Herd Immunity Simulator");
        });

        egui::SidePanel::left("controls").resizable(false).show(ctx, |ui| {
            ui.style_mut().text_styles.get_mut(&egui::TextStyle::Body).unwrap().size = 17.0;
            ui.style_mut().text_styles.get_mut(&egui::TextStyle::Button).unwrap().size = 17.0;
            ui.label(RichText::new("Vaccination Rates").strong());
            ui.add(egui::Slider::new(&mut self.params.vac_left, 0..=100).text("Left half"));
            ui.horizontal(|ui| {
                ui.checkbox(&mut self.params.right_same, "Right same as left");
                if self.params.right_same { self.params.vac_right = self.params.vac_left; }
            });
            ui.add_enabled_ui(!self.params.right_same, |ui| {
                ui.add(egui::Slider::new(&mut self.params.vac_right, 0..=100).text("Right half"));
            });
            if ui.button("Populate").clicked() { self.populate(); }
            ui.separator();

            ui.label(RichText::new("Infection Parameters").strong());
            ui.add(egui::Slider::new(&mut self.params.inf_rate_nonvac, 0..=100).text("Infection rate (unvaccinated)"));
            ui.add(egui::Slider::new(&mut self.params.inf_rate_vac, 0..=100).text("Infection rate (vaccinated)"));
            ui.add(egui::Slider::new(&mut self.params.inf_speed, 0.5..=10.0).text("Infection speed (multiplier)"));
            if ui.button("Clear Infections").clicked() {
                for c in &mut self.grid { c.infected = false; }
                self.scheduled.clear();
            }
            ui.separator();

            let (n_inf, n_vax_inf, n_unvax_inf, p_inf, p_vax_pop_inf, p_unvax_pop_inf, _p_inf_vax) = self.stats();
            ui.label(format!("Total population: {}", X_SIZE * Y_SIZE));
            ui.label(format!("Vaccinated: {}", self.total_vaccinated));
            ui.label(format!("Unvaccinated: {}", X_SIZE * Y_SIZE - self.total_vaccinated));
            ui.separator();
            ui.label(format!("Infected: {}", n_inf));
            ui.label(format!("— Vaccinated infected: {}", n_vax_inf));
            ui.label(format!("— Unvaccinated infected: {}", n_unvax_inf));
            ui.separator();
            ui.label(format!("% of population infected: {:.1}%", p_inf));
            ui.label(format!("% of vaccinated infected: {:.1}%", p_vax_pop_inf));
            ui.label(format!("% of unvaccinated infected: {:.1}%", p_unvax_pop_inf));
            //ui.label(format!("% of infections that are vaccinated: {:.1}%", p_inf_vax));
            ui.separator();
            ui.label("Tip: Click any square to seed an infection. Adjust sliders and click Populate to re-randomize vaccination.");
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            self.draw_grid(ui);
        });

        // request continuous repaint so scheduled infections can fire smoothly
        ctx.request_repaint_after(Duration::from_millis(16));
    }
}

// Web
#[cfg(target_arch = "wasm32")]
mod wasm {
    use crate::App;
    use eframe::wasm_bindgen::JsCast;
    use eframe::web_sys::HtmlCanvasElement;
    use wasm_bindgen::prelude::*;

    #[wasm_bindgen(start)]
    pub async fn run_wasm() -> Result<(), JsValue> {
        console_error_panic_hook::set_once();

        let doc = eframe::web_sys::window().unwrap().document().unwrap();
        let elem = doc
            .get_element_by_id("simulation")
            .expect("Missing <canvas id='simulation' in index.html");
        let canvas: HtmlCanvasElement = elem.dyn_into().unwrap();

        eframe::WebRunner::new()
            .start(
                canvas,
                eframe::WebOptions::default(),
                Box::new(|cc| Ok(Box::new(App::new(cc)))),
            )
            .await
    }
}
