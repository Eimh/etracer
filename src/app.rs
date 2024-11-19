use std::future::Future;
use std::sync::mpsc::{channel, Receiver, Sender};

use egui::{
    Color32, ColorImage, Frame, Pos2, Rect, Rounding, Sense, TextureHandle, TextureId,
    TextureOptions, Vec2,
};
use image::DynamicImage;
//use pdf_writer::Pdf;
#[derive(Debug, PartialEq, Copy, Clone)]
enum Units {
    Inches,
    Centimeters,
}

#[derive(Debug, PartialEq, Copy, Clone)]
enum Page {
    Letter,
    A4,
    Legal,
    Tabloid,
}

impl Page {
    fn size(&self) -> Vec2 {
        match self {
            Page::A4 => Vec2::new(8.3, 11.7),
            Page::Legal => Vec2::new(8.5, 14.0),
            Page::Letter => Vec2::new(8.5, 11.0),
            Page::Tabloid => Vec2::new(11.0, 17.0),
        }
    }
}
/// We derive Deserialize/Serialize so we can persist app state on shutdown.
//#[derive(serde::Deserialize, serde::Serialize)]
//#[serde(default)] // if we add new fields, give them default values when deserializing old state
pub struct EtracerApp {
    // Example stuff:
    label: String,
    image_channel: (Sender<Vec<u8>>, Receiver<Vec<u8>>),
    image_data: Option<DynamicImage>,
    raw_data: Option<Vec<u8>>,
    texture_handle: Option<TextureHandle>,
    texture_id: Option<TextureId>,
    //#[serde(skip)] // This how you opt-out of serialization of a field
    desired_width: f32,
    desired_height: f32,
    units: Units,
    page_size: Page,
    maintain_aspect_ratio: bool,
}

impl Default for EtracerApp {
    fn default() -> Self {
        Self {
            // Example stuff:
            label: "Hello World!".to_owned(),
            image_channel: channel(),
            image_data: None,
            raw_data: None,
            desired_width: 8.26,
            desired_height: 15.0,
            maintain_aspect_ratio: false,
            texture_handle: None,
            texture_id: None,
            units: Units::Inches,
            page_size: Page::Letter,
        }
    }
}

impl EtracerApp {
    /// Called once before the first frame.
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // This is also where you can customize the look and feel of egui using
        // `cc.egui_ctx.set_visuals` and `cc.egui_ctx.set_fonts`.
        egui_extras::install_image_loaders(&cc.egui_ctx);

        // Load previous app state (if any).
        // Note that you must enable the `persistence` feature for this to work.
        if let Some(storage) = cc.storage {
            //return eframe::get_value(storage, eframe::APP_KEY).unwrap_or_default();
        }

        Default::default()
    }
}

impl eframe::App for EtracerApp {
    /// Called by the frame work to save state before shutdown.
    // fn save(&mut self, storage: &mut dyn eframe::Storage) {
    //     eframe::set_value(storage, eframe::APP_KEY, self);
    // }

    /// Called each time the UI needs repainting, which may be many times per second.
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Put your widgets into a `SidePanel`, `TopBottomPanel`, `CentralPanel`, `Window` or `Area`.
        // For inspiration and more examples, go to https://emilk.github.io/egui
        ctx.set_theme(egui::Theme::Dark);
        let is_web = cfg!(target_arch = "wasm32");
        if !is_web {
            egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
                // The top panel is often a good place for a menu bar:

                egui::menu::bar(ui, |ui| {
                    // NOTE: no File->Quit on web pages!

                    ui.menu_button("File", |ui| {
                        if ui.button("Quit").clicked() {
                            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                        }
                    });
                    ui.add_space(16.0);

                    //egui::widgets::global_theme_preference_buttons(ui);
                    ctx.set_theme(egui::Theme::Dark);
                });
            });
        }

        egui::SidePanel::left("left").show(ctx, |ui| {
            //4, 13, 18
            //24, 61, 61
            //92, 131, 116
            //147, 177, 166
            // ui.style_mut().visuals.extreme_bg_color = Color32::from_rgb(4, 13, 18);
            // ui.style_mut().visuals.widgets.inactive.weak_bg_fill = Color32::from_rgb(24, 61, 61);
            // ui.style_mut().visuals.widgets.inactive.bg_fill = Color32::from_rgb(92, 131, 116);
            // ui.style_mut().visuals.widgets.inactive.rounding = Rounding::same(10.0);
            let btn_load = ui.button("load");

            if btn_load.clicked() {
                let sender = self.image_channel.0.clone();
                //open_file_picker();
                let open_file = rfd::AsyncFileDialog::new().pick_file();
                let ctx = ui.ctx().clone();
                execute(async move {
                    let file_opt = open_file.await;
                    if let Some(file) = file_opt {
                        let data = file.read().await;
                        sender.send(data);
                        ctx.request_repaint();
                    }
                });
            }

            match &self.image_data {
                Some(data) => ui.label(format!(
                    "Loaded image with dimensions: {} x {}.",
                    data.width(),
                    data.height()
                )),
                None => ui.label("No Image Loaded."),
            };

            ui.separator();
            ui.add(egui::Checkbox::new(
                &mut self.maintain_aspect_ratio,
                "Maintain aspect ratio",
            ));
            ui.add(
                egui::Slider::from_get_set(0.1..=100.0, |v| match v {
                    Some(val) => {
                        self.desired_width = match self.units {
                            Units::Inches => val as f32,
                            Units::Centimeters => val as f32 / 2.54,
                        };
                        if self.maintain_aspect_ratio && self.image_data.is_some() {
                            self.desired_height = (self.desired_width as f64
                                * self.image_data.as_ref().unwrap().height() as f64
                                / self.image_data.as_ref().unwrap().width() as f64)
                                as f32;
                        }
                        val
                    }
                    None => match self.units {
                        Units::Inches => self.desired_width as f64,
                        Units::Centimeters => self.desired_width as f64 * 2.54,
                    },
                })
                .text("Desired width"),
            );
            ui.add(
                egui::Slider::from_get_set(0.1..=100.0, |v| match v {
                    Some(val) => {
                        self.desired_height = match self.units {
                            Units::Inches => val as f32,
                            Units::Centimeters => val as f32 / 2.54,
                        };
                        if self.maintain_aspect_ratio && self.image_data.is_some() {
                            self.desired_width = (self.desired_height as f64
                                * self.image_data.as_ref().unwrap().width() as f64
                                / self.image_data.as_ref().unwrap().height() as f64)
                                as f32;
                        }
                        val
                    }
                    None => match self.units {
                        Units::Inches => self.desired_height as f64,
                        Units::Centimeters => self.desired_height as f64 * 2.54,
                    },
                })
                .text("Desired height"),
            );

            egui::ComboBox::from_label("Units")
                .selected_text(format!("{:?}", self.units))
                .show_ui(ui, |ui| {
                    ui.selectable_value(
                        &mut self.units,
                        Units::Inches,
                        format!("{:?}", Units::Inches),
                    );
                    ui.selectable_value(
                        &mut self.units,
                        Units::Centimeters,
                        format!("{:?}", Units::Centimeters),
                    );
                });
            let multiplier = match self.units {
                Units::Inches => 1.0,
                Units::Centimeters => 2.54,
            };
            egui::ComboBox::from_label("Page")
                .selected_text(format!("{:?}", self.page_size))
                .show_ui(ui, |ui| {
                    ui.selectable_value(
                        &mut self.page_size,
                        Page::Letter,
                        format!(
                            "{:?} ({:.2}x{:.2})",
                            Page::Letter,
                            Page::Letter.size().x * multiplier,
                            Page::Letter.size().y * multiplier
                        ),
                    );
                    ui.selectable_value(
                        &mut self.page_size,
                        Page::A4,
                        format!(
                            "{:?} ({:.2}x{:.2})",
                            Page::A4,
                            Page::A4.size().x * multiplier,
                            Page::A4.size().y * multiplier
                        ),
                    );
                    ui.selectable_value(
                        &mut self.page_size,
                        Page::Legal,
                        format!(
                            "{:?} ({:.2}x{:.2})",
                            Page::Legal,
                            Page::Legal.size().x * multiplier,
                            Page::Legal.size().y * multiplier
                        ),
                    );
                    ui.selectable_value(
                        &mut self.page_size,
                        Page::Tabloid,
                        format!(
                            "{:?} ({:.2}x{:.2})",
                            Page::Tabloid,
                            Page::Tabloid.size().x * multiplier,
                            Page::Tabloid.size().y * multiplier
                        ),
                    );
                });
            ui.separator();

            if ui.button("save").clicked() {
                let z = rfd::AsyncFileDialog::new().set_title("agh.pdf").save_file();
                let d = self.raw_data.as_ref().unwrap().clone();
                let dh = self.desired_height;
                let dw = self.desired_width;
                let p = self.page_size;
                execute(async move {
                    let q = z.await;
                    if let Some(file) = q {
                        let res = file.write(&generate_pdf(dw, dh, p, &d)).await;
                    }
                });
            }
            ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
                egui::warn_if_debug_build(ui);
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            if let Ok(img_data) = self.image_channel.1.try_recv() {
                let mut image = egui::Image::from_bytes("bytes://image.png", img_data.clone());
                image = image.max_size(Vec2::new(400.0, 400.0));
                image = image.fit_to_original_size(1.0);

                self.raw_data = Some(img_data.clone());
                let res = ctx.try_load_texture(
                    image.uri().unwrap(),
                    TextureOptions::default(),
                    egui::SizeHint::Scale(egui::emath::OrderedFloat(1.0)),
                );
                let im = image::load_from_memory(&img_data).unwrap();
                self.image_data = Some(im.clone());
                let size = [im.width() as _, im.height() as _];
                let ci = ColorImage::from_rgba_unmultiplied(
                    size,
                    im.to_rgba8().as_flat_samples().as_slice(),
                );
                self.texture_handle =
                    Some(ctx.load_texture("my_image", ci, TextureOptions::default()));
                self.texture_id = Some(TextureId::from(self.texture_handle.as_ref().unwrap()));
            }

            Frame::canvas(ui.style()).show(ui, |ui| {
                let draw_area = ui.available_rect_before_wrap();
                let (response, painter) =
                    ui.allocate_painter(ui.available_size_before_wrap(), Sense::hover());

                let page_count_horizontal =
                    calculate_page_count(self.desired_width, self.page_size.size().x);
                let page_count_vertical =
                    calculate_page_count(self.desired_height, self.page_size.size().y);
                let margin_frac = 0.05;
                let mut display_page_height = draw_area.height()
                    / (page_count_vertical as f32 * (1.0 + margin_frac) - margin_frac);
                let mut display_page_width = draw_area.width()
                    / (page_count_horizontal as f32 * (1.0 + margin_frac) - margin_frac);
                if display_page_width
                    >= display_page_height * self.page_size.size().x / self.page_size.size().y
                {
                    display_page_width =
                        display_page_height * self.page_size.size().x / self.page_size.size().y;
                } else {
                    display_page_height =
                        display_page_width * self.page_size.size().y / self.page_size.size().x;
                }

                let offset_vertical = ((page_count_vertical as f32 * self.page_size.size().y
                    - self.desired_height)
                    / 2.0)
                    / self.page_size.size().y
                    * display_page_height;
                let offset_horizontal = ((page_count_horizontal as f32 * self.page_size.size().x
                    - self.desired_width)
                    / 2.0)
                    / self.page_size.size().x
                    * display_page_width;

                for y in 0..page_count_vertical {
                    for x in 0..page_count_horizontal {
                        let display_page_start = Pos2::new(
                            x as f32 * (display_page_width + display_page_width * margin_frac),
                            y as f32 * (display_page_height + display_page_height * margin_frac),
                        );
                        let display_page_end = Pos2::new(
                            x as f32 * (display_page_width + display_page_width * margin_frac)
                                + display_page_width,
                            y as f32 * (display_page_height + display_page_height * margin_frac)
                                + display_page_height,
                        );
                        painter.rect_filled(
                            Rect::from_min_max(display_page_start, display_page_end)
                                .translate(draw_area.min.to_vec2()),
                            2.0,
                            Color32::WHITE,
                        );
                        let mut image_start = display_page_start;
                        let mut image_end = display_page_end;

                        if y == 0 {
                            image_start.y += offset_vertical;
                        }
                        if x == 0 {
                            image_start.x += offset_horizontal;
                        }
                        if y == page_count_vertical - 1 {
                            image_end.y -= offset_vertical;
                        }
                        if x == page_count_horizontal - 1 {
                            image_end.x -= offset_horizontal;
                        }
                        let page_count =
                            Vec2::new(page_count_horizontal as f32, page_count_vertical as f32);
                        let page_size = Vec2::new(self.page_size.size().x, self.page_size.size().y);
                        let desired_size = Vec2::new(self.desired_width, self.desired_height);
                        let prev_uv = calculate_uv_offset(
                            Vec2::new(x as f32, y as f32),
                            page_count,
                            page_size,
                            desired_size,
                        );
                        let uv = calculate_uv_offset(
                            Vec2::new(x as f32 + 1.0, y as f32 + 1.0),
                            page_count,
                            page_size,
                            desired_size,
                        );
                        if self.texture_id.is_some() {
                            painter.image(
                                self.texture_id.unwrap(),
                                Rect::from_min_max(image_start, image_end)
                                    .translate(draw_area.min.to_vec2()),
                                Rect::from_min_max(prev_uv.to_pos2(), uv.to_pos2()),
                                Color32::WHITE,
                            );
                        }
                    }
                }
            });
        });
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn execute<F: Future<Output = ()> + Send + 'static>(f: F) {
    // this is stupid... use any executor of your choice instead
    std::thread::spawn(move || futures::executor::block_on(f));
}

#[cfg(target_arch = "wasm32")]
fn execute<F: Future<Output = ()> + 'static>(f: F) {
    wasm_bindgen_futures::spawn_local(f);
}

#[cfg(target_arch = "wasm32")]
fn open_file_picker() {
    let window = web_sys::window().expect("Window not found");
    let document = window.document().expect("Document not found");

    let input_el = document.create_element("input").unwrap();
    let input: web_sys::HtmlInputElement =
        eframe::wasm_bindgen::JsCast::dyn_into(input_el).unwrap();

    input.set_id("rfd-input");
    input.set_type("file");
    input.set_accept(&[".jpeg", ".png"].join(","));
    input.click();

    let promise = web_sys::js_sys::Promise::new(&mut move |res, _rej| {
        let resolve_promise = eframe::wasm_bindgen::prelude::Closure::wrap(Box::new(move || {
            res.call0(&eframe::wasm_bindgen::JsValue::undefined())
                .unwrap();
        })
            as Box<dyn FnMut()>);
        resolve_promise.forget();
    });

    let future = wasm_bindgen_futures::JsFuture::from(promise);
    execute(async move {
        future.await.unwrap();
    });
}

fn calculate_page_count(desired: f32, print: f32) -> i32 {
    (desired / print).ceil() as i32
}

fn calculate_image_scale(desired: f32, print: f32, pdf_page: f32) -> f32 {
    (desired / print) * pdf_page
}

fn calculate_uv_offset(page: Vec2, page_count: Vec2, page_size: Vec2, desired_size: Vec2) -> Vec2 {
    let page_offset = (page_count * page_size - desired_size) / 2.0;
    let uv = (page_size * page - page_offset) / desired_size;
    uv.clamp(Vec2::ZERO, Vec2::new(1.0, 1.0))
}

fn generate_pdf(
    desired_width: f32,
    desired_height: f32,
    page_size: Page,
    image_data: &[u8],
) -> Vec<u8> {
    let page_width = page_size.size().x;
    let page_height = page_size.size().y;
    let dpi = 72.0;
    let pdf_point_page_width = page_width * dpi; //595;
    let pdf_point_page_height = page_height * dpi; //842;

    let page_count_horizontal = calculate_page_count(desired_width, page_width);
    let page_count_vertical = calculate_page_count(desired_height, page_height);

    let desired_image_width =
        calculate_image_scale(desired_width, page_width, pdf_point_page_width);
    let desired_image_height =
        calculate_image_scale(desired_height, page_height, pdf_point_page_height);

    let mut doc = krilla::Document::new();

    for y in 0..page_count_vertical {
        let y_offset =
            (((page_count_vertical as f32 * pdf_point_page_height) - desired_image_height) / 2.0)
                - (y as f32 * pdf_point_page_height);
        for x in 0..page_count_horizontal {
            let x_offset = (((page_count_horizontal as f32 * pdf_point_page_width)
                - desired_image_width)
                / 2.0)
                - (x as f32 * pdf_point_page_width);
            let mut page = doc.start_page();
            let mut surface = page.surface();
            surface.push_transform(&krilla::geom::Transform::from_translate(x_offset, y_offset));
            surface.draw_image(
                krilla::image::Image::from_png(&image_data).unwrap(),
                krilla::geom::Size::from_wh(desired_image_width, desired_image_height).unwrap(),
            );
            surface.pop();
            surface.finish();
            page.finish();
        }
    }
    doc.finish().unwrap()
}
